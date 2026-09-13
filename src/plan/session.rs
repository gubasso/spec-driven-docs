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

/// One release a caller already holds, borrowed.
///
/// A front resolves its bundle once and hands the same object over, so
/// the release its preview described is the release that lands. An owned
/// [`Release`] lends one of these; a caller holding only a bundle builds
/// one directly.
#[derive(Clone)]
pub(crate) struct ReleaseRef<'a> {
    /// The bytes, through the seam.
    pub bundle: &'a dyn ReleaseBundle,
    /// The exact release this is.
    pub version: String,
    /// Where its facts came from.
    pub provenance: &'static str,
    /// The registry checksum, where one was read.
    pub checksum: Option<Sha256>,
    /// Whether the registry marks it withdrawn.
    pub yanked: bool,
}

impl<'a> ReleaseRef<'a> {
    /// Describe a bundle a caller already holds.
    ///
    /// # Errors
    ///
    /// Whatever the bundle's manifest refuses.
    pub(crate) fn of(bundle: &'a dyn ReleaseBundle) -> Result<Self, AppError> {
        let manifest = bundle.manifest()?;
        Ok(Self {
            bundle,
            version: manifest.version.to_string(),
            provenance: match manifest.provenance {
                Provenance::Native => "native",
                Provenance::LegacyAdapted => "legacy-adapted",
            },
            checksum: None,
            yanked: false,
        })
    }
}

impl Release {
    /// Lend this release to the planner.
    pub(crate) fn borrow(&self) -> ReleaseRef<'_> {
        ReleaseRef {
            bundle: self.bundle.as_ref(),
            version: self.version.clone(),
            provenance: if self.provenance == "native" {
                "native"
            } else {
                "legacy-adapted"
            },
            checksum: self.checksum.clone(),
            yanked: self.yanked,
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
    payload_schema: u32,
    recorded: Option<crate::domain::version::CanonVersion>,
    destination: crate::domain::version::CanonVersion,
    selections: &decision::Selections,
) -> Result<Option<guidance::Briefing>, AppError> {
    // The same rule the compatibility declaration takes. A release that
    // owes a ledger and carries none would otherwise plan ready with no
    // breaking step raised, which is the one outcome the ledger exists to
    // prevent. Only a release from before the declaration may be silent.
    let bytes = match bundle.artifact(guidance::INDEX_PATH) {
        Ok(bytes) => bytes,
        Err(_) if payload_schema == 0 => return Ok(None),
        Err(source) => {
            return Err(AppError::Refused(format!(
                "this release declares payload schema {payload_schema} and its {} could not be read: {source}",
                guidance::INDEX_PATH
            )));
        }
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
    release: &ReleaseRef<'_>,
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

/// What the interval between the record and the destination is.
///
/// # Errors
///
/// [`AppError::Refused`] when the release is not a version triple.
type Interval = (
    Option<crate::domain::version::CanonVersion>,
    crate::domain::version::CanonVersion,
    Option<compatibility::Interval>,
);

fn interval_of(
    observation: &crate::plan::observe::Observation,
    release: &ReleaseRef<'_>,
    compatibility: Option<&compatibility::Compatibility>,
) -> Result<Interval, AppError> {
    let recorded = observation
        .installation
        .as_ref()
        .map(|installed| installed.canon_version);
    let destination: crate::domain::version::CanonVersion = release
        .version
        .parse()
        .map_err(|_| AppError::Refused(format!("{} is not a released triple", release.version)))?;
    let interval = compatibility.map(|_| compatibility::Interval {
        engine: crate::domain::version::CanonVersion::current(),
        recorded,
        destination,
    });
    Ok((recorded, destination, interval))
}

/// Everything the landing derivation reads.
struct Derivation<'a> {
    target: &'a Utf8Path,
    profile: ProfileId,
    declaration: &'a crate::domain::projection::Declaration,
    observation: &'a crate::plan::observe::Observation,
    selections: &'a decision::Selections,
    reserve: &'a [String],
    declared: Option<&'a InitOptions>,
    budget: &'a [crate::domain::debt::Measurement],
}

/// Every write one landing implies, and the bytes each one lands.
///
/// The installer owns what a release lands, so this takes its answer and
/// says what kind of write each destination is, then adds the one write
/// an operator has to ask for.
///
/// # Errors
///
/// Whatever the installer or the derivation refuses.
fn derive_landing(
    from: &Derivation<'_>,
    release: &ReleaseRef<'_>,
) -> Result<crate::plan::derive::Derived, AppError> {
    let options = match from.declared {
        Some(held) => held.clone(),
        None => landing_options(from.target, from.profile, from.selections, from.reserve)?,
    };
    let state = compute_target_state(from.target, &options, release.bundle)?;
    let mut derived = crate::plan::derive::operations_for(
        from.target,
        &state.files,
        from.declaration,
        from.profile,
        &recorded_managed(from.observation),
    )?;
    // The inherited violations become a ceiling only where the operator
    // asked for that. A version moving never records one.
    if from
        .selections
        .get(decision::id::DEBT_BASELINE)
        .map(String::as_str)
        == Some("record")
        && let Some((operation, bytes)) =
            crate::plan::derive::debt_operation(from.target, from.budget)?
    {
        derived.1.insert(Sha256::of(&bytes), bytes);
        derived.0.push(operation);
    }
    Ok(derived)
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
    release: &ReleaseRef<'_>,
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
    // A profile change is a named migration, not something a landing verb
    // does because a flag said so. The record decides for an installed
    // target, and a caller asking for another one is refused rather than
    // quietly landing the other profile's files under this one's record.
    if let (Some(held), Some(asked)) = (
        observation
            .installation
            .as_ref()
            .map(|installed| installed.profile),
        declared.map(|options| options.profile),
    ) && held != asked
    {
        return Err(AppError::Refused(format!(
            "this instance records the {held} profile and the request names {asked}; a profile change is its own migration, not a landing"
        )));
    }

    // The same measurements `sdd debt` records, taken once and read twice.
    // The gates resolve the documentation root the same way here as they
    // do at commit time, so a finding and a later gate failure agree.
    let budget = crate::services::budget::measure_all(target).unwrap_or_default();

    let landing = match profile {
        Some(profile) if answered && observation.invalid.is_none() => Some(derive_landing(
            &Derivation {
                target,
                profile,
                declaration: &declaration,
                observation: &observation,
                selections,
                reserve,
                declared,
                budget: &budget,
            },
            release,
        )?),
        _ => None,
    };
    let (proposed, blobs) = landing.map_or_else(
        || (None, BTreeMap::new()),
        |(operations, bytes)| (Some(operations), bytes),
    );

    let compatibility = read_compatibility(release, declaration.payload_schema)?;
    let (recorded, destination, interval) =
        interval_of(&observation, release, compatibility.as_ref())?;
    let briefing = read_briefing(
        release.bundle,
        declaration.payload_schema,
        recorded,
        destination,
        selections,
    )?;

    let computed = compute(&Inputs {
        observation: &observation,
        declaration: &declaration,
        candidate: &candidate,
        baseline: baseline.as_ref(),
        selector: to.to_string(),
        release: release.version.clone(),
        release_sha256: manifest.payload_sha256,
        provenance: release.provenance.to_string(),
        registry_checksum: release.checksum.clone(),
        yanked: release.yanked,
        compatibility: compatibility.as_ref(),
        interval: interval.as_ref(),
        briefing: briefing.as_ref(),
        proposed: proposed.as_deref(),
        selections,
        budget: &budget,
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
    /// The release that lands, resolved once by the caller.
    ///
    /// The object, not a selector to resolve again. A caller that reads a
    /// fixture or an older release must land that release's bytes, and a
    /// second resolution here would land whatever this binary carries
    /// while the caller's preview described something else.
    pub release: ReleaseRef<'a>,
    /// Whether the network is forbidden.
    pub offline: bool,
    /// Every decision the operator answered, validated against the plan.
    ///
    /// Only these. A value the front carries internally, such as the
    /// profile a record already names, is not an answer anybody typed and
    /// is not a decision the plan offers, so validating it would refuse a
    /// correct request.
    pub selections: decision::Selections,
    /// Answers the front supplies for itself, which nobody typed.
    pub carried: decision::Selections,
    /// Paths no delivered gate judges, from the caller's flags.
    pub reserve: Vec<String>,
    /// The declarations a front already holds, where it holds them.
    ///
    /// A front carries recorded values that have no flag spelling to
    /// round-trip through, so it hands them over as they are rather than
    /// rendering them into answers the planner would parse back.
    pub declared: Option<InitOptions>,
}

impl Landing<'_> {
    /// What the plan records as the caller's request.
    ///
    /// The embedded release has no version to look up, so it keeps the
    /// word rather than a triple: an apply that read the triple back
    /// would go to the registry for a release this binary already holds.
    fn selector(&self) -> String {
        if self.release.provenance == "native"
            && self.release.version == crate::domain::version::CanonVersion::current().to_string()
        {
            "embedded".to_string()
        } else {
            self.release.version.clone()
        }
    }
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

/// Compute the plan one landing would run, and write nothing.
///
/// What a preview owes its reader is the plan, not a list of paths. A
/// preview that could not show a blocked precondition or a decision the
/// interval raises would tell an operator the run is ready when it is not.
///
/// # Errors
///
/// [`AppError::Busy`] when a writer holds the target, and whatever the
/// planner refuses.
pub(crate) fn preview(request: &Landing<'_>) -> Result<Plan, AppError> {
    let _lock = Lock::shared(&target_lock(request.target)?, "landing preview")?;
    let mut answers = request.carried.clone();
    answers.extend(request.selections.clone());
    let (plan, _) = compute_plan(
        request.target,
        &request.selector(),
        request.offline,
        &answers,
        &request.reserve,
        request.declared.as_ref(),
        &request.release,
    )?;
    decision::validate(&plan.decisions, &request.selections)
        .map_err(|error| AppError::Usage(error.to_string()))?;
    Ok(plan)
}

/// What a preview says about a plan beyond the destinations it names.
///
/// One line per thing that would stop the run or ask a question. A ready
/// plan adds nothing, because the destination list already said it all.
#[must_use]
pub(crate) fn preview_lines(plan: &Plan) -> Vec<String> {
    let mut lines = Vec::new();
    for precondition in &plan.preconditions {
        if precondition.requirement == crate::plan::readiness::Requirement::Required
            && !precondition.evaluation.is_satisfied()
        {
            lines.push(format!(
                "BLOCKED {}: {}",
                precondition.id, precondition.statement
            ));
        }
    }
    for decision in &plan.decisions {
        if decision.selected.is_none() {
            lines.push(format!("DECISION {}: {}", decision.id, decision.question));
        }
    }
    lines
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
    let release = &request.release;
    let mut answers = request.carried.clone();
    answers.extend(request.selections.clone());
    let (plan, blobs) = compute_plan(
        request.target,
        &request.selector(),
        request.offline,
        &answers,
        &request.reserve,
        request.declared.as_ref(),
        release,
    )?;
    // An answer the plan does not offer is not authorization. The front
    // verbs take `--set` too, so the check belongs here rather than in one
    // caller: a malformed answer that reached a precondition would turn
    // typing into consent.
    decision::validate(&plan.decisions, &request.selections)
        .map_err(|error| AppError::Usage(error.to_string()))?;

    let store = Store::new(&state_root()?);
    store.create()?;
    let _store_lock = plan_lock(&store, &plan.identity.plan_id)?;
    if !store.holds(&plan.identity.plan_id) {
        let mut blobs = blobs;
        for (digest, bytes) in blobs_for(&plan, release.bundle) {
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
        bundle: release.bundle,
        now: jiff::Timestamp::now().to_string(),
    })
}
