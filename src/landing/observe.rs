//! What the planner is told, and the one place that reads it.
//!
//! The planner is pure, so everything it needs arrives as a value: the
//! repository, the installation, the host, and the corpus. This module is
//! the impure half that produces those values, and it is the only part of
//! the plan tree that touches a disk.
//!
//! Reading is bounded. An observation that cannot be made is recorded as
//! not observed, with the reason, so the readiness policy can weigh it.
//! An observation whose failure has no bound — an unreadable target — is a
//! command error rather than a plan built on a guess.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::domain::instance_config::CONFIG_PATH;
use crate::domain::manifest::{INSTANCE_DIR, MANIFEST_PATH};
use crate::domain::ownership::Sha256;
use crate::domain::paths::UserEnv;
use crate::domain::profile::{DocsRoot, ProfileId};
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::landing::finding::{is_ordinal_name, is_record_shaped, is_spec_shaped};
use crate::landing::path::TargetPath;

/// The documentation roots a corpus conventionally lives under.
const DOC_ROOTS: &[&str] = &["docs", "_docs", "doc", "documentation"];

/// Root-level filename stems that are metadata rather than a corpus.
const ROOT_METADATA: &[&str] = &[
    "readme",
    "license",
    "licence",
    "contributing",
    "changelog",
    "agents",
    "claude",
    "code_of_conduct",
];

/// Directory names no observation walks into.
const SKIPPED: &[&str] = &[".git", ".jj", "target", "node_modules", INSTANCE_DIR];

/// Extensions a durable document conventionally carries.
const DOC_EXTENSIONS: &[&str] = &["md", "markdown", "adoc", "rst", "org"];

/// The target itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repository {
    /// Where it is.
    pub root: Utf8PathBuf,
    /// Whether version control is there to restore a retirement.
    pub version_controlled: bool,
    /// Whether it holds anything at all.
    pub empty: bool,
}

/// One file the instance records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedFile {
    /// Where, relative to the target.
    pub path: TargetPath,
    /// What the record says the tool wrote there.
    pub recorded: Sha256,
    /// What the baseline was, for an adopted file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<Sha256>,
    /// What the target holds now, or nothing where the file is gone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held: Option<Sha256>,
}

/// What is installed at the target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Installation {
    /// The release the record names.
    pub canon_version: CanonVersion,
    /// The profile the record names.
    pub profile: ProfileId,
    /// The documentation root the record names.
    pub docs_root: DocsRoot,
    /// The record's own digest.
    pub record_sha256: Sha256,
    /// The project's declaration digest, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declaration_sha256: Option<Sha256>,
    /// Every managed file, as recorded and as found.
    pub managed: Vec<RecordedFile>,
    /// Every adopted file, as recorded and as found.
    pub adopted: Vec<RecordedFile>,
    /// Every marked region the canon owns, as recorded and as found.
    pub blocks: Vec<RecordedBlock>,
}

/// One marked region a file the project owns carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedBlock {
    /// The host file, relative to the target.
    pub path: TargetPath,
    /// The region's digest as the record has it.
    pub recorded: Sha256,
    /// The region's digest now, or nothing where the host or the markers
    /// are gone.
    pub held: Option<Sha256>,
}

impl Installation {
    /// Whether any recorded managed file has moved or gone.
    #[must_use]
    pub fn drifted(&self) -> bool {
        self.managed
            .iter()
            .any(|file| file.held.as_ref() != Some(&file.recorded))
            || self
                .blocks
                .iter()
                .any(|block| block.held.as_ref() != Some(&block.recorded))
    }
}

/// The host the command ran on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Host {
    /// Whether this run may reach the network.
    pub offline: bool,
    /// Where this tool keeps what it can fetch again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_root: Option<Utf8PathBuf>,
}

/// What the corpus looks like, as counts and paths a detector reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Corpus {
    /// Documentation roots at the top level that hold anything.
    pub populated_doc_roots: Vec<String>,
    /// Every durable document, relative to the target, sorted.
    pub documents: Vec<TargetPath>,
    /// Documents shaped like a specification of this convention.
    pub spec_shaped: Vec<TargetPath>,
    /// Specification-shaped documents that define no rule identifier.
    pub spec_without_rule_id: Vec<TargetPath>,
    /// Documents named by their position rather than their subject.
    pub ordinal_named: Vec<TargetPath>,
    /// Decision records outside a decisions directory.
    pub records_outside_decisions: Vec<TargetPath>,
    /// Whether a specifications directory exists under a documentation root.
    pub has_specs_directory: bool,
}

impl Corpus {
    /// Whether the target documents itself already.
    ///
    /// Documents, not directories. A documentation root holding empty
    /// directories is a layout somebody started and not a corpus somebody
    /// wrote, and refusing to land beside it would refuse a repository
    /// that has written nothing.
    #[must_use]
    pub const fn settled(&self) -> bool {
        !self.documents.is_empty()
    }
}

/// Everything the planner is told about one target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// The target itself.
    pub repository: Repository,
    /// What is installed, where anything is.
    pub installation: Option<Installation>,
    /// Why no installation was read, where metadata exists and is broken.
    pub invalid: Option<String>,
    /// The host.
    pub host: Host,
    /// The corpus.
    pub corpus: Corpus,
}

/// Read one target.
///
/// # Errors
///
/// [`AppError::Usage`] when the target is not a directory, and
/// [`AppError::Io`] when the walk cannot complete. Both are failures whose
/// scope cannot be bounded: a plan built on half a reading would describe
/// a repository nobody looked at.
pub fn observe(target: &Utf8Path) -> Result<Observation, AppError> {
    match std::fs::metadata(target) {
        Ok(metadata) if !metadata.is_dir() => {
            return Err(AppError::Usage(format!(
                "target is not a directory: {target}"
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(AppError::Usage(format!("unresolved target: {target}")));
        }
        Err(error) => return Err(AppError::Io(error)),
    }

    let env = UserEnv::from_process();
    let host = Host {
        offline: crate::domain::paths::variable(crate::domain::paths::OFFLINE_VAR).is_some(),
        cache_root: env.user_paths().map(|paths| paths.cache_root.path),
    };

    let (installation, invalid) = read_installation(target);
    let corpus = read_corpus(target)?;
    let repository = Repository {
        root: target.to_owned(),
        version_controlled: target.join(".git").exists(),
        empty: is_empty(target)?,
    };
    Ok(Observation {
        repository,
        installation,
        invalid,
        host,
        corpus,
    })
}

/// Whether a target holds anything but version control.
fn is_empty(target: &Utf8Path) -> Result<bool, AppError> {
    for entry in std::fs::read_dir(target)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name != ".git" && name != ".jj" {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Read the instance record, or say why it could not be trusted.
///
/// Every failure here is one of the two answers rather than an error: a
/// record that is absent and one that is broken are both things the plan
/// reports, and neither stops the observation.
fn read_installation(target: &Utf8Path) -> (Option<Installation>, Option<String>) {
    let path = target.join(MANIFEST_PATH);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        // Absent is absent. Anything else — unreadable, not UTF-8, a
        // permission refusal — is a record that exists and cannot be
        // trusted, and reporting it as absence would land seeds over it.
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return (None, None),
        Err(source) => {
            return (
                None,
                Some(format!("{path} exists and cannot be read: {source}")),
            );
        }
    };
    // The record's schema is its own axis. A record an older release wrote
    // is a record this engine reads well enough to classify: what it needs
    // is the version, the profile, the root, and the two file lists, and
    // every schema this tool has written carries those under those names.
    // Only a record it cannot read at all is invalid, because absence and
    // breakage are different findings.
    let Some(manifest) = read_any_schema(&text) else {
        return (
            None,
            Some(format!(
                "{MANIFEST_PATH} is not a record this engine can read"
            )),
        );
    };
    let held = |destination: &str| -> Option<Sha256> {
        std::fs::read(target.join(destination))
            .ok()
            .map(|bytes| Sha256::of(&bytes))
    };
    let mut managed = Vec::new();
    for (destination, recorded) in &manifest.managed_files {
        let Ok(path) = TargetPath::new(destination) else {
            return (
                None,
                Some(format!(
                    "{MANIFEST_PATH} records the destination {destination}, which no operation may name"
                )),
            );
        };
        managed.push(RecordedFile {
            held: held(destination),
            path,
            recorded: recorded.clone(),
            baseline: None,
        });
    }
    let mut adopted = Vec::new();
    for (destination, recorded, baseline) in &manifest.adopted_files {
        let Ok(path) = TargetPath::new(destination) else {
            return (
                None,
                Some(format!(
                    "{MANIFEST_PATH} records the destination {destination}, which no operation may name"
                )),
            );
        };
        adopted.push(RecordedFile {
            held: held(destination),
            path,
            recorded: recorded.clone(),
            baseline: Some(baseline.clone()),
        });
    }
    // A marked region is managed too. The host file is the project's and
    // its other bytes are none of this tool's business, so the region is
    // compared by its own hash: an edit inside the markers is a conflict
    // the next landing would overwrite, and an edit outside them is not.
    let mut blocks = Vec::new();
    for (host, recorded) in &manifest.integration_blocks {
        let Ok(path) = TargetPath::new(host) else {
            return (
                None,
                Some(format!(
                    "{MANIFEST_PATH} records the block host {host}, which no operation may name"
                )),
            );
        };
        let (begin, end) = markers_for(host);
        let held = std::fs::read_to_string(target.join(host))
            .ok()
            .and_then(|text| crate::domain::marker::block_hash_with(&text, begin, end));
        blocks.push(RecordedBlock {
            path,
            recorded: recorded.clone(),
            held,
        });
    }

    let declaration_sha256 = std::fs::read(target.join(CONFIG_PATH))
        .ok()
        .map(|bytes| Sha256::of(&bytes));
    (
        Some(Installation {
            canon_version: manifest.canon_version,
            profile: manifest.profile,
            docs_root: manifest.docs_root,
            record_sha256: Sha256::of(text.as_bytes()),
            declaration_sha256,
            managed,
            adopted,
            blocks,
        }),
        None,
    )
}

/// The markers one integration host carries.
fn markers_for(path: &str) -> (&'static str, &'static str) {
    use crate::domain::marker::{AGENTS_BEGIN, AGENTS_END, BEGIN, END};
    if path == crate::domain::paths::HOOKS_CONFIG_PATH {
        (BEGIN, END)
    } else {
        (AGENTS_BEGIN, AGENTS_END)
    }
}

/// One record's facts, whichever schema wrote it.
struct AnyRecord {
    canon_version: CanonVersion,
    profile: ProfileId,
    docs_root: DocsRoot,
    managed_files: Vec<(String, Sha256)>,
    adopted_files: Vec<(String, Sha256, Sha256)>,
    integration_blocks: Vec<(String, Sha256)>,
}

fn read_any_schema(text: &str) -> Option<AnyRecord> {
    let held: serde_json::Value = serde_json::from_str(text).ok()?;
    let files = |key: &str| -> Vec<serde_json::Value> {
        held.get(key)
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default()
    };
    let digest = |entry: &serde_json::Value, key: &str| -> Option<Sha256> {
        entry.get(key)?.as_str()?.parse().ok()
    };
    let destination = |entry: &serde_json::Value| -> Option<String> {
        Some(entry.get("destination")?.as_str()?.to_string())
    };
    Some(AnyRecord {
        canon_version: held.get("canon_version")?.as_str()?.parse().ok()?,
        profile: serde_json::from_value(held.get("profile")?.clone()).ok()?,
        docs_root: serde_json::from_value(held.get("docs_root")?.clone()).ok()?,
        managed_files: files("managed_files")
            .iter()
            .filter_map(|entry| Some((destination(entry)?, digest(entry, "sha256")?)))
            .collect(),
        // All or nothing. An entry this engine cannot read is a record it
        // cannot vouch for, and dropping it would report a managed region
        // as absent, which is what lets the next landing overwrite it.
        integration_blocks: files("integration_blocks")
            .iter()
            .map(|entry| {
                Some((
                    entry.get("path")?.as_str()?.to_string(),
                    digest(entry, "marker_hash")?,
                ))
            })
            .collect::<Option<Vec<_>>>()?,
        adopted_files: files("adopted_files")
            .iter()
            .filter_map(|entry| {
                Some((
                    destination(entry)?,
                    digest(entry, "sha256")?,
                    digest(entry, "baseline_sha256")?,
                ))
            })
            .collect(),
    })
}

/// Walk the corpus, reading only what a detector can prove.
fn read_corpus(target: &Utf8Path) -> Result<Corpus, AppError> {
    let mut corpus = Corpus::default();
    for root in DOC_ROOTS {
        let path = target.join(root);
        if path.is_dir() && std::fs::read_dir(&path)?.next().is_some() {
            corpus.populated_doc_roots.push((*root).to_string());
        }
        if path.join("specs").is_dir() {
            corpus.has_specs_directory = true;
        }
    }

    for entry in walkdir::WalkDir::new(target)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !entry.file_type().is_dir()
                || !SKIPPED.contains(&entry.file_name().to_string_lossy().as_ref())
        })
    {
        let entry = entry.map_err(|source| AppError::Io(std::io::Error::from(source)))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path().to_path_buf()) else {
            continue;
        };
        let Ok(relative) = path.strip_prefix(target) else {
            continue;
        };
        let Ok(held) = TargetPath::new(relative.as_str()) else {
            continue;
        };
        let name = relative.file_name().unwrap_or_default();
        let extension = relative.extension().unwrap_or_default();
        if !DOC_EXTENSIONS.contains(&extension) {
            continue;
        }
        // Root metadata is what every repository carries, whether or not
        // it documents itself.
        let stem = relative.file_stem().unwrap_or_default().to_lowercase();
        if relative
            .parent()
            .is_none_or(|parent| parent.as_str().is_empty())
            && ROOT_METADATA.contains(&stem.as_str())
        {
            continue;
        }
        corpus.documents.push(held.clone());
        if is_spec_shaped(name) {
            corpus.spec_shaped.push(held.clone());
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            if crate::embedded::rule_ids_in(&text).next().is_none() {
                corpus.spec_without_rule_id.push(held.clone());
            }
        }
        if is_ordinal_name(name) {
            corpus.ordinal_named.push(held.clone());
        }
        if is_record_shaped(name)
            && relative
                .parent()
                .is_none_or(|parent| parent.file_name() != Some("decisions"))
        {
            corpus.records_outside_decisions.push(held);
        }
    }
    corpus.documents.sort();
    corpus.spec_shaped.sort();
    corpus.spec_without_rule_id.sort();
    corpus.ordinal_named.sort();
    corpus.records_outside_decisions.sort();
    Ok(corpus)
}

/// What the recorded destinations hold, keyed by path.
#[must_use]
pub fn held_by_path(installation: Option<&Installation>) -> BTreeMap<String, Sha256> {
    let mut held = BTreeMap::new();
    let Some(installation) = installation else {
        return held;
    };
    for file in installation.managed.iter().chain(&installation.adopted) {
        if let Some(digest) = file.held.clone() {
            held.insert(file.path.as_str().to_string(), digest);
        }
    }
    held
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn scratch() -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from(dir.path().to_str().unwrap());
        std::fs::create_dir(root.join(".git")).unwrap();
        (dir, root)
    }

    #[test]
    fn an_empty_target_reads_as_empty_and_unsettled() {
        let (_dir, root) = scratch();
        let held = observe(&root).unwrap();
        assert!(held.repository.empty);
        assert!(held.repository.version_controlled);
        assert!(held.installation.is_none());
        assert_eq!(held.invalid, None);
        assert!(!held.corpus.settled());
    }

    #[test]
    fn root_metadata_is_not_a_corpus() {
        let (_dir, root) = scratch();
        std::fs::write(root.join("README.md"), "# x\n").unwrap();
        std::fs::write(root.join("CHANGELOG.md"), "# x\n").unwrap();
        let held = observe(&root).unwrap();
        assert!(!held.repository.empty);
        assert!(!held.corpus.settled(), "{:?}", held.corpus.documents);
    }

    #[test]
    fn a_populated_documentation_root_is_a_settled_corpus() {
        let (_dir, root) = scratch();
        crate::adapters::fs::write_file(&root.join("docs/guide.md"), b"# guide\n").unwrap();
        let held = observe(&root).unwrap();
        assert_eq!(held.corpus.populated_doc_roots, ["docs"]);
        assert!(held.corpus.settled());
        assert_eq!(held.corpus.documents.len(), 1);
    }

    #[test]
    fn a_documentation_root_of_empty_directories_is_not_a_corpus() {
        let (_dir, root) = scratch();
        std::fs::create_dir_all(root.join("_docs/specs")).unwrap();
        let held = observe(&root).unwrap();
        assert_eq!(held.corpus.populated_doc_roots, ["_docs"]);
        assert!(!held.corpus.settled(), "a layout is not a corpus");
    }

    #[test]
    fn the_corpus_reads_only_what_a_detector_can_prove() {
        let (_dir, root) = scratch();
        crate::adapters::fs::write_file(&root.join("docs/specs/SPEC-x.md"), b"# x\n").unwrap();
        crate::adapters::fs::write_file(&root.join("docs/specs/SPEC-y.md"), b"### `a-b:c-d` - t\n")
            .unwrap();
        crate::adapters::fs::write_file(&root.join("docs/01-intro.md"), b"# x\n").unwrap();
        crate::adapters::fs::write_file(&root.join("docs/ADR-a-choice.md"), b"# x\n").unwrap();
        crate::adapters::fs::write_file(&root.join("docs/decisions/ADR-b-choice.md"), b"# x\n")
            .unwrap();
        crate::adapters::fs::write_file(&root.join("docs/specs/notes.md"), b"# x\n").unwrap();

        let corpus = observe(&root).unwrap().corpus;
        assert!(corpus.has_specs_directory);
        assert_eq!(corpus.spec_shaped.len(), 2);
        assert_eq!(
            corpus
                .spec_without_rule_id
                .iter()
                .map(TargetPath::as_str)
                .collect::<Vec<_>>(),
            ["docs/specs/SPEC-x.md"]
        );
        assert_eq!(
            corpus
                .ordinal_named
                .iter()
                .map(TargetPath::as_str)
                .collect::<Vec<_>>(),
            ["docs/01-intro.md"]
        );
        assert_eq!(
            corpus
                .records_outside_decisions
                .iter()
                .map(TargetPath::as_str)
                .collect::<Vec<_>>(),
            ["docs/ADR-a-choice.md"]
        );
    }

    #[test]
    fn a_record_that_does_not_parse_is_invalid_and_never_absent() {
        let (_dir, root) = scratch();
        crate::adapters::fs::write_file(&root.join(MANIFEST_PATH), b"{not json").unwrap();
        let held = observe(&root).unwrap();
        assert!(held.installation.is_none());
        assert!(held.invalid.is_some(), "a broken record read as absent");
    }

    #[test]
    fn an_unreadable_target_is_a_command_error_and_not_a_plan() {
        let (_dir, root) = scratch();
        std::fs::write(root.join("a-file"), b"x").unwrap();
        let error = observe(&root.join("a-file")).unwrap_err();
        assert_eq!(error.exit_code(), 64);
        assert!(observe(&root.join("absent")).is_err());
    }
}
