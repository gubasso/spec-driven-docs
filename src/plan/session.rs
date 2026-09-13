//! One planning session, from an observed target to a stored plan.
//!
//! Both fronts reach the engine through this module: `sdd reconcile` and
//! the compatibility verbs alike. It exists so there is one place that
//! turns a target and a set of answers into a plan, and one place that
//! stores and executes one. A second path here would be the drift the
//! plan model exists to remove.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::manifest::{PlanZone, parse_docs_scratch};
use crate::domain::ownership::Sha256;
use crate::domain::paths::UserEnv;
use crate::domain::profile::ProfileId;
use crate::error::AppError;
use crate::plan::apply::{Request, apply as execute};
use crate::plan::observe::observe;
use crate::plan::planner::{Inputs, plan as compute};
use crate::plan::store::{Result as ApplyResult, Store};
use crate::plan::{Plan, compatibility, decision, guidance};
use crate::release::crates_io::CratesIoResolver;
use crate::release::embedded::EmbeddedReleaseBundle;
use crate::release::{Provenance, ReleaseBundle, ReleaseResolver, Role, Selector};
use crate::services::installer::{InitOptions, compute_target_state};
use crate::transaction::lock::Lock;

/// How long a verb waits for the store before it refuses.
///
/// The store's critical section is a prune and a directory write, so a
/// second writer queues behind it rather than failing. A wait this long
/// running out means a holder died, which is worth reporting.
pub(crate) const STORE_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

/// What the caller asked for.
fn selector(value: &str) -> Result<Selector, AppError> {
    match value {
        "embedded" => Ok(Selector::Embedded),
        "latest" => Ok(Selector::Latest),
        version => version.parse().map(Selector::Exact).map_err(|_| {
            AppError::Usage(format!(
                "--to takes embedded, latest, or a semantic version; {version} is none of those"
            ))
        }),
    }
}

/// Where this tool keeps state that outlives a command.
pub(crate) fn state_root() -> Result<Utf8PathBuf, AppError> {
    Ok(UserEnv::from_process()
        .state_root()
        .ok_or_else(|| AppError::Usage("no state root resolves".to_string()))?
        .path)
}

/// The lock one target takes, keyed by where it is.
///
/// A digest rather than the path itself: a lock file named after a
/// repository would put a person's directory layout in the state root, and
/// two targets whose paths differ only in case would collide.
pub(crate) fn target_lock(target: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    let key = Sha256::of(target.as_str().as_bytes());
    Ok(state_root()?.join("locks").join(format!("{key}.lock")))
}

/// One release, read through the seam.
pub(crate) struct Release {
    pub(crate) bundle: Box<dyn ReleaseBundle>,
    pub(crate) version: String,
    pub(crate) provenance: String,
    pub(crate) checksum: Option<Sha256>,
    pub(crate) yanked: bool,
}

pub(crate) fn read_release(to: &str, offline: bool) -> Result<Release, AppError> {
    match selector(to)? {
        Selector::Embedded => Ok(Release {
            bundle: Box::new(EmbeddedReleaseBundle::new()),
            version: crate::domain::version::CanonVersion::current().to_string(),
            provenance: "native".to_string(),
            checksum: None,
            yanked: false,
        }),
        chosen => {
            let cache = UserEnv::from_process()
                .user_paths()
                .ok_or_else(|| AppError::Usage("no cache root resolves".to_string()))?
                .bundle_cache
                .path;
            let resolved = CratesIoResolver::new(&cache)
                .offline(offline)
                .resolve(&chosen)?;
            let manifest = resolved.bundle.manifest()?;
            let provenance = match manifest.provenance {
                Provenance::Native => "native",
                Provenance::LegacyAdapted => "legacy-adapted",
            };
            Ok(Release {
                bundle: resolved.bundle,
                version: resolved.version.to_string(),
                provenance: provenance.to_string(),
                checksum: resolved.registry_checksum,
                yanked: resolved.yanked,
            })
        }
    }
}

/// What every release in the interval asks of this target.
///
/// The vocabulary a step filters against is the destination set a landed
/// instance has, so a step about something the target did not install is
/// excluded rather than shown.
fn read_briefing(
    bundle: &dyn ReleaseBundle,
    recorded: Option<crate::domain::version::CanonVersion>,
    destination: crate::domain::version::CanonVersion,
    selections: &decision::Selections,
) -> Result<Option<guidance::Briefing>, AppError> {
    let Ok(bytes) = bundle.artifact(guidance::INDEX_PATH) else {
        return Ok(None);
    };
    let index =
        guidance::Index::parse(&bytes).map_err(|error| AppError::Refused(error.to_string()))?;
    let bodies: Vec<String> = bundle
        .manifest()?
        .artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect();
    let mut files = Vec::new();
    for entry in index.interval(recorded, destination) {
        if entry.guidance == guidance::NONE {
            continue;
        }
        let path = format!("guidance/{}", entry.guidance);
        let bytes = bundle.artifact(&path)?;
        let held = guidance::Guidance::parse(&path, &bytes, &bodies)
            .map_err(|error| AppError::Refused(error.to_string()))?;
        files.push((entry.version, held));
    }
    // Every destination a landed instance has. An unlanded target has
    // none, so nothing filters through and the briefing is empty.
    let held: Vec<String> = if recorded.is_some() {
        guidance::DESTINATIONS
            .iter()
            .map(|destination| (*destination).to_string())
            .collect()
    } else {
        Vec::new()
    };
    Ok(Some(guidance::brief(
        &index, &files, recorded, &held, selections,
    )))
}

/// Read the declarations one landing would record, from the answers given.
fn landing_options(
    target: &Utf8Path,
    profile: ProfileId,
    selections: &decision::Selections,
    reserve: &[String],
) -> Result<InitOptions, AppError> {
    let plan_zone = selections
        .get(decision::id::PLAN_ZONE)
        .map(|answer| PlanZone::parse(answer.strip_prefix("project:").unwrap_or(answer)))
        .transpose()
        .map_err(|error| AppError::Usage(format!("--set plan-zone: {error}")))?;
    let docs_scratch = selections
        .get(decision::id::DOCS_SCRATCH)
        .map(|answer| {
            let bare = answer
                .strip_prefix("project:")
                .or_else(|| answer.strip_prefix("external:"))
                .unwrap_or(answer);
            parse_docs_scratch(bare)
        })
        .transpose()
        .map_err(|error| AppError::Usage(format!("--set docs-scratch: {error}")))?;
    let writing_style = selections
        .get(decision::id::WRITING_STYLE)
        .map(|answer| crate::domain::instance_config::WritingStyle::parse_flag(answer))
        .transpose()
        .map_err(|error| AppError::Usage(format!("--set writing-style: {error}")))?;
    Ok(InitOptions {
        target: target.to_owned(),
        profile,
        apply: false,
        dry_run: true,
        plan_zone,
        docs_scratch,
        reserve: reserve.to_vec(),
        writing_style,
    })
}

/// What the record holds now, which is what a removal can take back.
fn recorded_managed(
    observation: &crate::plan::observe::Observation,
) -> Vec<(String, crate::domain::ownership::Sha256)> {
    observation
        .installation
        .as_ref()
        .map(|installed| {
            installed
                .managed
                .iter()
                .map(|file| (file.path.as_str().to_string(), file.recorded.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// What the destination needs of this engine.
///
/// A schema-one bundle without the declaration is invalid: an absence
/// meaning "no requirement" cannot be told from an absence meaning
/// somebody forgot.
///
/// # Errors
///
/// [`AppError::Refused`] when the declaration does not parse, or when a
/// bundle that owes one carries none.
fn read_compatibility(
    release: &Release,
    payload_schema: u32,
) -> Result<Option<compatibility::Compatibility>, AppError> {
    match release.bundle.artifact(compatibility::DECLARATION_PATH) {
        Ok(bytes) => compatibility::Compatibility::parse(&bytes)
            .map(Some)
            .map_err(|error| AppError::Refused(error.to_string())),
        Err(_) if payload_schema == 0 => Ok(None),
        Err(_) => Err(AppError::Refused(format!(
            "release {} declares payload schema {payload_schema} and carries no {}",
            release.version,
            compatibility::DECLARATION_PATH
        ))),
    }
}

/// Compute one plan against one target.
///
/// # Errors
///
/// Whatever the observation, the bundle, or the declarations refuse.
pub(crate) fn compute_plan(
    target: &Utf8Path,
    to: &str,
    offline: bool,
    selections: &decision::Selections,
    reserve: &[String],
    declared: Option<&InitOptions>,
    release: &Release,
) -> Result<(Plan, BTreeMap<Sha256, Vec<u8>>), AppError> {
    let _ = offline;
    let observation = observe(target)?;
    let declaration = release.bundle.declaration()?;
    let manifest = release.bundle.manifest()?;
    let candidate: BTreeMap<String, Sha256> = manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.role == Role::Payload)
        .map(|artifact| (artifact.path.clone(), artifact.sha256.clone()))
        .collect();
    // The baseline comes from the recorded release. Where that is the
    // destination, the candidate is the baseline; where it is not, this
    // engine does not fetch it, and the plan says so rather than guessing.
    let recorded_is_destination = observation
        .installation
        .as_ref()
        .is_some_and(|installed| installed.canon_version.to_string() == release.version);
    let baseline = recorded_is_destination.then(|| candidate.clone());

    // The installer owns what a landing writes, so the plan takes its
    // answer once every declaration the landing records is settled.
    let profile = observation
        .installation
        .as_ref()
        .map(|installed| installed.profile)
        .or_else(
            || match selections.get(decision::id::PROFILE).map(String::as_str) {
                Some("codebase") => Some(ProfileId::Codebase),
                Some("knowledge-base") => Some(ProfileId::KnowledgeBase),
                _ => None,
            },
        );
    // A front that handed over its declarations has answered every
    // question the planner would otherwise ask for them.
    let answered = declared.is_some()
        || observation.installation.is_some()
        || [
            decision::id::PLAN_ZONE,
            decision::id::DOCS_SCRATCH,
            decision::id::WRITING_STYLE,
        ]
        .iter()
        .all(|id| selections.contains_key(*id));
    let landing = match profile {
        Some(profile) if answered && observation.invalid.is_none() => {
            let options = match declared {
                Some(held) => held.clone(),
                None => landing_options(target, profile, selections, reserve)?,
            };
            let state = compute_target_state(target, &options, release.bundle.as_ref())?;
            Some(crate::plan::derive::operations_for(
                target,
                &state.files,
                &declaration,
                profile,
                &recorded_managed(&observation),
            )?)
        }
        _ => None,
    };
    let (proposed, blobs) = landing.map_or_else(
        || (None, BTreeMap::new()),
        |(operations, bytes)| (Some(operations), bytes),
    );

    let compatibility = read_compatibility(release, declaration.payload_schema)?;
    let recorded = observation
        .installation
        .as_ref()
        .map(|installed| installed.canon_version);
    let destination: crate::domain::version::CanonVersion = release
        .version
        .parse()
        .map_err(|_| AppError::Refused(format!("{} is not a released triple", release.version)))?;
    let interval = compatibility.as_ref().map(|_| compatibility::Interval {
        engine: crate::domain::version::CanonVersion::current(),
        recorded,
        destination,
    });
    let briefing = read_briefing(release.bundle.as_ref(), recorded, destination, selections)?;

    let computed = compute(&Inputs {
        observation: &observation,
        declaration: &declaration,
        candidate: &candidate,
        baseline: baseline.as_ref(),
        selector: to.to_string(),
        release: release.version.clone(),
        release_sha256: manifest.payload_sha256,
        provenance: release.provenance.clone(),
        registry_checksum: release.checksum.clone(),
        yanked: release.yanked,
        compatibility: compatibility.as_ref(),
        interval: interval.as_ref(),
        briefing: briefing.as_ref(),
        proposed: proposed.as_deref(),
        selections,
        reserve,
        declarations_settled: declared.is_some(),
        now: jiff::Timestamp::now().to_string(),
    });
    Ok((computed, blobs))
}

/// Every byte one plan will write, by digest.
pub(crate) fn blobs_for(plan: &Plan, bundle: &dyn ReleaseBundle) -> BTreeMap<Sha256, Vec<u8>> {
    let mut blobs = BTreeMap::new();
    for operation in &plan.operations {
        let Some(after) = operation.after() else {
            continue;
        };
        if blobs.contains_key(after) {
            continue;
        }
        // A digest the bundle does not carry came from the target state,
        // which already handed its bytes over.
        if let Ok(bytes) = bundle.blob(after) {
            blobs.insert(after.clone(), bytes);
        }
    }
    blobs
}

/// What one landing asked for.
pub(crate) struct Landing<'a> {
    /// The repository, already canonical.
    pub target: &'a Utf8Path,
    /// The destination, as the caller named it.
    pub selector: &'a str,
    /// Whether the network is forbidden.
    pub offline: bool,
    /// Every decision the caller answered.
    pub selections: decision::Selections,
    /// Paths no delivered gate judges, from the caller's flags.
    pub reserve: Vec<String>,
    /// The declarations a front already holds, where it holds them.
    ///
    /// A front carries recorded values that have no flag spelling to
    /// round-trip through, so it hands them over as they are rather than
    /// rendering them into answers the planner would parse back.
    pub declared: Option<InitOptions>,
}

/// Take one plan's own lock, after an opportunistic prune.
///
/// The prune walks the whole store, so it takes the store lock and gives
/// up rather than waiting: it is housekeeping, and a landing that skipped
/// it loses nothing but disk.
///
/// # Errors
///
/// [`AppError::Busy`] when another writer holds this fingerprint.
fn plan_lock(store: &Store, fingerprint: &str) -> Result<Lock, AppError> {
    if let Ok(_walk) = Lock::exclusive(&store.lock_path(), "plan store prune") {
        let _ = store.prune(jiff::Timestamp::now(), Some(fingerprint));
    }
    Lock::exclusive_waiting(&store.plan_lock_path(fingerprint)?, "landing", STORE_WAIT)
}

/// Plan one landing and execute it.
///
/// The one path from a target to a write. Every front reaches the engine
/// here, so an operation is never derived twice and never applied outside
/// the journal that can take it back. A caller that wants a preview asks
/// for a plan and does not call this.
///
/// # Errors
///
/// [`AppError::Busy`] when another writer holds the target, and whatever
/// the planner, the store, or the executor refuses.
pub(crate) fn land(request: &Landing<'_>) -> Result<ApplyResult, AppError> {
    let _lock = Lock::exclusive(&target_lock(request.target)?, "landing")?;
    let release = read_release(request.selector, request.offline)?;
    let (plan, blobs) = compute_plan(
        request.target,
        request.selector,
        request.offline,
        &request.selections,
        &request.reserve,
        request.declared.as_ref(),
        &release,
    )?;

    let store = Store::new(&state_root()?);
    store.create()?;
    let _store_lock = plan_lock(&store, &plan.identity.plan_id)?;
    if !store.holds(&plan.identity.plan_id) {
        let mut blobs = blobs;
        for (digest, bytes) in blobs_for(&plan, release.bundle.as_ref()) {
            blobs.entry(digest).or_insert(bytes);
        }
        store.put(&plan, &blobs)?;
    }
    execute(&Request {
        store: &store,
        target: request.target,
        stored: &plan,
        // The plan was computed a moment ago under this same exclusive
        // lock, so nothing could move between the two. The apply still
        // compares the two fingerprints rather than assuming that.
        recomputed: &plan,
        bundle: release.bundle.as_ref(),
        now: jiff::Timestamp::now().to_string(),
    })
}
