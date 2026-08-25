//! Binary-driven instance upgrade.
//!
//! The newer binary carries the newer payload, so an upgrade is mechanical:
//! refuse atomically while any managed file is locally edited, reinstall
//! from the embedded payload, prune what the new version stopped managing —
//! inside the vendored directory only, and never through a symlink — and
//! report the rule-ID diff the operator reconciles by hand. Copier's model;
//! what changed between versions is the changelog's business.

use camino::{Utf8Path, Utf8PathBuf};

use crate::adapters::fs::sha256_file;
use crate::domain::manifest::{LegacyManifest, MANIFEST_PATH, Manifest, ManifestParseError};
use crate::domain::ownership::Sha256;
use crate::domain::profile::ProfileId;
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::services::installer::{InitOptions, init};

/// What an upgrade was asked to do.
#[derive(Debug, Clone)]
pub struct UpgradeOptions {
    /// The absolute target instance.
    pub target: Utf8PathBuf,
    /// Report the plan and change nothing.
    pub dry_run: bool,
}

/// What an upgrade did.
#[derive(Debug, Default)]
pub struct UpgradeOutcome {
    /// Every line to print, in order.
    pub lines: Vec<String>,
    /// How many lines are conflicts or failures.
    pub failures: usize,
}

struct Installed {
    version: CanonVersion,
    profile: ProfileId,
    docs_root: String,
    managed: Vec<(Utf8PathBuf, Sha256)>,
}

fn read_installed(target: &Utf8Path) -> Result<Installed, AppError> {
    let path = target.join(MANIFEST_PATH);
    if !path.is_file() {
        return Err(AppError::ManifestMissing(path));
    }
    let text = std::fs::read_to_string(&path)?;
    match Manifest::parse(&text) {
        Ok(manifest) => Ok(Installed {
            version: manifest.canon_version,
            profile: manifest.profile,
            docs_root: manifest.docs_root.as_str().to_string(),
            managed: manifest
                .managed_files
                .into_iter()
                .map(|entry| (entry.destination, entry.sha256))
                .collect(),
        }),
        Err(ManifestParseError::Older(1)) => {
            let legacy: LegacyManifest = serde_json::from_str(&text)
                .map_err(|e| AppError::ManifestInvalid(e.to_string()))?;
            Ok(Installed {
                version: legacy.canon_version,
                profile: legacy.profile,
                docs_root: legacy.docs_root.as_str().to_string(),
                managed: legacy
                    .managed_files
                    .into_iter()
                    .map(|entry| (entry.destination, entry.sha256))
                    .collect(),
            })
        }
        Err(error) => Err(AppError::ManifestInvalid(error.to_string())),
    }
}

fn prune(
    target: &Utf8Path,
    dropped: &[Utf8PathBuf],
    outcome: &mut UpgradeOutcome,
) -> Vec<Utf8PathBuf> {
    let mut unremoved = Vec::new();
    for destination in dropped {
        let raw = destination.as_str();
        if raw.starts_with('/')
            || raw == ".."
            || raw.starts_with("../")
            || raw.ends_with("/..")
            || raw.contains("/../")
        {
            outcome.lines.push(format!(
                "refused to remove a destination that leaves the target: {raw}"
            ));
            outcome.failures += 1;
            continue;
        }
        if !raw.starts_with(".spec-driven-docs/") {
            continue;
        }
        let mut prefix = target.to_path_buf();
        let parts: Vec<&str> = raw.split('/').collect();
        let mut escapes = false;
        for part in &parts[..parts.len() - 1] {
            prefix.push(part);
            if prefix.is_symlink() {
                escapes = true;
            }
        }
        let full = target.join(destination);
        if escapes || full.is_symlink() {
            outcome.lines.push(format!(
                "refused to remove a destination reached through a symlink: {raw}"
            ));
            outcome.failures += 1;
            continue;
        }
        if !full.is_file() {
            continue;
        }
        if std::fs::remove_file(&full).is_ok() {
            outcome
                .lines
                .push(format!("removed managed file no longer owned: {raw}"));
        } else {
            unremoved.push(destination.clone());
        }
    }
    unremoved
}

/// Upgrade an installed instance to this binary's version.
///
/// # Errors
///
/// [`AppError::Violations`] when conflicts block the upgrade or removals
/// remain unfinished, [`AppError::Refused`] when the binary is older than
/// the instance or the reinstall refuses, and manifest errors when the
/// record cannot be read.
pub fn upgrade(options: &UpgradeOptions) -> Result<UpgradeOutcome, AppError> {
    if !options.target.is_absolute() {
        return Err(AppError::Usage("target must be absolute".to_string()));
    }
    if !options.target.is_dir() {
        return Err(AppError::Usage(format!(
            "unresolved target: {}",
            options.target
        )));
    }
    let target = Utf8PathBuf::from_path_buf(std::fs::canonicalize(&options.target)?)
        .map_err(|p| AppError::Usage(format!("target is not UTF-8: {}", p.display())))?;

    let installed = read_installed(&target)?;
    let new = CanonVersion::current();
    let old = installed.version;
    let mut outcome = UpgradeOutcome::default();

    if old == new {
        outcome.lines.push(format!("OK already at {new}"));
        return Ok(outcome);
    }
    if old > new {
        return Err(AppError::Refused(format!(
            "sdd {new} is older than the installed canon {old}; upgrade sdd"
        )));
    }

    let mut conflicts = Vec::new();
    for (destination, recorded) in &installed.managed {
        let file = target.join(destination);
        if !file.is_file() {
            conflicts.push(format!("CONFLICT missing managed file: {destination}"));
            continue;
        }
        if sha256_file(&file)? != *recorded {
            conflicts.push(format!(
                "CONFLICT locally edited managed file: {destination}"
            ));
        }
    }
    if !conflicts.is_empty() {
        let count = conflicts.len();
        outcome.lines.extend(conflicts);
        outcome.failures += count;
        return Ok(outcome);
    }

    if options.dry_run {
        outcome
            .lines
            .push(format!("DRY RUN upgrade {old} to {new}"));
        return Ok(outcome);
    }

    init(&InitOptions {
        target: target.clone(),
        profile: installed.profile,
        apply: true,
        dry_run: false,
    })
    .map_err(|error| {
        AppError::Refused(format!(
            "upgrade aborted during reinstall from {old} to {new}: {error}"
        ))
    })?;

    finish(&target, &installed, old, new, &mut outcome)?;
    Ok(outcome)
}

fn finish(
    target: &Utf8Path,
    installed: &Installed,
    old: CanonVersion,
    new: CanonVersion,
    outcome: &mut UpgradeOutcome,
) -> Result<(), AppError> {
    let fresh = Manifest::parse(&std::fs::read_to_string(target.join(MANIFEST_PATH))?)
        .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;
    let kept: std::collections::BTreeSet<&Utf8PathBuf> = fresh
        .managed_files
        .iter()
        .map(|entry| &entry.destination)
        .collect();
    let dropped: Vec<Utf8PathBuf> = installed
        .managed
        .iter()
        .map(|(destination, _)| destination.clone())
        .filter(|destination| !kept.contains(destination))
        .collect();
    let unremoved = prune(target, &dropped, outcome);
    if !unremoved.is_empty() {
        outcome.lines.push(
            "FAIL these files are no longer owned and could not be removed; delete them by hand:"
                .to_string(),
        );
        for destination in &unremoved {
            outcome.lines.push(format!("  {destination}"));
        }
        outcome.lines.push(format!(
            "the payload and manifest are upgraded to {new}; only these removals remain, and"
        ));
        outcome.lines.push(format!(
            "re-running this upgrade will report 'already at {new}' rather than retry them"
        ));
        outcome.failures += unremoved.len();
    }

    let mut local_ids = std::collections::BTreeSet::new();
    let specs = target.join(&installed.docs_root).join("specs");
    if let Ok(entries) = specs.read_dir_utf8() {
        for entry in entries.filter_map(Result::ok) {
            if let Ok(text) = std::fs::read_to_string(entry.path()) {
                local_ids.extend(crate::embedded::rule_ids_in(&text));
            }
        }
    }
    let upstream_only: Vec<String> = crate::embedded::spec_rule_ids()
        .difference(&local_ids)
        .cloned()
        .collect();
    if !upstream_only.is_empty() {
        outcome
            .lines
            .push("upstream rule IDs not present locally:".to_string());
        for id in upstream_only {
            outcome.lines.push(format!("  {id}"));
        }
    }

    if outcome.failures > 0 {
        outcome.lines.push(format!(
            "FAIL upgraded {old} to {new} with unfinished removals above"
        ));
    } else {
        outcome.lines.push(format!("OK upgraded {old} to {new}"));
    }
    Ok(())
}
