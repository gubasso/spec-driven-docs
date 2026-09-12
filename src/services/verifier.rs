//! Offline instance verification.
//!
//! Reads the manifest, compares every recorded projection to the disk, holds
//! the managed block to its recorded hash, checks the block's `sdd` entries
//! resolve, scans for duplicate rule IDs, and compares the instance's canon
//! version to this binary's. Everything is local: no network, no canon
//! checkout, no ambient tools. What gets recorded is the installer's
//! business; this only holds the record to the disk.

use camino::Utf8Path;

use crate::adapters::fs::{DestinationRefusal, check_destination, sha256_file};
use crate::domain::gate_id::GateId;
use crate::domain::manifest::{MANIFEST_PATH, Manifest, ManifestParseError};
use crate::domain::marker;
use crate::domain::version::CanonVersion;
use crate::error::AppError;

/// What a verification run reports.
#[derive(Debug, Default)]
pub struct VerifyReport {
    /// Every line to print, failures and notes alike, in order.
    pub lines: Vec<String>,
    /// How many lines are failures.
    pub failures: usize,
    /// How many managed files are missing, symlinked, or byte-drifted.
    pub managed_drift: usize,
    /// How many adopted files await reconciliation.
    pub adopted_drift: usize,
}

impl VerifyReport {
    fn fail(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
        self.failures += 1;
    }

    fn note(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }
}

pub(crate) fn read_manifest(target: &Utf8Path) -> Result<Manifest, AppError> {
    let path = target.join(MANIFEST_PATH);
    if reached_through_symlink(target, Utf8Path::new(MANIFEST_PATH)) {
        return Err(AppError::ManifestInvalid(
            "manifest reached through a symlink".to_string(),
        ));
    }
    if !path.is_file() {
        return Err(AppError::ManifestMissing(path));
    }
    let text = std::fs::read_to_string(&path)?;
    Manifest::parse(&text).map_err(|error| match error {
        ManifestParseError::Invalid(detail) => AppError::ManifestInvalid(detail),
        other => AppError::ManifestInvalid(other.to_string()),
    })
}

fn check_block_entries(block: &str, report: &mut VerifyReport) {
    let mut sdd_entries = 0usize;
    for line in block.lines() {
        let Some(entry) = line.trim_start().strip_prefix("entry: ") else {
            continue;
        };
        let words: Vec<&str> = entry.split_whitespace().collect();
        let Some(gate_position) = words.iter().position(|word| *word == "gate") else {
            if words.last() == Some(&"verify") {
                sdd_entries += 1;
            }
            continue;
        };
        sdd_entries += 1;
        match words.get(gate_position + 1) {
            Some(id) if id.parse::<GateId>().is_ok() => {}
            Some(id) => report.fail(format!(
                "FAIL managed block entry names an unknown gate: {id}"
            )),
            None => report.fail(format!("FAIL managed block entry names no gate: {entry}")),
        }
    }
    if sdd_entries == 0 {
        report.fail("FAIL managed block wires no sdd entry");
    }
}

fn reached_through_symlink(target: &Utf8Path, destination: &Utf8Path) -> bool {
    matches!(
        check_destination(target, destination),
        Err(DestinationRefusal::SymlinkEscape)
    )
}

/// The manifest must record every projection the profile declares — an
/// omitted record is an owned file the verifier would silently stop
/// holding. Applies when the instance is at this binary's version, in
/// whichever of the two layouts the record claims: the installed layout,
/// or the canon's self-manifest layout where every owned file sits at its
/// authored path. Either way the complete expected set for that layout is
/// required, so no hand edit of the record can shrink what is held —
/// masquerading as the other layout only changes which files must exist
/// and hash clean.
/// The declaration and the managed block must say the same thing.
///
/// The block is rendered from the declaration, so a difference means the
/// project edited the declaration and nothing reached the block. That is a
/// real disagreement between two artifacts that claim to agree, not drift in
/// a file the project owns, so it fails and names the command that fixes it.
///
/// A declaration that does not parse fails once here rather than once from
/// every always-run gate.
fn check_declaration(target: &Utf8Path, manifest: &Manifest, report: &mut VerifyReport) {
    let declaration = match crate::domain::instance_config::InstanceConfig::read(target) {
        Ok(declaration) => declaration,
        Err(error) => {
            report.fail(format!("FAIL {error}"));
            return;
        }
    };

    let config = target.join(crate::commands::hooks::CONFIG);
    let Ok(host) = std::fs::read_to_string(&config) else {
        return;
    };
    // Measure from the host stripped of its block, which is what the
    // installer measures. With the block still in place the first item under
    // `repos:` is the block's own, and the depth read back differs.
    let Ok((base, _)) = crate::domain::marker::split_block(&host) else {
        return;
    };
    // The indentation is the host file's, measured the same way the
    // installer measures it. Rendering with the default would report every
    // instance whose `repos:` items sit at another depth.
    let Ok(indent) = crate::domain::marker::splice_indent(&base) else {
        return;
    };
    let rendered = crate::services::hooks_render::render_block(
        &crate::services::hooks_render::RenderOptions {
            docs_root: manifest.docs_root.to_string(),
            indent,
            declaration,
            ..crate::services::hooks_render::RenderOptions::default()
        },
    );
    // Compare per gate rather than over the whole region. A region may
    // carry hooks this renderer never emits, and this repository's own does,
    // so byte equality would report every such instance as stale.
    let expected = crate::services::hooks_render::selectors(&rendered);
    let Some(region) = crate::domain::marker::block_region(&host) else {
        return;
    };
    let found = crate::services::hooks_render::selectors(&region);
    for (id, wanted) in &expected {
        let Some(actual) = found.get(id) else {
            continue;
        };
        if actual != wanted {
            report.fail(format!(
                "FAIL the wiring for {id} in {} does not match the declaration; run 'sdd hooks --apply'",
                crate::commands::hooks::CONFIG
            ));
        }
    }
}

fn check_projection(manifest: &Manifest, report: &mut VerifyReport) {
    if manifest.canon_version != CanonVersion::current() {
        return;
    }
    let declaration = manifest.profile.profile();
    let managed: std::collections::BTreeSet<&str> = manifest
        .managed_files
        .iter()
        .map(|entry| entry.destination.as_str())
        .collect();
    let adopted: std::collections::BTreeSet<&str> = manifest
        .adopted_files
        .iter()
        .map(|entry| entry.destination.as_str())
        .collect();

    let self_layout = declaration
        .managed
        .iter()
        .all(|projection| managed.contains(projection.source));

    let mut expected: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut missing: Vec<String> = Vec::new();
    if self_layout {
        for projection in declaration.managed {
            expected.insert(projection.source.to_string());
        }
        for file in crate::embedded::SPECS.files() {
            if let Some(name) = file.path().as_os_str().to_str() {
                expected.insert(format!("_docs/specs/{name}"));
            }
        }
        for template in crate::domain::profile::CANON_TEMPLATES {
            expected.insert((*template).to_string());
        }
        for destination in &expected {
            if !managed.contains(destination.as_str()) && !adopted.contains(destination.as_str()) {
                missing.push(destination.clone());
            }
        }
    } else {
        for projection in declaration.managed {
            if !managed.contains(projection.destination) {
                missing.push(projection.destination.to_string());
            }
        }
        for projection in declaration.adopted {
            let destination = crate::domain::profile::resolve_destination(
                projection.destination,
                manifest.docs_root,
            );
            if !adopted.contains(destination.as_str()) {
                missing.push(destination.to_string());
            }
        }
    }
    for destination in missing {
        report.fail(format!(
            "FAIL manifest omits a declared projection: {destination}"
        ));
    }
    // Every installed instance carries both integration blocks. The canon's
    // own layout carries the pre-commit block by hand; its root AGENTS.md is
    // release-kit-owned and outside this projection.
    let required: &[&str] = if self_layout {
        &[".pre-commit-config.yaml"]
    } else {
        &[".pre-commit-config.yaml", "AGENTS.md"]
    };
    for path in required {
        if !manifest
            .integration_blocks
            .iter()
            .any(|block| block.path.as_str() == *path)
        {
            report.fail(format!(
                "FAIL manifest records no integration block for {path}"
            ));
        }
    }
}

/// Verify an installed instance offline.
///
/// # Errors
///
/// [`AppError::ManifestMissing`] / [`AppError::ManifestInvalid`] when the
/// record itself cannot be trusted, and I/O errors when the disk cannot be
/// read; recorded-versus-disk differences are reported, not raised.
pub fn verify(target: &Utf8Path) -> Result<VerifyReport, AppError> {
    let manifest = read_manifest(target)?;
    let mut report = VerifyReport::default();

    check_declaration(target, &manifest, &mut report);

    for entry in &manifest.managed_files {
        let file = target.join(&entry.destination);
        if reached_through_symlink(target, &entry.destination) {
            report.managed_drift += 1;
            report.fail(format!(
                "FAIL managed file reached through a symlink: {}",
                entry.destination
            ));
            continue;
        }
        if !file.is_file() {
            report.managed_drift += 1;
            report.fail(format!("FAIL missing managed file: {}", entry.destination));
            continue;
        }
        if sha256_file(&file)? != entry.sha256 {
            report.managed_drift += 1;
            report.fail(format!("FAIL managed drift: {}", entry.destination));
        }
    }

    for entry in &manifest.adopted_files {
        let file = target.join(&entry.destination);
        if reached_through_symlink(target, &entry.destination) {
            report.fail(format!(
                "FAIL adopted file reached through a symlink: {}",
                entry.destination
            ));
            continue;
        }
        if !file.is_file() {
            report.fail(format!("FAIL missing adopted file: {}", entry.destination));
            continue;
        }
        if sha256_file(&file)? != entry.sha256 {
            report.adopted_drift += 1;
            report.note(format!(
                "DRIFT adopted file requires reconciliation: {}",
                entry.destination
            ));
        }
    }

    check_projection(&manifest, &mut report);
    check_integration(target, &manifest, &mut report)?;
    check_specs(target, &manifest, &mut report)?;
    check_debt(target, &mut report);
    for reconciliation in crate::services::policy::needed(target, manifest.docs_root)? {
        report.note(reconciliation.note(manifest.docs_root));
    }

    let current = CanonVersion::current();
    if manifest.canon_version > current {
        report.fail(format!(
            "FAIL sdd {current} is older than the installed canon {}; upgrade sdd",
            manifest.canon_version
        ));
    } else if manifest.canon_version < current {
        report.note(format!(
            "note: sdd {current} is newer than the installed canon {}; run 'sdd upgrade'",
            manifest.canon_version
        ));
    }

    if report.failures == 0 {
        report.note(format!(
            "OK spec-driven-docs {} at {target}",
            manifest.canon_version
        ));
    }
    Ok(report)
}

/// The debt file must be readable, and it must be the only debt format.
///
/// A file no budget gate can read fails once here rather than once from
/// every budget gate. The legacy list alone is a note naming its migration:
/// it still works, and nothing about it is wrong until the day the project
/// wants a ceiling that only shrinks.
fn check_debt(target: &Utf8Path, report: &mut VerifyReport) {
    use crate::domain::debt::{Debt, LEGACY_DEBT_PATH, Presence};
    let presence = Presence::at(target);
    if let Err(error) = Debt::read(target) {
        report.fail(format!("FAIL {error}"));
        return;
    }
    if presence.legacy {
        report.note(format!(
            "note: {LEGACY_DEBT_PATH} is the legacy debt list, which skips a listed chapter instead of holding it to a ceiling; run 'sdd debt migrate --apply'"
        ));
    }
}

/// The marker pair a host file's managed region uses.
fn markers_for(path: &str) -> (&'static str, &'static str) {
    if path == ".pre-commit-config.yaml" {
        (marker::BEGIN, marker::END)
    } else {
        (marker::AGENTS_BEGIN, marker::AGENTS_END)
    }
}

fn check_integration(
    target: &Utf8Path,
    manifest: &Manifest,
    report: &mut VerifyReport,
) -> Result<(), AppError> {
    for block in &manifest.integration_blocks {
        let path = block.path.as_str();
        let (begin, end) = markers_for(path);
        let full = target.join(&block.path);
        if reached_through_symlink(target, &block.path) {
            report.fail(format!("FAIL {path} reached through a symlink"));
            continue;
        }
        if !full.is_file() {
            report.fail(format!("FAIL missing integration host: {path}"));
            continue;
        }
        let host = std::fs::read_to_string(&full)?;
        let begins = host.lines().filter(|line| *line == begin).count();
        let ends = host.lines().filter(|line| *line == end).count();
        if begins != 1 {
            report.fail(format!("FAIL missing managed block: {path}"));
            continue;
        }
        if ends != 1 {
            report.fail(format!("FAIL malformed managed block: {path}"));
            continue;
        }
        match marker::block_hash_with(&host, begin, end) {
            Some(present) if present == block.marker_hash => {
                if path == ".pre-commit-config.yaml"
                    && let Some(region) = marker::block_region_with(&host, begin, end)
                {
                    check_block_entries(&region, report);
                }
            }
            _ => report.fail(format!("FAIL managed block tampered: {path}")),
        }
    }
    Ok(())
}

fn check_specs(
    target: &Utf8Path,
    manifest: &Manifest,
    report: &mut VerifyReport,
) -> Result<(), AppError> {
    let specs = target.join(manifest.docs_root.as_str()).join("specs");
    if specs.is_dir() {
        let mut counts = std::collections::BTreeMap::new();
        let mut names: Vec<_> = specs
            .read_dir_utf8()?
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string())
            .filter(|name| {
                #[allow(
                    clippy::case_sensitive_file_extension_comparisons,
                    reason = "the corpus convention is lowercase"
                )]
                name.ends_with(".md")
            })
            .collect();
        names.sort();
        for name in names {
            let text = std::fs::read_to_string(specs.join(name))?;
            for id in crate::embedded::rule_ids_in(&text) {
                *counts.entry(id).or_insert(0usize) += 1;
            }
        }
        let duplicated: Vec<String> = counts
            .into_iter()
            .filter(|(_, n)| *n > 1)
            .map(|(id, _)| id)
            .collect();
        if !duplicated.is_empty() {
            report.fail("FAIL duplicate rule ID in local specs");
            for id in duplicated {
                report.note(format!("### `{id}`"));
            }
        }
    } else {
        report.fail(format!(
            "FAIL missing local specs: {}/specs",
            manifest.docs_root
        ));
    }
    Ok(())
}
