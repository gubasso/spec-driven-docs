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
use crate::domain::manifest::{CANON_SOURCE, MANIFEST_PATH, Manifest, SCHEMA_VERSION};
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

fn installed_at(target: &Utf8Path) -> String {
    std::fs::read_to_string(target.join(MANIFEST_PATH))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| {
            value
                .get("installed_at")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| {
            jiff::Timestamp::now()
                .strftime("%Y-%m-%dT%H:%M:%SZ")
                .to_string()
        })
}

struct TargetState {
    files: Vec<(Utf8PathBuf, Vec<u8>)>,
    lines: Vec<String>,
}

fn compute_target_state(target: &Utf8Path, profile: ProfileId) -> Result<TargetState, AppError> {
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

    for projection in declaration.adopted {
        let seed = crate::embedded::asset(projection.source)
            .ok_or_else(|| anyhow::anyhow!("payload asset missing: {}", projection.source))?;
        let destination = resolve_destination(projection.destination, declaration.docs_root);
        let existing = target.join(&destination);
        let bytes = if existing.is_file() {
            std::fs::read(&existing)?
        } else {
            seed.to_vec()
        };
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
    let block = render_block(&RenderOptions {
        docs_root: declaration.docs_root.to_string(),
        indent,
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

    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        canon_version: CanonVersion::current(),
        canon_source: CANON_SOURCE.to_string(),
        profile,
        docs_root: declaration.docs_root,
        installed_at: installed_at(target),
        managed_files: managed_entries,
        adopted_files: adopted_entries,
        integration_blocks: vec![IntegrationBlock {
            path: ".pre-commit-config.yaml".into(),
            marker_hash,
        }],
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
    let abort = |unrestored: Vec<Utf8PathBuf>| {
        if unrestored.is_empty() {
            AppError::Refused("apply aborted; the target was restored".to_string())
        } else {
            let paths: Vec<&str> = unrestored.iter().map(|p| p.as_str()).collect();
            AppError::Refused(format!(
                "apply aborted and restoration is incomplete; verify by hand: {}",
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

    if write_all().is_err() {
        return Err(abort(rollback(&backups)));
    }

    match verifier::verify(target) {
        Ok(report) if report.failures == 0 => Ok(()),
        _ => Err(abort(rollback(&backups))),
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

    let state = compute_target_state(&target, options.profile)?;
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
