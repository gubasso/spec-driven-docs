//! Instance installation: project the embedded payload into a target.
//!
//! The chezmoi vocabulary applies: the embedded payload and profile are the
//! source state, the computed projection is the target state, the repository
//! on disk is the destination state, and the manifest — written last — is
//! the persistent entry state. The whole target state is computed before a
//! byte lands; a non-empty target previews by default; every destination is
//! guarded; and any failure mid-apply rolls the target back. What the
//! payload contains is `embedded`'s and the profiles' business.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};

use crate::adapters::fs::{DestinationRefusal, check_destination, write_file};
use crate::domain::manifest::{
    CANON_SOURCE, MANIFEST_PATH, Manifest, PlanZone, SCHEMA_VERSION, validate_docs_scratch_path,
    validate_plan_zone_path,
};
use crate::domain::ownership::{AdoptedEntry, IntegrationBlock, ManagedEntry, Sha256};
use crate::domain::profile::{ProfileId, resolve_destination};
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::services::hooks_render::{RenderOptions, render_block};
use crate::services::verifier;

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
}

/// What an installation did.
#[derive(Debug)]
pub struct InitOutcome {
    /// Every line to print: the proposed destinations, then any notices.
    pub lines: Vec<String>,
    /// Whether files were written.
    pub applied: bool,
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

struct TargetState {
    files: Vec<(Utf8PathBuf, Vec<u8>)>,
    lines: Vec<String>,
}

#[allow(
    clippy::too_many_lines,
    reason = "computing the target state is one ordered pass the installer replays"
)]
fn compute_target_state(target: &Utf8Path, options: &InitOptions) -> Result<TargetState, AppError> {
    let profile = options.profile;
    let declaration = profile.profile();
    let mut files: Vec<(Utf8PathBuf, Vec<u8>)> = Vec::new();
    let mut lines = Vec::new();
    let mut managed_entries = Vec::new();
    let mut adopted_entries = Vec::new();

    for projection in declaration.managed {
        let bytes = crate::embedded::asset(projection.source)
            .ok_or_else(|| anyhow::anyhow!("payload asset missing: {}", projection.source))?;
        let destination = Utf8PathBuf::from(projection.destination);
        managed_entries.push(ManagedEntry {
            source: projection.source.into(),
            destination: destination.clone(),
            sha256: Sha256::of(bytes),
        });
        lines.push(destination.to_string());
        files.push((destination, bytes.to_vec()));
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
        let seed = crate::embedded::asset(projection.source)
            .ok_or_else(|| anyhow::anyhow!("payload asset missing: {}", projection.source))?;
        let destination = resolve_destination(projection.destination, declaration.docs_root);
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
            seed.to_vec()
        };
        // `--reserve` records into the declaration, keeping its comments and
        // whatever the project already wrote there.
        if destination == crate::domain::instance_config::CONFIG_PATH && !options.reserve.is_empty()
        {
            if let Ok(text) = std::str::from_utf8(&bytes) {
                bytes = crate::domain::instance_config::with_reserved(text, &options.reserve)
                    .into_bytes();
            }
        }
        adopted_entries.push(AdoptedEntry {
            source: projection.source.into(),
            destination: destination.clone(),
            sha256: Sha256::of(&bytes),
            baseline_sha256: Sha256::of(seed),
        });
        lines.push(destination.to_string());
        files.push((destination, bytes));
    }

    let config_path = target.join(".pre-commit-config.yaml");
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
    let block = render_block(&RenderOptions {
        docs_root: declaration.docs_root.to_string(),
        indent,
        declaration: declared,
        ..RenderOptions::default()
    });
    let spliced = crate::domain::marker::splice(&base, &block)?;
    let marker_hash = crate::domain::marker::block_hash(&spliced)
        .ok_or_else(|| anyhow::anyhow!("the rendered block lost its markers"))?;
    lines.push(".pre-commit-config.yaml".to_string());
    files.push((
        Utf8PathBuf::from(".pre-commit-config.yaml"),
        spliced.into_bytes(),
    ));

    let mut integration_blocks = vec![IntegrationBlock {
        path: ".pre-commit-config.yaml".into(),
        marker_hash,
    }];

    // The root AGENTS.md documentation block routes authors to the context they
    // load before editing. A symlinked host is refused before it is read, so a
    // link cannot redirect the read outside the target.
    let agents_relative = Utf8Path::new("AGENTS.md");
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
    let agents_block =
        crate::services::agents_render::render_block(&declaration.docs_root.to_string());
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
    lines.push("AGENTS.md".to_string());
    files.push((agents_relative.to_path_buf(), agents.into_bytes()));
    integration_blocks.push(IntegrationBlock {
        path: "AGENTS.md".into(),
        marker_hash: agents_hash,
    });

    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        canon_version: CanonVersion::current(),
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

fn refusal_line(destination: &Utf8Path, refusal: &DestinationRefusal) -> String {
    match refusal {
        DestinationRefusal::SymlinkEscape => {
            format!("destination escapes the target through a symlink: {destination}")
        }
        DestinationRefusal::FileBlocksDirectory(blocked) => {
            format!("a file blocks a directory the install needs: {blocked}")
        }
        DestinationRefusal::NotARegularFile => {
            format!("destination exists and is not a regular file: {destination}")
        }
    }
}

fn apply(target: &Utf8Path, state: &TargetState) -> Result<(), AppError> {
    let mut ordered: Vec<&(Utf8PathBuf, Vec<u8>)> = state.files.iter().collect();
    ordered.sort_by(|a, b| a.0.as_str().as_bytes().cmp(b.0.as_str().as_bytes()));

    for (destination, _) in &ordered {
        check_destination(target, destination)
            .map_err(|refusal| AppError::Refused(refusal_line(destination, &refusal)))?;
    }

    let mut backups: BTreeMap<Utf8PathBuf, Option<Vec<u8>>> = BTreeMap::new();
    let rollback = |backups: &BTreeMap<Utf8PathBuf, Option<Vec<u8>>>| -> Vec<Utf8PathBuf> {
        let mut unrestored = Vec::new();
        for (destination, previous) in backups {
            let full = target.join(destination);
            let restored = previous.as_ref().map_or_else(
                || std::fs::remove_file(&full).is_ok() || !full.exists(),
                |bytes| write_file(&full, bytes).is_ok(),
            );
            if !restored {
                unrestored.push(destination.clone());
            }
        }
        unrestored
    };
    // The cause travels with the refusal: the caller has already lost the
    // written tree by the time it reads this, so a bare "aborted" leaves
    // nothing to act on.
    let abort = |unrestored: Vec<Utf8PathBuf>, cause: &str| {
        if unrestored.is_empty() {
            AppError::Refused(format!("apply aborted; the target was restored: {cause}"))
        } else {
            let paths: Vec<&str> = unrestored.iter().map(|p| p.as_str()).collect();
            AppError::Refused(format!(
                "apply aborted and restoration is incomplete; verify by hand: {}: {cause}",
                paths.join(" ")
            ))
        }
    };

    for (destination, _) in &ordered {
        let full = target.join(destination);
        let previous = if full.is_file() {
            Some(std::fs::read(&full).map_err(|source| {
                AppError::Refused(format!("cannot back up {destination}: {source}"))
            })?)
        } else {
            None
        };
        backups.insert((*destination).clone(), previous);
    }

    let write_all = || -> std::io::Result<()> {
        for (destination, bytes) in &ordered {
            if destination.as_str() != MANIFEST_PATH {
                write_file(&target.join(destination), bytes)?;
            }
        }
        for (destination, bytes) in &ordered {
            if destination.as_str() == MANIFEST_PATH {
                write_file(&target.join(destination), bytes)?;
            }
        }
        Ok(())
    };

    if let Err(source) = write_all() {
        return Err(abort(
            rollback(&backups),
            &format!("write failed: {source}"),
        ));
    }

    match verifier::verify(target) {
        Ok(report) if report.failures == 0 => Ok(()),
        Ok(report) => {
            let failures: Vec<&str> = report
                .lines
                .iter()
                .filter(|line| line.starts_with("FAIL"))
                .map(String::as_str)
                .collect();
            let cause = failures.join("; ");
            Err(abort(rollback(&backups), &cause))
        }
        Err(source) => Err(abort(
            rollback(&backups),
            &format!("the written target could not be verified: {source}"),
        )),
    }
}

/// Install or reinstall an instance.
///
/// # Errors
///
/// [`AppError::Usage`] for a target the arguments cannot mean,
/// [`AppError::Marker`] for a configuration whose markers cannot be trusted,
/// and [`AppError::Refused`] when the apply could not complete — the target
/// is restored before that returns.
pub fn init(options: &InitOptions) -> Result<InitOutcome, AppError> {
    let target = canonical_target(&options.target)?;
    let forced_dry = !options.apply
        && !options.dry_run
        && target_has_content(&target)?
        && !target.join(MANIFEST_PATH).is_file();
    let dry = options.dry_run || forced_dry;

    let state = compute_target_state(&target, options)?;
    let mut lines = state.lines.clone();

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
        });
    }

    apply(&target, &state)?;
    Ok(InitOutcome {
        lines,
        applied: true,
    })
}
