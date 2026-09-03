//! Classify a target repository before anything lands.
//!
//! The assessment is read-only evidence plus one classification computed
//! from it by an explicit rule: `greenfield` when the project has written
//! no durable documentation beyond root metadata, `brownfield` when a
//! documentation root or a methodology marker shows a settled corpus, and
//! `needs-decision` when documents sit outside any recognized home. The
//! rule lives here so a routing skill reads a verdict it can cite instead
//! of judging "little docs" by feel.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::domain::profile::{ProfileId, resolve_destination};
use crate::error::AppError;
use crate::gates::PRUNED_DIRS;
use crate::services::status::{StatusReport, status};

/// The directory names a documentation corpus conventionally lives under.
const DOC_ROOTS: &[&str] = &["docs", "_docs", "doc", "documentation"];

/// Root-level files and directories that mark an existing documentation
/// methodology, whatever it is.
const ROOT_MARKERS: &[&str] = &[
    "specs",
    "decisions",
    "adr",
    "adrs",
    "mkdocs.yml",
    "docusaurus.config.js",
    "docusaurus.config.ts",
    "conf.py",
];

/// Extensions a durable document conventionally carries.
const DOC_EXTENSIONS: &[&str] = &["md", "markdown", "adoc", "rst", "org"];

/// Root-level filename stems that are metadata, not a documentation corpus.
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

/// What the target is, for routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    /// No durable documentation beyond root metadata: land an instance.
    Greenfield,
    /// A settled corpus or a methodology marker: migrate, not just land.
    Brownfield,
    /// Documents outside any recognized home: the operator decides.
    NeedsDecision,
}

impl Classification {
    /// The kebab-case verdict word, as the JSON serializes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Greenfield => "greenfield",
            Self::Brownfield => "brownfield",
            Self::NeedsDecision => "needs-decision",
        }
    }
}

/// The document inventory the classification is computed from.
#[derive(Debug, Serialize)]
pub struct Documents {
    /// How many document files the walk found.
    pub count: usize,
    /// Every document path, relative to the target, sorted.
    pub paths: Vec<Utf8PathBuf>,
}

/// The whole assessment: evidence first, one verdict from it.
#[derive(Debug, Serialize)]
pub struct AssessReport {
    /// The shape version of this document.
    pub schema: &'static str,
    /// The assessed repository.
    pub target: Utf8PathBuf,
    /// The verdict the evidence below produces.
    pub classification: Classification,
    /// The instance report, verbatim from `sdd status`.
    pub instance: StatusReport,
    /// Documentation roots found at the target's top level.
    pub doc_roots: Vec<String>,
    /// The documentation roots holding any entry at all — the evidence the
    /// brownfield verdict reads, whatever format or link shape the entries
    /// have.
    pub populated_doc_roots: Vec<String>,
    /// The document inventory.
    pub documents: Documents,
    /// Methodology markers found, as target-relative paths.
    pub methodology_markers: Vec<String>,
    /// Per profile, the install destinations that already exist.
    pub collisions: BTreeMap<String, Vec<String>>,
    /// Whether the target carries a `.draft/` workshop.
    pub draft_present: bool,
}

/// Assess `target`, reading and never writing.
///
/// # Errors
///
/// [`AppError::Usage`] when the target exists and is not a directory,
/// [`AppError::ManifestInvalid`] when an instance manifest exists but
/// cannot be trusted — a broken instance must not silently classify — and
/// [`AppError::Io`] for metadata failures and walk errors.
pub fn assess(target: &Utf8Path) -> Result<AssessReport, AppError> {
    // A file target would walk as its own single entry and read as an
    // empty repository; refuse it instead, on proven metadata only. An
    // absent path falls through to the walk, whose I/O error names it,
    // and a metadata failure is an I/O result, never a usage mistake.
    match std::fs::metadata(target) {
        Ok(metadata) if !metadata.is_dir() => {
            return Err(AppError::Usage(format!(
                "target is not a directory: {target}"
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(AppError::Io(error)),
    }
    let instance = status(target)?;
    // `is_dir` follows a link and reads false through a broken one, so a
    // symlink is recognized on its own: a root the project points
    // elsewhere is evidence whether or not the destination resolves.
    let doc_roots: Vec<String> = DOC_ROOTS
        .iter()
        .filter(|root| {
            let root = target.join(root);
            root.is_dir() || root.is_symlink()
        })
        .map(|root| (*root).to_string())
        .collect();
    let walked = walk(target)?;
    let paths = walked.documents;
    let methodology_markers = markers(target, &doc_roots)?;
    let collisions = collisions(target)?;
    let draft_present = target.join(".draft").is_dir();

    // A populated documentation root is a corpus whatever format it uses:
    // a tree of .adoc or .rst files under docs/ is exactly as settled as
    // one of markdown, and a verdict that missed it would land seeds
    // beside it. A root that is itself a symlink is evidence the same way,
    // without being followed: the walk does not traverse links, so the
    // link's presence is what there is to read.
    let populated_doc_roots: Vec<String> = doc_roots
        .iter()
        .filter(|root| walked.populated_roots.contains(*root) || target.join(root).is_symlink())
        .cloned()
        .collect();
    let beyond_metadata = paths.iter().any(|path| !is_root_metadata(path));
    let classification = if !populated_doc_roots.is_empty() || !methodology_markers.is_empty() {
        Classification::Brownfield
    } else if beyond_metadata {
        Classification::NeedsDecision
    } else {
        Classification::Greenfield
    };

    Ok(AssessReport {
        schema: "sdd.assess/1",
        target: target.to_owned(),
        classification,
        instance,
        doc_roots,
        populated_doc_roots,
        documents: Documents {
            count: paths.len(),
            paths,
        },
        methodology_markers,
        collisions,
        draft_present,
    })
}

/// What one walk over the target observed.
struct Walked {
    /// Every document file, relative to the target, sorted.
    documents: Vec<Utf8PathBuf>,
    /// The top-level directory names holding any entry at all.
    populated_roots: Vec<String>,
}

/// Walk `target` once, with the pruned directories, the workshop, and the
/// instance's own tree skipped. Symlinks are evidence and are not
/// followed: a link named like a document still marks its directory as
/// populated.
fn walk(target: &Utf8Path) -> Result<Walked, AppError> {
    let mut documents = Vec::new();
    let mut populated_roots = Vec::new();
    let walker = walkdir::WalkDir::new(target).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !(e.depth() > 0
            && e.file_type().is_dir()
            && (PRUNED_DIRS.contains(&name.as_ref())
                || name == ".draft"
                || name == ".spec-driven-docs"))
    });
    for entry in walker {
        let entry = entry.map_err(|source| AppError::Io(std::io::Error::from(source)))?;
        if entry.file_type().is_dir() {
            continue;
        }
        let Some(path) = entry.path().to_str() else {
            continue;
        };
        let relative = Utf8Path::new(path)
            .strip_prefix(target)
            .unwrap_or_else(|_| Utf8Path::new(path));
        if let Some(root) = relative.components().next() {
            let root = root.as_str().to_string();
            if relative.components().nth(1).is_some() && !populated_roots.contains(&root) {
                populated_roots.push(root);
            }
        }
        if entry.file_type().is_file()
            && relative.extension().is_some_and(|extension| {
                DOC_EXTENSIONS
                    .iter()
                    .any(|known| extension.eq_ignore_ascii_case(known))
            })
        {
            documents.push(relative.to_owned());
        }
    }
    documents.sort();
    Ok(Walked {
        documents,
        populated_roots,
    })
}

/// Whether `path` is root-level project metadata rather than a corpus.
fn is_root_metadata(path: &Utf8Path) -> bool {
    if path
        .parent()
        .is_some_and(|parent| !parent.as_str().is_empty())
    {
        return false;
    }
    let Some(stem) = path.file_stem() else {
        return false;
    };
    let stem = stem.to_ascii_lowercase();
    // Exact stems only: `README.architecture.md` is a document wearing a
    // metadata prefix, and an allowlist that took every dotted suffix
    // would classify it away.
    ROOT_METADATA.iter().any(|metadata| stem == *metadata)
}

/// Whether an entry sits at `path`, broken symlinks included.
///
/// `symlink_metadata` rather than `exists`: a broken symlink named
/// `mkdocs.yml` is still the project saying it documents itself there.
/// Absence is the only failure that reads as absence; any other metadata
/// error propagates, because evidence that cannot be read must never
/// count as evidence that is not there.
fn entry_present(path: &Utf8Path) -> Result<bool, AppError> {
    match path.symlink_metadata() {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(AppError::Io(error)),
    }
}

/// The methodology markers present: root markers, and the conventional
/// zone directories under each detected documentation root.
fn markers(target: &Utf8Path, doc_roots: &[String]) -> Result<Vec<String>, AppError> {
    let mut found = Vec::new();
    for marker in ROOT_MARKERS {
        if entry_present(&target.join(marker))? {
            found.push((*marker).to_string());
        }
    }
    for root in doc_roots {
        for zone in ["specs", "decisions", "adr", "adrs", "conf.py"] {
            let candidate = format!("{root}/{zone}");
            if entry_present(&target.join(&candidate))? {
                found.push(candidate);
            }
        }
    }
    Ok(found)
}

/// Per profile, the install destinations already present at the target.
fn collisions(target: &Utf8Path) -> Result<BTreeMap<String, Vec<String>>, AppError> {
    let mut collisions = BTreeMap::new();
    for id in [ProfileId::Codebase, ProfileId::KnowledgeBase] {
        let profile = id.profile();
        let mut existing = Vec::new();
        for projection in profile.managed.iter().chain(profile.adopted) {
            let destination = resolve_destination(projection.destination, profile.docs_root);
            if entry_present(&target.join(&destination))? {
                existing.push(destination.to_string());
            }
        }
        collisions.insert(id.as_str().to_string(), existing);
    }
    Ok(collisions)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn utf8(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap())
    }

    fn write(root: &Utf8Path, relative: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "content\n").unwrap();
    }

    #[test]
    fn root_metadata_is_recognized_case_insensitively_and_only_at_root() {
        assert!(is_root_metadata(Utf8Path::new("README.md")));
        assert!(is_root_metadata(Utf8Path::new("readme.md")));
        assert!(is_root_metadata(Utf8Path::new("code_of_conduct.md")));
        assert!(is_root_metadata(Utf8Path::new("CONTRIBUTING.md")));
        assert!(is_root_metadata(Utf8Path::new("AGENTS.md")));
        assert!(!is_root_metadata(Utf8Path::new("notes.md")));
        assert!(!is_root_metadata(Utf8Path::new("sub/README.md")));
    }

    #[test]
    fn an_empty_target_classifies_greenfield() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        write(&root, "README.md");
        write(&root, "CHANGELOG.md");
        let report = assess(&root).unwrap();
        assert_eq!(report.classification, Classification::Greenfield);
        assert_eq!(report.documents.count, 2);
    }

    /// A populated documentation root is a corpus whatever format it uses.
    #[test]
    fn a_non_markdown_corpus_under_a_doc_root_classifies_brownfield() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        write(&root, "docs/guide.adoc");
        let report = assess(&root).unwrap();
        assert_eq!(report.classification, Classification::Brownfield);
    }

    /// A symlink named like a document marks its root populated without
    /// being followed.
    #[test]
    fn a_symlinked_document_under_a_doc_root_classifies_brownfield() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        write(&root, "elsewhere.md");
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::os::unix::fs::symlink(root.join("elsewhere.md"), root.join("docs/architecture.md"))
            .unwrap();
        let report = assess(&root).unwrap();
        assert_eq!(report.classification, Classification::Brownfield);
        assert_eq!(report.populated_doc_roots, vec!["docs".to_string()]);
    }

    /// A broken documentation-root symlink is still a root, and still
    /// populated: the project pointed its docs somewhere, and where does
    /// not matter to the verdict.
    #[test]
    fn a_broken_doc_root_symlink_classifies_brownfield() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        std::os::unix::fs::symlink(root.join("no-such-corpus"), root.join("docs")).unwrap();
        let report = assess(&root).unwrap();
        assert_eq!(report.doc_roots, vec!["docs".to_string()]);
        assert_eq!(report.populated_doc_roots, vec!["docs".to_string()]);
        assert_eq!(report.classification, Classification::Brownfield);
    }

    /// A broken marker symlink still marks: the project pointed its
    /// configuration somewhere, and where does not matter to the verdict.
    #[test]
    fn a_broken_marker_symlink_still_classifies_brownfield() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        std::os::unix::fs::symlink(root.join("no-such-config"), root.join("mkdocs.yml")).unwrap();
        let report = assess(&root).unwrap();
        assert_eq!(report.methodology_markers, vec!["mkdocs.yml".to_string()]);
        assert_eq!(report.classification, Classification::Brownfield);
    }

    /// A broken symlink at a projected destination is a collision: the
    /// path is occupied whatever it points at.
    #[test]
    fn a_broken_destination_symlink_reads_as_a_collision() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        std::fs::create_dir_all(root.join("docs/specs")).unwrap();
        std::os::unix::fs::symlink(
            root.join("gone.md"),
            root.join("docs/specs/SPEC-docs-format.md"),
        )
        .unwrap();
        let report = assess(&root).unwrap();
        assert!(
            report.collisions["codebase"]
                .iter()
                .any(|path| path == "docs/specs/SPEC-docs-format.md")
        );
    }

    /// Evidence that cannot be read is an error, never absence: the
    /// helper itself is exercised, because a whole-assess call would trip
    /// over the walk before the marker probe runs.
    #[test]
    fn an_unreadable_entry_propagates_as_io_rather_than_absence() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        std::fs::create_dir_all(root.join("locked")).unwrap();
        std::fs::write(root.join("locked/mkdocs.yml"), "site_name: x\n").unwrap();
        std::fs::set_permissions(root.join("locked"), std::fs::Permissions::from_mode(0o000))
            .unwrap();
        let result = entry_present(&root.join("locked/mkdocs.yml"));
        std::fs::set_permissions(root.join("locked"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        if nix_is_root() {
            // Mode 000 stays readable to a privileged runner; the case
            // this test constructs does not exist there.
            return;
        }
        match result {
            Err(AppError::Io(_)) => {}
            other => panic!("expected an I/O error, got {other:?}"),
        }
    }

    /// Whether the suite runs privileged, where mode 000 stays readable.
    fn nix_is_root() -> bool {
        std::fs::read_dir("/root").is_ok()
    }

    /// A file target is a usage error, not an empty repository.
    #[test]
    fn a_file_target_refuses_instead_of_classifying() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        write(&root, "just-a-file.md");
        let error = assess(&root.join("just-a-file.md")).unwrap_err();
        assert!(matches!(error, AppError::Usage(_)), "{error}");
    }

    /// A documentation root that is itself a symlink is evidence without
    /// being followed.
    #[test]
    fn a_symlinked_doc_root_classifies_brownfield() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        std::fs::create_dir_all(root.join("external-corpus")).unwrap();
        std::fs::write(root.join("external-corpus/guide.txt"), "prose\n").unwrap();
        std::os::unix::fs::symlink(root.join("external-corpus"), root.join("docs")).unwrap();
        let report = assess(&root).unwrap();
        assert_eq!(report.classification, Classification::Brownfield);
        assert_eq!(report.populated_doc_roots, vec!["docs".to_string()]);
    }

    /// The allowlist takes exact stems only.
    #[test]
    fn a_dotted_metadata_prefix_is_not_metadata() {
        assert!(!is_root_metadata(Utf8Path::new("README.architecture.md")));
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        write(&root, "README.architecture.md");
        let report = assess(&root).unwrap();
        assert_eq!(report.classification, Classification::NeedsDecision);
    }

    #[test]
    fn a_corpus_under_a_doc_root_classifies_brownfield() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        write(&root, "docs/architecture.md");
        let report = assess(&root).unwrap();
        assert_eq!(report.classification, Classification::Brownfield);
        assert_eq!(report.doc_roots, vec!["docs".to_string()]);
        assert_eq!(
            report.documents.paths,
            vec![Utf8PathBuf::from("docs/architecture.md")]
        );
    }

    #[test]
    fn a_methodology_marker_alone_classifies_brownfield() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        write(&root, "README.md");
        write(&root, "mkdocs.yml");
        let report = assess(&root).unwrap();
        assert_eq!(report.classification, Classification::Brownfield);
        assert_eq!(report.methodology_markers, vec!["mkdocs.yml".to_string()]);
    }

    #[test]
    fn scattered_markdown_classifies_needs_decision() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        write(&root, "notes/design.md");
        let report = assess(&root).unwrap();
        assert_eq!(report.classification, Classification::NeedsDecision);
    }

    #[test]
    fn the_workshop_and_pruned_directories_stay_out_of_the_inventory() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        write(&root, ".draft/scratch.md");
        write(&root, "target/build.md");
        write(&root, "node_modules/pkg/README.md");
        let report = assess(&root).unwrap();
        assert_eq!(report.classification, Classification::Greenfield);
        assert_eq!(report.documents.count, 0);
        assert!(report.draft_present);
    }
}
