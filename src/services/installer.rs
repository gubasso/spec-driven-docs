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

use crate::domain::manifest::{MANIFEST_PATH, validate_docs_scratch_path};
use crate::domain::paths::{AGENTS_DIGEST_PATH, HOOKS_CONFIG_PATH};
use crate::domain::profile::{ProfileId, resolve_destination};
use crate::domain::version::CanonVersion;
use crate::error::AppError;

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

/// The docs scratch to record: the flag, else the recorded value, else none.
///
/// An omitted flag never clears a declared value. `sdd upgrade` reinstalls
/// with no flag at all, so "absent means the default" would erase the
/// operator's declaration on every upgrade. The flag is two-level on
/// purpose: absent keeps what is recorded, and `--docs-scratch none` clears
/// it.
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

/// What this binary would put in a target, and what it would say.
///
/// Observation happens here and projection happens in [`crate::candidate`].
/// Everything this function reads from the target is a value it hands over,
/// so the bytes a stage renders and the bytes a landing writes come out of
/// one pure pass over the same evidence.
///
/// # Errors
///
/// [`AppError::Refused`] when this release declares no such profile, when a
/// marked region cannot be read, or when the root author-instructions file
/// is a link, and I/O errors reading the target.
pub fn compute_target_state(
    target: &Utf8Path,
    options: &InitOptions,
) -> Result<TargetState, AppError> {
    let candidate = candidate_for(target, options)?;
    let mut lines: Vec<String> = Vec::new();
    for destination in &candidate.destinations {
        lines.push(destination.path.to_string());
    }
    lines.extend(candidate.notes.iter().cloned());
    lines.push(MANIFEST_PATH.to_string());
    Ok(TargetState {
        files: candidate.files(),
        lines,
    })
}

/// The candidate this binary renders for one target.
///
/// # Errors
///
/// As [`compute_target_state`].
pub fn candidate_for(
    target: &Utf8Path,
    options: &InitOptions,
) -> Result<crate::candidate::Candidate, AppError> {
    crate::candidate::project(&gather(target, options)?)
}

/// The target a landing or a stage will read, canonical and known-good.
///
/// # Errors
///
/// [`AppError::Usage`] for a path no verb can mean: relative, absent, the
/// filesystem root, or inside the canon checkout.
pub fn resolved_target(target: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    canonical_target(target)
}

/// Read the target once, so the projection never has to.
fn gather(target: &Utf8Path, options: &InitOptions) -> Result<crate::candidate::Input, AppError> {
    let docs_root = crate::candidate::docs_root_of(options.profile)?;

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

    let mut existing = std::collections::BTreeMap::new();
    for projection in &crate::domain::profile::DECLARATION.adopted {
        let destination = resolve_destination(&projection.destination, docs_root);
        let path = target.join(&destination);
        if path.is_file() {
            existing.insert(destination, std::fs::read(&path)?);
        }
    }

    let hooks_path = target.join(HOOKS_CONFIG_PATH);
    let hooks_host = if hooks_path.is_file() {
        std::fs::read_to_string(&hooks_path)?
    } else {
        String::new()
    };

    // The root AGENTS.md documentation block routes authors to the context they
    // load before editing. A symlinked host is refused before it is read, so a
    // link cannot redirect the read outside the target.
    let agents_path = target.join(AGENTS_DIGEST_PATH);
    if agents_path.is_symlink() {
        return Err(AppError::Refused(
            "AGENTS.md is a symlink; refusing to write the documentation block through it"
                .to_string(),
        ));
    }
    let agents_host = if agents_path.is_file() {
        std::fs::read_to_string(&agents_path)?
    } else {
        String::new()
    };

    Ok(crate::candidate::Input {
        profile: options.profile,
        version: CanonVersion::current(),
        installed_at: installed_at(target),
        docs_scratch: resolved_docs_scratch(target, options.docs_scratch.as_ref())?,
        reserve: options.reserve.clone(),
        writing_style: options.writing_style.clone(),
        evidence: crate::candidate::Evidence {
            existing,
            recorded_adopted,
            hooks_host,
            agents_host,
        },
    })
}

/// Install or reinstall an instance.
///
/// # Errors
///
/// [`AppError::Usage`] for a target the arguments cannot mean,
/// [`AppError::Marker`] for a configuration whose markers cannot be
/// trusted, and [`AppError::Refused`] when a destination escapes the
/// target, holds bytes no record accounts for, or cannot be written.
pub fn init(
    options: &InitOptions,
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

    let candidate = candidate_for(&target, options)?;
    let mut lines: Vec<String> = candidate
        .destinations
        .iter()
        .map(|destination| destination.path.to_string())
        .collect();
    lines.extend(candidate.notes.iter().cloned());
    lines.push(MANIFEST_PATH.to_string());

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

    let outcome = crate::landing::apply::land(&target, &candidate, &recorded_managed(&target))?;
    Ok(InitOutcome {
        lines,
        applied: true,
        removed: outcome.removed,
    })
}

/// What the target's own record says this tool owns today.
///
/// Read as free JSON, so a record an older release wrote still says which
/// files this tool may refresh and which it may take back.
pub(crate) fn recorded_managed(
    target: &Utf8Path,
) -> Vec<(String, crate::domain::ownership::Sha256)> {
    let Some(value) = recorded_field(target, "managed_files") else {
        return Vec::new();
    };
    let Some(entries) = value.as_array() else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let destination = entry.get("destination")?.as_str()?.to_string();
            let digest = entry.get("sha256")?.as_str()?;
            let digest = digest.parse::<crate::domain::ownership::Sha256>().ok()?;
            Some((destination, digest))
        })
        .collect()
}
