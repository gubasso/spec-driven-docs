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
    /// Report what would change and change nothing.
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
    integration: Vec<(Utf8PathBuf, Sha256)>,
    /// Whether the record is already at this binary's schema.
    ///
    /// The two versions move independently, so an instance can carry this
    /// binary's canon version and an older schema. Without this the "already
    /// at" shortcut would return before migrating, while every other verb
    /// refuses the record and sends the operator back here.
    schema_current: bool,
}

/// The marker pair a host file's managed region uses.
fn markers_for(path: &str) -> (&'static str, &'static str) {
    use crate::domain::marker::{AGENTS_BEGIN, AGENTS_END, BEGIN, END};
    if path == crate::domain::paths::HOOKS_CONFIG_PATH {
        (BEGIN, END)
    } else {
        (AGENTS_BEGIN, AGENTS_END)
    }
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
            integration: manifest
                .integration_blocks
                .into_iter()
                .map(|block| (block.path, block.marker_hash))
                .collect(),
            schema_current: true,
        }),
        Err(ManifestParseError::Older(_)) => {
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
                integration: legacy
                    .integration_blocks
                    .into_iter()
                    .map(|block| (block.path, block.marker_hash))
                    .collect(),
                schema_current: false,
            })
        }
        Err(error) => Err(AppError::ManifestInvalid(error.to_string())),
    }
}

/// Refuse an answer on a path where no plan offers a decision.
///
/// A target with nothing to do and one whose managed files were edited
/// both end before a plan exists. An answer given on either is an answer
/// to a question nobody asked, and letting it pass silently would make
/// typing look like consent on the one path where nothing checked it.
/// A downgrade refuses on its own terms before this can matter.
///
/// # Errors
///
/// [`AppError::Usage`] naming the decision nothing offered.
/// Every managed file and region the target no longer holds as recorded.
///
/// A reinstall replaces a managed file and re-splices a managed region, so
/// an edit to either would be lost. An edit outside the markers is the
/// project's own and survives, which is why the region is compared by its
/// own hash rather than the host file's.
///
/// # Errors
///
/// Any I/O error reading a destination.
fn conflicts_at(target: &Utf8Path, installed: &Installed) -> Result<Vec<String>, AppError> {
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
    for (path, recorded) in &installed.integration {
        let full = target.join(path);
        if !full.is_file() {
            conflicts.push(format!("CONFLICT missing integration host: {path}"));
            continue;
        }
        let (begin, end) = markers_for(path.as_str());
        let host = std::fs::read_to_string(&full)?;
        match crate::domain::marker::block_hash_with(&host, begin, end) {
            Some(present) if present == *recorded => {}
            _ => conflicts.push(format!("CONFLICT locally edited managed block: {path}")),
        }
    }
    Ok(conflicts)
}

/// The options a reinstall carries.
///
/// No flag: the reinstall carries the recorded declarations forward, and
/// the project's own declaration file is adopted, so the reinstall reads
/// it rather than replacing it.
fn reinstall_options(target: &Utf8Path, profile: ProfileId) -> InitOptions {
    InitOptions {
        target: target.to_path_buf(),
        profile,
        apply: false,
        dry_run: true,
        docs_scratch: None,
        reserve: Vec::new(),
        writing_style: None,
    }
}

/// Report what the landing took back.
///
/// The removals are the landing's own, taken only where the record
/// vouched for the bytes it removed. This only says what happened.
fn report_removals(removed: &[String], outcome: &mut UpgradeOutcome) {
    for raw in removed {
        outcome
            .lines
            .push(format!("removed managed file no longer owned: {raw}"));
    }
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
    // The version landed is the version running. The operator chose which
    // binary to install, and that binary projects only itself.
    let new = CanonVersion::current();
    let old = installed.version;
    let mut outcome = UpgradeOutcome::default();

    if old > new {
        return Err(AppError::Refused(format!(
            "sdd {new} is older than the installed canon {old}; upgrade sdd"
        )));
    }

    // The conflict scan comes before the version shortcut. A managed file
    // edited in a current instance is exactly the drift this verb serves,
    // and a shortcut that reported success first would leave the one route
    // to it unable to do its job.
    let conflicts = conflicts_at(&target, &installed)?;
    if !conflicts.is_empty() {
        let count = conflicts.len();
        outcome.lines.extend(conflicts);
        outcome.failures += count;
        return Ok(outcome);
    }

    if old == new && installed.schema_current {
        outcome.lines.push(format!("OK already at {new}"));
        return Ok(outcome);
    }

    if options.dry_run {
        // At the same canon version the work is the record's schema alone,
        // so saying "upgrade X to X" would describe nothing.
        if old == new {
            outcome
                .lines
                .push(format!("DRY RUN migrate the record of {new}"));
        } else {
            outcome
                .lines
                .push(format!("DRY RUN upgrade {old} to {new}"));
        }
        // A seed the landing will not write, and a file it cannot account
        // for, are what a dry run exists to show.
        let preview = init(
            &reinstall_options(&target, installed.profile),
            crate::landing::classify::Intent::Reconcile,
        )
        .map_err(|error| {
            AppError::Refused(format!(
                "upgrade could not be previewed from {old} to {new}: {error}"
            ))
        })?;
        outcome.lines.extend(
            preview
                .lines
                .into_iter()
                .filter(|line| line.starts_with("note:")),
        );
        return Ok(outcome);
    }

    let reinstalled = init(
        &InitOptions {
            apply: true,
            dry_run: false,
            ..reinstall_options(&target, installed.profile)
        },
        // The upgrade already classified the target; the reinstall is its
        // own act rather than a second landing decision.
        crate::landing::classify::Intent::Reconcile,
    )
    .map_err(|error| {
        AppError::Refused(format!(
            "upgrade aborted during reinstall from {old} to {new}: {error}"
        ))
    })?;
    // The reinstall's destination list is noise here, and its notes are
    // not: a seed that did not land because the project already holds the
    // destination is something the operator must hear about once.
    let removed = reinstalled.removed.clone();
    outcome.lines.extend(
        reinstalled
            .lines
            .into_iter()
            .filter(|line| line.starts_with("note:")),
    );

    finish(&target, &installed, &removed, old, new, &mut outcome);
    Ok(outcome)
}

fn finish(
    target: &Utf8Path,
    installed: &Installed,
    removed: &[String],
    old: CanonVersion,
    new: CanonVersion,
    outcome: &mut UpgradeOutcome,
) {
    report_removals(removed, outcome);

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
    } else if old == new {
        outcome
            .lines
            .push(format!("OK migrated the record of {new}"));
    } else {
        outcome.lines.push(format!("OK upgraded {old} to {new}"));
    }
}
