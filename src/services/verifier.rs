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
use crate::domain::paths::{AGENTS_DIGEST_PATH, HOOKS_CONFIG_PATH};
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

    // The documentation block carries the writing-style route the
    // declaration selects, so a stale route is the same disagreement as a
    // stale hook filter, and the same command repairs it.
    let agents = target.join(crate::commands::hooks::AGENTS);
    let agents_recorded = manifest
        .integration_blocks
        .iter()
        .any(|block| block.path.as_str() == crate::commands::hooks::AGENTS);
    if agents_recorded
        && let Ok(host) = std::fs::read_to_string(&agents)
        && let Some(region) = crate::domain::marker::block_region_with(
            &host,
            crate::domain::marker::AGENTS_BEGIN,
            crate::domain::marker::AGENTS_END,
        )
    {
        let expected = crate::services::agents_render::render_block(
            manifest.docs_root.as_str(),
            &declaration.writing_style,
        );
        if region != expected {
            report.fail(format!(
                "FAIL the documentation block in {} does not match the declaration; run 'sdd hooks --apply'",
                crate::commands::hooks::AGENTS
            ));
        }
    }

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
    // Compare per gate rather than over the whole region. A block an older
    // release rendered carries hooks this renderer no longer emits, and the
    // upgrade rewrites those; what this names is the wiring a declaration
    // edit left stale.
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

/// Where the canon's own record keeps the artifacts every skill shares.
///
/// `sdd self-manifest` records them as managed entries at their authored
/// paths, and no consumer landing writes anything there, so their presence
/// is what tells the two layouts apart.
const CANON_ONLY_ROOT: &str = "skill-shared/";

/// The manifest must record every projection the profile declares — an
/// omitted record is an owned file the verifier would silently stop
/// holding. Applies when the instance is at this binary's version, in
/// whichever of the two layouts the record claims.
///
/// The layout is read from what only the canon's self-manifest records: a
/// managed entry under `skill-shared/`. That recognizes the ordinary canon
/// and consumer records. It authenticates nothing, because the record is a
/// file a user can edit, and changing the claimed layout changes which
/// obligations this check enforces. The canon branch requires the managed
/// projection sources, the embedded specs, the canon templates, and the
/// pre-commit block. The consumer branch requires every declared
/// destination and both integration blocks. Neither set contains the
/// other, and the canon branch does not require the whole skill inventory.
///
/// A seed that yields, and that the record does not name, is not owed:
/// the target root held a file of its set, so the landing did not write it.
fn check_projection_against(
    released: &crate::domain::projection::Declaration,
    manifest: &Manifest,
    present_at_root: &[String],
    report: &mut VerifyReport,
) {
    if manifest.canon_version != CanonVersion::current() {
        return;
    }
    let Some(declaration) = released.profile(manifest.profile) else {
        return;
    };
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

    let self_layout = managed
        .iter()
        .any(|destination| destination.starts_with(CANON_ONLY_ROOT));

    let mut expected: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut missing: Vec<String> = Vec::new();
    if self_layout {
        for projection in declaration.managed {
            expected.insert(projection.source.clone());
        }
        for file in crate::embedded::SPECS.files() {
            if let Some(name) = file.path().as_os_str().to_str() {
                expected.insert(format!("_docs/specs/{name}"));
            }
        }
        for template in crate::domain::profile::CANON_TEMPLATES.iter() {
            expected.insert((*template).to_string());
        }
        for destination in &expected {
            if !managed.contains(destination.as_str()) && !adopted.contains(destination.as_str()) {
                missing.push(destination.clone());
            }
        }
    } else {
        for projection in declaration.managed {
            let destination = crate::domain::profile::resolve_destination(
                &projection.destination,
                manifest.docs_root,
            );
            if !managed.contains(destination.as_str()) {
                missing.push(destination.to_string());
            }
        }
        for projection in declaration.adopted {
            let destination = crate::domain::profile::resolve_destination(
                &projection.destination,
                manifest.docs_root,
            );
            if adopted.contains(destination.as_str()) {
                continue;
            }
            let yielded = projection.yield_set().is_some_and(|set| {
                set.names()
                    .any(|name| present_at_root.iter().any(|held| held == name))
            });
            if !yielded {
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
    // own layout carries the pre-commit block, which `just hooks` renders;
    // its root AGENTS.md is release-kit-owned and outside this projection.
    let required: &[&str] = if self_layout {
        &[HOOKS_CONFIG_PATH]
    } else {
        &[HOOKS_CONFIG_PATH, AGENTS_DIGEST_PATH]
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
    let released = &*crate::domain::profile::DECLARATION;
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

    check_projection_against(
        released,
        &manifest,
        &crate::services::installer::present_at_root(target),
        &mut report,
    );
    check_integration(target, &manifest, &mut report)?;
    check_specs(target, &manifest, &mut report)?;
    check_lint_composition(target, &manifest, &mut report);
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

/// Every markdownlint configuration that would replace the delivered one.
///
/// A `.markdownlint.*` file beside the documentation root's configuration
/// replaces its `config`, and any configuration beneath `specs/` or
/// `decisions/` replaces its `overrides` for that directory. Either one
/// silently turns the heading shapes off. The repository root is allowed:
/// a configuration there merges beneath the documentation root's, and it
/// cannot disable the shapes. Only filenames are read, so no configuration
/// grammar is parsed.
///
/// A symbolic link counts by its own name, because the linter reads a
/// configuration through one. The scan skips a linked documentation root
/// or subtree before reading beneath it, and [`linked_lint_scan_roots`]
/// names it instead. The walk follows neither root nor nested links, so a
/// linked directory is not descended and a cycle cannot hold it.
///
/// SATISFIES instance:the-lint-configuration-composes
#[must_use]
pub fn lint_configurations_that_break_composition(
    target: &Utf8Path,
    docs_root: crate::domain::profile::DocsRoot,
) -> Vec<camino::Utf8PathBuf> {
    use crate::domain::markdownlint::{LIBRARY_NAMES, discovery_names};

    let root = Utf8Path::new(docs_root.as_str());
    if reached_through_symlink(target, root) {
        return Vec::new();
    }
    let mut found: Vec<camino::Utf8PathBuf> = LIBRARY_NAMES
        .iter()
        .map(|name| root.join(name))
        .filter(|path| {
            std::fs::symlink_metadata(target.join(path)).is_ok_and(|held| !held.is_dir())
        })
        .collect();
    for subtree in ["specs", "decisions"] {
        let subtree = root.join(subtree);
        if reached_through_symlink(target, &subtree) {
            continue;
        }
        for entry in walkdir::WalkDir::new(target.join(subtree))
            .follow_links(false)
            .follow_root_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| !entry.file_type().is_dir())
        {
            let Some(name) = entry.file_name().to_str() else {
                continue;
            };
            if !discovery_names().any(|known| known == name) {
                continue;
            }
            if let Ok(relative) = entry.path().strip_prefix(target.as_std_path())
                && let Some(relative) = camino::Utf8Path::from_path(relative)
            {
                found.push(relative.to_path_buf());
            }
        }
    }
    found.sort();
    found
}

/// The documentation root or scan roots reached through a symbolic link.
///
/// The linter reads a configuration through the link, and the composition
/// scan does not follow it, so such a directory is a place no check can see
/// into. Naming it keeps the failure with the rule it breaks rather than
/// leaving it to whichever ownership check happens to cover a recorded file
/// beneath it. A linked documentation root is named alone, because both
/// scan roots sit beneath it.
///
/// SATISFIES instance:the-lint-configuration-composes
#[must_use]
pub fn linked_lint_scan_roots(
    target: &Utf8Path,
    docs_root: crate::domain::profile::DocsRoot,
) -> Vec<camino::Utf8PathBuf> {
    let root = Utf8Path::new(docs_root.as_str());
    if reached_through_symlink(target, root) {
        return vec![root.to_path_buf()];
    }
    ["specs", "decisions"]
        .into_iter()
        .map(|subtree| root.join(subtree))
        .filter(|subtree| reached_through_symlink(target, subtree))
        .collect()
}

fn check_lint_composition(target: &Utf8Path, manifest: &Manifest, report: &mut VerifyReport) {
    for path in linked_lint_scan_roots(target, manifest.docs_root) {
        report.fail(format!(
            "FAIL {path} is reached through a symlink, so no check sees the lint configuration beneath it; replace the link with the directory (instance:the-lint-configuration-composes)"
        ));
    }
    for path in lint_configurations_that_break_composition(target, manifest.docs_root) {
        report.fail(format!(
            "FAIL {path} replaces the delivered lint configuration and turns the heading shapes off; fold it into the root .markdownlint-cli2.jsonc (instance:the-lint-configuration-composes)"
        ));
    }
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
    if path == HOOKS_CONFIG_PATH {
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
                if path == HOOKS_CONFIG_PATH
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

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;
    use crate::candidate::{Evidence, Input, project};
    use crate::domain::ownership::{ManagedEntry, Sha256};
    use crate::domain::profile::{DECLARATION, ProfileId};

    fn consumer(profile: ProfileId) -> Manifest {
        project(&Input {
            profile,
            version: CanonVersion::current(),
            installed_at: "2026-01-01T00:00:00Z".to_string(),
            docs_scratch: None,
            reserve: Vec::new(),
            writing_style: None,
            evidence: Evidence::default(),
        })
        .unwrap()
        .manifest
    }

    fn failures(manifest: &Manifest) -> Vec<String> {
        let mut report = VerifyReport::default();
        check_projection_against(&DECLARATION, manifest, &[], &mut report);
        report.lines
    }

    fn managed(path: &str) -> ManagedEntry {
        ManagedEntry {
            source: path.into(),
            destination: path.into(),
            sha256: Sha256::of(path.as_bytes()),
        }
    }

    /// VERIFIES instance:the-lint-configuration-composes
    ///
    /// Neither a linked scan root nor its documentation-root ancestor may
    /// expose external configurations to the composition scan.
    #[test]
    fn the_lint_scan_does_not_follow_its_roots_or_their_ancestors() {
        use crate::domain::profile::DocsRoot;

        for docs_root in [DocsRoot::Docs, DocsRoot::UnderscoreDocs] {
            for boundary in ["", "specs", "decisions"] {
                let target = tempfile::tempdir().unwrap();
                let target = Utf8Path::from_path(target.path()).unwrap();
                let outside = tempfile::tempdir().unwrap();
                // A linked documentation root could expose both the direct
                // library configuration and either recursive scan.
                for relative in [
                    ".markdownlint.json",
                    "specs/.markdownlint-cli2.jsonc",
                    "decisions/.markdownlint.yaml",
                ] {
                    let path = outside.path().join(relative);
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(path, "{}\n").unwrap();
                }
                let link = if boundary.is_empty() {
                    target.join(docs_root.as_str())
                } else {
                    target.join(docs_root.as_str()).join(boundary)
                };
                std::fs::create_dir_all(link.parent().unwrap()).unwrap();
                std::os::unix::fs::symlink(outside.path(), &link).unwrap();

                assert!(
                    lint_configurations_that_break_composition(target, docs_root).is_empty(),
                    "{docs_root}/{boundary}: the scan followed a directory link"
                );
                let linked = if boundary.is_empty() {
                    camino::Utf8PathBuf::from(docs_root.as_str())
                } else {
                    Utf8Path::new(docs_root.as_str()).join(boundary)
                };
                assert_eq!(linked_lint_scan_roots(target, docs_root), [linked]);
            }
        }
    }

    /// VERIFIES instance:the-lint-configuration-composes
    #[test]
    fn the_lint_scan_skips_nested_directory_links_and_cycles() {
        use crate::domain::profile::DocsRoot;

        for docs_root in [DocsRoot::Docs, DocsRoot::UnderscoreDocs] {
            let target = tempfile::tempdir().unwrap();
            let target = Utf8Path::from_path(target.path()).unwrap();
            let specs = target.join(docs_root.as_str()).join("specs");
            std::fs::create_dir_all(&specs).unwrap();
            let outside = tempfile::tempdir().unwrap();
            std::fs::write(outside.path().join(".markdownlint-cli2.jsonc"), "{}\n").unwrap();
            std::os::unix::fs::symlink(outside.path(), specs.join("linked")).unwrap();
            std::os::unix::fs::symlink(&specs, specs.join("cycle")).unwrap();
            // Skipping linked directories must still count a configuration
            // link by its own filename, even where its target is absent.
            let config = specs.join(".markdownlint.yaml");
            std::os::unix::fs::symlink(outside.path().join("absent.yaml"), &config).unwrap();

            assert_eq!(
                lint_configurations_that_break_composition(target, docs_root),
                [config.strip_prefix(target).unwrap()]
            );
            // A link nested inside a scan root is not a linked scan root.
            assert!(linked_lint_scan_roots(target, docs_root).is_empty());
        }
    }

    /// A knowledge-base record whose managed destinations equal their
    /// sources satisfied the old discriminator, which then demanded the
    /// canon's specs and templates and dropped the `AGENTS.md` block.
    #[test]
    fn a_consumer_whose_managed_destination_equals_its_source_is_still_a_consumer() {
        let mut manifest = consumer(ProfileId::KnowledgeBase);
        for projection in &DECLARATION.managed {
            if !manifest
                .managed_files
                .iter()
                .any(|entry| entry.destination == projection.source.as_str())
            {
                manifest.managed_files.push(managed(&projection.source));
            }
        }
        assert!(failures(&manifest).is_empty(), "{:?}", failures(&manifest));

        let mut without_block = manifest.clone();
        without_block
            .integration_blocks
            .retain(|block| block.path != AGENTS_DIGEST_PATH);
        assert!(
            failures(&without_block)
                .iter()
                .any(|line| line.contains("no integration block for AGENTS.md")),
            "{:?}",
            failures(&without_block)
        );

        let mut without_seed = manifest;
        let dropped = without_seed.adopted_files.remove(1).destination;
        assert!(
            failures(&without_seed)
                .iter()
                .any(|line| line.ends_with(dropped.as_str())),
            "{:?}",
            failures(&without_seed)
        );
    }

    #[test]
    fn the_canon_record_is_read_as_the_canon() {
        let text =
            std::fs::read_to_string(Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join(MANIFEST_PATH))
                .unwrap();
        let mut manifest = Manifest::parse(&text).unwrap();
        manifest.canon_version = CanonVersion::current();
        assert!(failures(&manifest).is_empty(), "{:?}", failures(&manifest));
    }

    #[test]
    fn a_consumer_omitting_a_declared_seed_fails() {
        let mut manifest = consumer(ProfileId::Codebase);
        let dropped = manifest.adopted_files.pop().unwrap().destination;
        let lines = failures(&manifest);
        assert!(
            lines.contains(&format!(
                "FAIL manifest omits a declared projection: {dropped}"
            )),
            "{lines:?}"
        );
    }

    /// The marker recognizes a layout and authenticates nothing: a record
    /// that claims the canon layout is held to the canon's obligations, and
    /// those do not include the `AGENTS.md` block.
    #[test]
    fn a_claimed_canon_layout_changes_which_obligations_are_held() {
        let mut manifest = consumer(ProfileId::KnowledgeBase);
        manifest
            .integration_blocks
            .retain(|block| block.path != AGENTS_DIGEST_PATH);
        manifest
            .managed_files
            .push(managed("skill-shared/plan-gate.md"));
        let lines = failures(&manifest);
        assert!(
            lines
                .iter()
                .any(|line| line
                    .contains("omits a declared projection: _docs/specs/SPEC-release.md")),
            "the canon branch owes the canon-only specs: {lines:?}"
        );

        for projection in &DECLARATION.managed {
            manifest.managed_files.push(managed(&projection.source));
        }
        for file in crate::embedded::SPECS.files() {
            let name = file.path().to_str().unwrap();
            manifest
                .managed_files
                .push(managed(&format!("_docs/specs/{name}")));
        }
        for template in crate::domain::profile::CANON_TEMPLATES.iter() {
            manifest.managed_files.push(managed(template));
        }
        let lines = failures(&manifest);
        assert!(lines.is_empty(), "{lines:?}");
    }

    #[test]
    fn a_yielded_seed_the_record_does_not_name_is_not_owed() {
        let Some(yielding) = DECLARATION
            .adopted
            .iter()
            .find(|projection| projection.yield_set().is_some())
        else {
            return;
        };
        let mut manifest = consumer(ProfileId::Codebase);
        let destination =
            crate::domain::profile::resolve_destination(&yielding.destination, manifest.docs_root);
        manifest
            .adopted_files
            .retain(|entry| entry.destination != destination);
        assert!(!failures(&manifest).is_empty());

        let mut report = VerifyReport::default();
        let cause = yielding
            .yield_set()
            .unwrap()
            .names()
            .last()
            .unwrap()
            .to_string();
        check_projection_against(&DECLARATION, &manifest, &[cause], &mut report);
        assert!(report.lines.is_empty(), "{:?}", report.lines);
    }
}
