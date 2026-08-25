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

fn read_manifest(target: &Utf8Path) -> Result<Manifest, AppError> {
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

    let mut expected: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    if self_layout {
        for projection in declaration.managed {
            expected.push(projection.source.to_string());
        }
        for file in crate::embedded::SPECS.files() {
            if let Some(name) = file.path().as_os_str().to_str() {
                expected.push(format!("_docs/specs/{name}"));
            }
        }
        expected.push("_docs/decisions/TEMPLATE-adr.md".to_string());
        expected.push("_docs/reference/TEMPLATE-agents-digest.md".to_string());
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
    if !manifest
        .integration_blocks
        .iter()
        .any(|block| block.path == ".pre-commit-config.yaml")
    {
        report.fail("FAIL manifest records no integration block for .pre-commit-config.yaml");
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

    for entry in &manifest.managed_files {
        let file = target.join(&entry.destination);
        if reached_through_symlink(target, &entry.destination) {
            report.fail(format!(
                "FAIL managed file reached through a symlink: {}",
                entry.destination
            ));
            continue;
        }
        if !file.is_file() {
            report.fail(format!("FAIL missing managed file: {}", entry.destination));
            continue;
        }
        if sha256_file(&file)? != entry.sha256 {
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
            report.note(format!(
                "DRIFT adopted file requires reconciliation: {}",
                entry.destination
            ));
        }
    }

    check_projection(&manifest, &mut report);
    check_integration(target, &manifest, &mut report)?;
    check_specs(target, &manifest, &mut report)?;

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

fn check_integration(
    target: &Utf8Path,
    manifest: &Manifest,
    report: &mut VerifyReport,
) -> Result<(), AppError> {
    let config_path = target.join(".pre-commit-config.yaml");
    if reached_through_symlink(target, Utf8Path::new(".pre-commit-config.yaml")) {
        report.fail("FAIL .pre-commit-config.yaml reached through a symlink");
    } else if config_path.is_file() {
        let config = std::fs::read_to_string(&config_path)?;
        let begins = config.lines().filter(|line| *line == marker::BEGIN).count();
        let ends = config.lines().filter(|line| *line == marker::END).count();
        if begins != 1 {
            report.fail("FAIL missing managed pre-commit block");
        } else if ends != 1 {
            report.fail("FAIL malformed managed pre-commit block");
        } else {
            let recorded = manifest
                .integration_blocks
                .iter()
                .find(|block| block.path == ".pre-commit-config.yaml")
                .map(|block| &block.marker_hash);
            match (recorded, marker::block_hash(&config)) {
                (None, _) => {
                    report.fail("FAIL manifest records no marker hash for .pre-commit-config.yaml");
                }
                (Some(recorded), Some(present)) if *recorded == present => {
                    if let Some(block) = marker::block_region(&config) {
                        check_block_entries(&block, report);
                    }
                }
                (Some(_), _) => report.fail("FAIL managed block tampered: .pre-commit-config.yaml"),
            }
        }
    } else {
        report.fail("FAIL missing .pre-commit-config.yaml");
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
                // The corpus convention is lowercase.
                #[allow(clippy::case_sensitive_file_extension_comparisons)]
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
