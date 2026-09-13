//! Instance installation: project the embedded payload into a target.
//!
//! The chezmoi vocabulary applies: the embedded payload and profile are the
//! source state, the computed projection is the target state, the repository
//! on disk is the destination state, and the manifest — written last — is
//! the persistent entry state. The whole target state is computed before a
//! byte lands; a non-empty target previews by default; every destination is
//! guarded; and any failure mid-apply rolls the target back. What the
//! payload contains is `embedded`'s and the profiles' business.

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::manifest::{
    CANON_SOURCE, MANIFEST_PATH, Manifest, PlanZone, SCHEMA_VERSION, validate_docs_scratch_path,
    validate_plan_zone_path,
};
use crate::domain::ownership::{AdoptedEntry, IntegrationBlock, ManagedEntry, Sha256};
use crate::domain::paths::{AGENTS_DIGEST_PATH, HOOKS_CONFIG_PATH};
use crate::domain::profile::{ProfileId, resolve_destination};
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::release::ReleaseBundle;
use crate::services::hooks_render::{RenderOptions, render_block};

/// What an installation was asked to do.
#[derive(Debug, Clone)]
pub struct InitOptions {
    /// The absolute target repository.
    pub target: Utf8PathBuf,
    /// The profile to project.
    pub profile: ProfileId,
    /// Write even into a non-empty target with no instance.
    pub apply: bool,
    /// Preview only, regardless of the target's state.
    pub dry_run: bool,
    /// The plan zone to record; `None` keeps whatever is recorded.
    pub plan_zone: Option<PlanZone>,
    /// The docs scratch to record. `None` keeps whatever is recorded, and
    /// `Some(None)` clears it.
    pub docs_scratch: Option<Option<Utf8PathBuf>>,
    /// Paths to record under `reserved:` in the instance's declaration. An
    /// empty list keeps whatever is recorded.
    pub reserve: Vec<String>,
    /// The writing-style selection to record in the declaration. `None`
    /// keeps whatever is recorded.
    pub writing_style: Option<crate::domain::instance_config::WritingStyle>,
}

/// What an installation did.
#[derive(Debug)]
pub struct InitOutcome {
    /// Every line to print: the proposed destinations, then any notices.
    pub lines: Vec<String>,
    /// Whether files were written.
    pub applied: bool,
    /// Every destination the landing took back, relative to the target.
    pub removed: Vec<String>,
}

fn canonical_target(target: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    if !target.is_absolute() {
        return Err(AppError::Usage("target must be absolute".to_string()));
    }
    if !target.is_dir() {
        return Err(AppError::Usage(format!("unresolved target: {target}")));
    }
    let canonical = std::fs::canonicalize(target)?;
    let canonical = Utf8PathBuf::from_path_buf(canonical)
        .map_err(|p| AppError::Usage(format!("target is not UTF-8: {}", p.display())))?;
    if canonical.as_str().chars().all(|c| c == '/') {
        return Err(AppError::Usage("refusing root target".to_string()));
    }
    let mut ancestor = Some(canonical.as_path());
    while let Some(dir) = ancestor {
        if let Ok(cargo) = std::fs::read_to_string(dir.join("Cargo.toml"))
            && cargo.contains("name = \"spec-driven-docs\"")
        {
            return Err(AppError::Usage(
                "target is inside the canon checkout".to_string(),
            ));
        }
        ancestor = dir.parent();
    }
    Ok(canonical)
}

fn target_has_content(target: &Utf8Path) -> Result<bool, AppError> {
    for entry in target.read_dir_utf8()? {
        let entry = entry?;
        if entry.file_name() != ".git" {
            return Ok(true);
        }
    }
    Ok(false)
}

/// One field of whatever manifest the target already carries.
///
/// Read as free JSON rather than through [`Manifest::parse`]: a reinstall
/// over a record of another schema version must still carry the operator's
/// declared values forward, and a typed parse would refuse to read it.
pub(crate) fn recorded_field(target: &Utf8Path, key: &str) -> Option<serde_json::Value> {
    std::fs::read_to_string(target.join(MANIFEST_PATH))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.get(key).cloned())
        .filter(|value| !value.is_null())
}

fn installed_at(target: &Utf8Path) -> String {
    recorded_field(target, "installed_at")
        .and_then(|value| value.as_str().map(String::from))
        .unwrap_or_else(|| {
            jiff::Timestamp::now()
                .strftime("%Y-%m-%dT%H:%M:%SZ")
                .to_string()
        })
}

/// The plan zone to record: the flag, else the recorded value, else none.
///
/// An omitted flag never clears a declared value. `sdd upgrade` reinstalls
/// with no flag at all, so "absent means the default" would erase the
/// operator's declaration on every upgrade.
///
/// A recorded value this binary cannot decode refuses rather than defaults,
/// for the same reason: writing the default over it would erase a
/// declaration silently, which is the failure the preservation exists to
/// prevent.
///
/// # Errors
///
/// [`AppError::ManifestInvalid`] when a value is recorded in a shape this
/// binary does not understand.
pub(crate) fn resolved_plan_zone(
    target: &Utf8Path,
    flag: Option<&PlanZone>,
) -> Result<PlanZone, AppError> {
    if let Some(zone) = flag {
        return Ok(zone.clone());
    }
    let Some(recorded) = recorded_field(target, "plan_zone") else {
        return Ok(PlanZone::default());
    };
    let zone: PlanZone = serde_json::from_value(recorded).map_err(|source| {
        AppError::ManifestInvalid(format!(
            "the recorded plan_zone is in a shape this sdd does not read ({source}); \
             upgrade sdd, or re-declare it with --plan-zone"
        ))
    })?;
    // The same invariants the argument enforces. Carried forward unchecked,
    // a hand-edited path fails the post-write verification instead, which
    // rolls the whole target back and names no repair.
    if let Some(path) = zone.path()
        && let Err(error) = validate_plan_zone_path(path)
    {
        return Err(AppError::ManifestInvalid(format!(
            "the recorded plan_zone is not usable ({error}); re-declare it with --plan-zone"
        )));
    }
    Ok(zone)
}

/// The docs scratch to record: the flag, else the recorded value, else none.
///
/// The flag is two-level on purpose: absent keeps what is recorded, and
/// `--docs-scratch none` clears it.
///
/// # Errors
///
/// [`AppError::ManifestInvalid`] when the recorded value is not a string.
pub(crate) fn resolved_docs_scratch(
    target: &Utf8Path,
    flag: Option<&Option<Utf8PathBuf>>,
) -> Result<Option<Utf8PathBuf>, AppError> {
    if let Some(declared) = flag {
        return Ok(declared.clone());
    }
    let Some(recorded) = recorded_field(target, "docs_scratch") else {
        return Ok(None);
    };
    let path = recorded
        .as_str()
        .filter(|path| !path.is_empty())
        .map(Utf8PathBuf::from)
        .ok_or_else(|| {
            AppError::ManifestInvalid(format!(
                "the recorded docs_scratch is not a path ({recorded}); \
                 re-declare it with --docs-scratch"
            ))
        })?;
    if let Err(error) = validate_docs_scratch_path(&path) {
        return Err(AppError::ManifestInvalid(format!(
            "the recorded docs_scratch is not usable ({error}); \
             re-declare it with --docs-scratch"
        )));
    }
    Ok(Some(path))
}

/// Every byte one landing would put in a target, and what it would say.
///
/// The planner takes this rather than deriving it a second time: what a
/// release lands into a target is one computation, and two of them would
/// be two places for one rule to drift.
#[derive(Debug, Clone)]
pub struct TargetState {
    /// Each destination and the bytes that would go there.
    pub files: Vec<(Utf8PathBuf, Vec<u8>)>,
    /// What the operator would be told.
    pub lines: Vec<String>,
}

#[allow(
    clippy::too_many_lines,
    reason = "computing the target state is one ordered pass the installer replays"
)]
/// What one landing would put in a target.
///
/// # Errors
///
/// [`AppError::Refused`] when the release declares no such profile or a
/// marked region cannot be read, and I/O errors reading the target.
pub fn compute_target_state(
    target: &Utf8Path,
    options: &InitOptions,
    bundle: &dyn ReleaseBundle,
) -> Result<TargetState, AppError> {
    let profile = options.profile;
    // The release the bundle is, not the release the engine is. A plan
    // toward an older version lands that version's bytes, so recording
    // this binary's version would leave the target claiming a release it
    // does not hold, and every later classification would read the lie.
    let landed: CanonVersion = bundle
        .manifest()?
        .version
        .to_string()
        .parse()
        .map_err(|_| AppError::Refused("the release is not a version triple".to_string()))?;
    let released = bundle.declaration()?;
    let declaration = released.profile(profile).ok_or_else(|| {
        AppError::Refused(format!(
            "the release declares no {profile} profile, so it cannot land one"
        ))
    })?;
    let mut files: Vec<(Utf8PathBuf, Vec<u8>)> = Vec::new();
    let mut lines = Vec::new();
    let mut managed_entries = Vec::new();
    let mut adopted_entries = Vec::new();

    for projection in declaration.managed {
        let bytes = bundle.artifact(&projection.source)?;
        let destination = Utf8PathBuf::from(&projection.destination);
        managed_entries.push(ManagedEntry {
            source: projection.source.clone().into(),
            destination: destination.clone(),
            sha256: Sha256::of(&bytes),
        });
        lines.push(destination.to_string());
        files.push((destination, bytes));
    }

    // What the target already records as adopted. A destination that holds
    // project content and is recorded nowhere is preserved and noted: the
    // seed does not land, and the project should know the specification it
    // would have received.
    let recorded_adopted: Vec<String> = recorded_field(target, "adopted_files")
        .and_then(|value| {
            value.as_array().map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| entry.get("destination")?.as_str().map(String::from))
                    .collect()
            })
        })
        .unwrap_or_default();
    for projection in declaration.adopted {
        let seed = bundle.artifact(&projection.source)?;
        let destination = resolve_destination(&projection.destination, declaration.docs_root);
        let existing = target.join(&destination);
        let mut bytes = if existing.is_file() {
            let held = std::fs::read(&existing)?;
            if held != seed && !recorded_adopted.iter().any(|d| d == destination.as_str()) {
                lines.push(format!(
                    "note: {destination} already exists and is kept; the seed was not written, so read it with 'sdd spec' and reconcile by hand"
                ));
            }
            held
        } else {
            seed.clone()
        };
        // `--reserve` and `--writing-style` record into the declaration,
        // keeping its comments and whatever the project already wrote there.
        if destination == crate::domain::instance_config::CONFIG_PATH
            && let Ok(text) = std::str::from_utf8(&bytes)
        {
            let mut text = text.to_string();
            if !options.reserve.is_empty() {
                text = crate::domain::instance_config::with_reserved(&text, &options.reserve);
            }
            if let Some(selection) = &options.writing_style {
                text = crate::domain::instance_config::with_writing_style(&text, selection);
            }
            bytes = text.into_bytes();
        }
        adopted_entries.push(AdoptedEntry {
            source: projection.source.clone().into(),
            destination: destination.clone(),
            sha256: Sha256::of(&bytes),
            baseline_sha256: Sha256::of(&seed),
        });
        lines.push(destination.to_string());
        files.push((destination, bytes));
    }

    let config_path = target.join(HOOKS_CONFIG_PATH);
    let host = if config_path.is_file() {
        std::fs::read_to_string(&config_path)?
    } else {
        "repos:\n".to_string()
    };
    let (base, _) = crate::domain::marker::split_block(&host)?;
    let indent = crate::domain::marker::splice_indent(&base)?;
    // Render from the declaration this install is writing, not from the one
    // on disk. With `--reserve` they differ, and a block rendered from the
    // old one would disagree with the file the same install lands.
    let declared = files
        .iter()
        .find(|(destination, _)| destination == crate::domain::instance_config::CONFIG_PATH)
        .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
        .map(crate::domain::instance_config::InstanceConfig::parse)
        .transpose()
        .map_err(|error| anyhow::anyhow!("{error}"))?
        .unwrap_or_default();
    let writing_style = declared.writing_style.clone();
    let block = render_block(&RenderOptions {
        docs_root: declaration.docs_root.to_string(),
        indent,
        declaration: declared,
        ..RenderOptions::default()
    });
    let spliced = crate::domain::marker::splice(&base, &block)?;
    let marker_hash = crate::domain::marker::block_hash(&spliced)
        .ok_or_else(|| anyhow::anyhow!("the rendered block lost its markers"))?;
    lines.push(HOOKS_CONFIG_PATH.to_string());
    files.push((Utf8PathBuf::from(HOOKS_CONFIG_PATH), spliced.into_bytes()));

    let mut integration_blocks = vec![IntegrationBlock {
        path: HOOKS_CONFIG_PATH.into(),
        marker_hash,
    }];

    // The root AGENTS.md documentation block routes authors to the context they
    // load before editing. A symlinked host is refused before it is read, so a
    // link cannot redirect the read outside the target.
    let agents_relative = Utf8Path::new(AGENTS_DIGEST_PATH);
    if target.join(agents_relative).is_symlink() {
        return Err(AppError::Refused(
            "AGENTS.md is a symlink; refusing to write the documentation block through it"
                .to_string(),
        ));
    }
    let agents_host = if target.join(agents_relative).is_file() {
        std::fs::read_to_string(target.join(agents_relative))?
    } else {
        String::new()
    };
    let agents_block = crate::services::agents_render::render_block(
        &declaration.docs_root.to_string(),
        &writing_style,
    );
    let agents = crate::domain::marker::place_agents_block(&agents_host, &agents_block)?;
    let agents_hash = crate::domain::marker::block_hash_with(
        &agents,
        crate::domain::marker::AGENTS_BEGIN,
        crate::domain::marker::AGENTS_END,
    )
    .ok_or_else(|| anyhow::anyhow!("the rendered AGENTS.md block lost its markers"))?;
    // An old unmarked documentation section is preserved, never deleted; the
    // note tells the operator to remove the duplicate by hand.
    if agents_host.contains("## Documentation")
        && crate::domain::marker::block_region_with(
            &agents_host,
            crate::domain::marker::AGENTS_BEGIN,
            crate::domain::marker::AGENTS_END,
        )
        .is_none()
    {
        lines.push(
            "note: AGENTS.md carries an unmarked '## Documentation' section; the managed block was appended and the old section left in place — remove it by hand".to_string(),
        );
    }
    lines.push(AGENTS_DIGEST_PATH.to_string());
    files.push((agents_relative.to_path_buf(), agents.into_bytes()));
    integration_blocks.push(IntegrationBlock {
        path: AGENTS_DIGEST_PATH.into(),
        marker_hash: agents_hash,
    });

    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        canon_version: landed,
        canon_source: CANON_SOURCE.to_string(),
        profile,
        docs_root: declaration.docs_root,
        installed_at: installed_at(target),
        plan_zone: resolved_plan_zone(target, options.plan_zone.as_ref())?,
        docs_scratch: resolved_docs_scratch(target, options.docs_scratch.as_ref())?,
        managed_files: managed_entries,
        adopted_files: adopted_entries,
        integration_blocks,
    };
    lines.push(MANIFEST_PATH.to_string());
    files.push((
        Utf8PathBuf::from(MANIFEST_PATH),
        manifest.to_json().into_bytes(),
    ));

    Ok(TargetState { files, lines })
}

/// Install or reinstall an instance.
///
/// # Errors
///
/// [`AppError::Usage`] for a target the arguments cannot mean,
/// [`AppError::Marker`] for a configuration whose markers cannot be trusted,
/// and [`AppError::Refused`] when the apply could not complete — the target
/// is restored before that returns.
pub fn init(
    options: &InitOptions,
    bundle: &dyn ReleaseBundle,
    intent: crate::plan::classify::Intent,
) -> Result<InitOutcome, AppError> {
    init_with(
        &crate::plan::decision::Selections::new(),
        options,
        bundle,
        intent,
    )
}

/// Install or reinstall an instance, carrying answers the caller collected.
///
/// # Errors
///
/// As [`init`], plus whatever the plan refuses when a decision it raises
/// is unanswered.
pub fn init_with(
    answered: &crate::plan::decision::Selections,
    options: &InitOptions,
    bundle: &dyn ReleaseBundle,
    intent: crate::plan::classify::Intent,
) -> Result<InitOutcome, AppError> {
    let target = canonical_target(&options.target)?;
    // The target is known-good before it is classified, so an argument
    // this verb cannot mean is a usage answer rather than a walk of
    // whatever the argument happened to name.
    crate::commands::front::serves(intent, &target)?;
    let forced_dry = !options.apply
        && !options.dry_run
        && target_has_content(&target)?
        && !target.join(MANIFEST_PATH).is_file();
    let dry = options.dry_run || forced_dry;

    let state = compute_target_state(&target, options, bundle)?;
    let mut lines = state.lines;

    if dry {
        if forced_dry {
            lines.push(
                "DRY RUN: the target is a non-empty repository with no instance; re-run with --apply to write these files"
                    .to_string(),
            );
        }
        lines.push("DRY RUN: no files written".to_string());
        return Ok(InitOutcome {
            lines,
            applied: false,
            removed: Vec::new(),
        });
    }

    // Every write into a target comes from an operation in one plan, so
    // this verb reaches the engine rather than writing what it computed.
    // The state above is what the planner derives its operations from, so
    // the landing is the same landing; what it gains is the plan's own id,
    // the journal that can take it back, and a recorded result.
    let result = crate::plan::session::land(&crate::plan::session::Landing {
        target: &target,
        selector: "embedded",
        offline: true,
        selections: with_profile(answered, options.profile),
        reserve: options.reserve.clone(),
        declared: Some(options.clone()),
    })?;
    for refused in result
        .postconditions
        .iter()
        .filter(|postcondition| !postcondition.held)
    {
        lines.push(format!(
            "FAIL {} did not hold: {}",
            refused.id,
            refused.detail.clone().unwrap_or_default()
        ));
    }
    Ok(InitOutcome {
        lines,
        applied: true,
        removed: result
            .operations
            .iter()
            .filter(|operation| operation.kind == "remove-owned-file")
            .map(|operation| operation.path.clone())
            .collect(),
    })
}

/// The one decision a front's flags still answer by name.
///
/// The rest travel as the options themselves. A flag and a decision are
/// the same answer under two names, and rendering a recorded value back
/// into its flag spelling only to parse it again is a round trip that can
/// lose what it carries.
fn with_profile(
    answered: &crate::plan::decision::Selections,
    profile: ProfileId,
) -> crate::plan::decision::Selections {
    let mut selections = answered.clone();
    selections.insert(
        crate::plan::decision::id::PROFILE.to_string(),
        profile.to_string(),
    );
    selections
}
