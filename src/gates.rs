//! The delivered gates: every check an instance wires as a pre-commit hook.
//!
//! This module owns the registry — identity, display name, hook wiring,
//! citable rules, and implementation for each gate — so a gate cannot exist
//! unwired: the exhaustive match over [`GateId`] is the declaration. Gate
//! implementations live one per file below; rendering the registry into
//! pre-commit YAML and running one gate from the command line live in
//! `services` and `commands`.

pub mod paths;

pub mod adr_cites_a_live_rule;
pub mod adr_filename_shape;
pub mod adr_word_cap;
pub mod agents_digest_size;
pub mod chapter_size_cap;
pub mod comparison_dated_tables;
pub mod comparison_escaped_pipes;
pub mod comparison_legend;
pub mod comparison_one_reference_per_cell;
pub mod comparison_verdict_word;
pub mod gate_message_cites_a_rule;
pub mod instance_manifest;
pub mod ki_bugzilla_report_width;
pub mod ki_checked_date;
pub mod ki_filename_shape;
pub mod ki_filing;
pub mod ki_mechanism_walkthrough;
pub mod ki_record;
pub mod ki_report_body;
pub mod ki_retire_when;
pub mod ki_state;
pub mod markdown_prose;
pub mod no_personal_path;
pub mod no_self_narration;
pub mod prose_stays_unwrapped;
pub mod spec_change_is_typed;
pub mod spec_requirement_parts;
pub mod spec_rule_id_unique;
pub mod spec_size_cap;
pub mod spec_verify_hooks_exist;
pub mod suppression_names_its_case;
pub mod tracking_registry;

use std::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;

use crate::domain::finding::Finding;
use crate::domain::gate_id::GateId;
use crate::domain::path_filter::PathFilter;
use crate::domain::rule_id::RuleId;

/// Where a gate runs: the repository root pre-commit invoked it from, and
/// the subject filter that bounds what it judges there.
///
/// # Subject paths and support paths
///
/// A *subject* path is one whose content the gate judges and which can
/// appear in a finding. A *support* path is one the gate reads to know what
/// to judge: the canon manifest, the known-issue records, the docs-root
/// resolution, the tracking registry. The filter governs subject paths.
/// [`Self::path`] and [`read_text`] stay open, because a filter that reached
/// support paths would let a project disable a gate by excluding the file
/// that configures it.
///
/// # Every route a subject path takes
///
/// There are three, and each passes through [`Self::subjects`], so a gate
/// author cannot reach an unfiltered subject list:
///
/// 1. The `&[String]` a gate is handed, filtered in `commands::gate`.
/// 2. [`walk_files`], which filters before it returns.
/// 3. [`crate::gates::spec_change_is_typed`], which resolves its own
///    candidate set and filters it explicitly.
///
/// `canon::every_subject_producer_is_filter_aware` holds that list.
#[derive(Debug)]
pub struct GateCtx {
    /// The repository root; every path a gate reads or reports is relative to it.
    pub repo_root: Utf8PathBuf,
    /// What this gate may judge. Private, so the only way to a subject list
    /// is [`Self::subjects`].
    filter: PathFilter,
}

impl GateCtx {
    /// A context rooted at the given repository, judging everything.
    ///
    /// This is the shape every test and every internal caller wants. The
    /// command path uses [`Self::with_filter`].
    #[must_use]
    pub fn new(repo_root: impl Into<Utf8PathBuf>) -> Self {
        Self {
            repo_root: repo_root.into(),
            filter: PathFilter::permissive(),
        }
    }

    /// A context whose gate judges only what the filter admits.
    #[must_use]
    pub fn with_filter(repo_root: impl Into<Utf8PathBuf>, filter: PathFilter) -> Self {
        Self {
            repo_root: repo_root.into(),
            filter,
        }
    }

    /// Resolve a repository-relative path for reading.
    ///
    /// Deliberately unfiltered: a gate reads its support files through here.
    #[must_use]
    pub fn path(&self, relative: impl AsRef<Utf8Path>) -> Utf8PathBuf {
        self.repo_root.join(relative)
    }

    /// The form a pattern speaks: repository-relative, forward-slashed.
    ///
    /// An absolute path naming a file inside the repository is the same
    /// file as its relative name, and a declaration only ever writes the
    /// relative one. Without this, naming a reserved file by absolute path
    /// would walk straight past the reservation.
    fn relative(&self, path: &Utf8Path) -> Utf8PathBuf {
        if path.is_relative() {
            return path.to_path_buf();
        }
        let (Ok(resolved), Ok(root)) = (
            std::fs::canonicalize(path),
            std::fs::canonicalize(&self.repo_root),
        ) else {
            return path.to_path_buf();
        };
        resolved
            .strip_prefix(&root)
            .ok()
            .and_then(|rest| Utf8PathBuf::from_path_buf(rest.to_path_buf()).ok())
            .unwrap_or_else(|| path.to_path_buf())
    }

    /// The subset of `candidates` this gate judges.
    ///
    /// Every subject path pre-commit or an operator hands a gate comes
    /// through here, and the registry whitelist binds.
    #[must_use]
    pub fn subjects<P: AsRef<Utf8Path>>(&self, candidates: impl IntoIterator<Item = P>) -> Vec<P> {
        candidates
            .into_iter()
            .filter(|path| self.filter.judges(&self.relative(path.as_ref())))
            .collect()
    }

    /// The subset of `candidates` this gate's exclusions leave.
    ///
    /// For a subject set the gate discovered itself. See
    /// [`PathFilter::retains`].
    #[must_use]
    pub fn retained<P: AsRef<Utf8Path>>(&self, candidates: impl IntoIterator<Item = P>) -> Vec<P> {
        candidates
            .into_iter()
            .filter(|path| self.filter.retains(&self.relative(path.as_ref())))
            .collect()
    }

    /// The filter itself, for `--explain` and for the renderer.
    #[must_use]
    pub const fn filter(&self) -> &PathFilter {
        &self.filter
    }
}

/// One line a failing gate prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A rule violation, rendered as its `FAIL <domain>:<rule> ...` line.
    Finding(Finding),
    /// The repository does not have the shape the gate needs; rendered as
    /// `FAIL <reason>` with no rule to cite.
    Layout(String),
    /// A continuation line under a preceding violation, rendered verbatim.
    Note(String),
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Finding(finding) => finding.fmt(f),
            Self::Layout(reason) => write!(f, "FAIL {reason}"),
            Self::Note(text) => f.write_str(text),
        }
    }
}

/// A gate that could not run at all — distinct from one that found violations.
#[derive(Debug, Error)]
pub enum GateError {
    /// A file the gate needed could not be read.
    #[error("{path}: {source}")]
    Io {
        /// The path that failed.
        path: Utf8PathBuf,
        /// The underlying failure.
        source: std::io::Error,
    },
}

impl GateError {
    pub(crate) fn io(path: impl Into<Utf8PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

impl From<GateError> for crate::error::AppError {
    fn from(error: GateError) -> Self {
        match error {
            GateError::Io { path, source } => {
                let kind = source.kind();
                Self::Io(std::io::Error::new(kind, format!("{path}: {source}")))
            }
        }
    }
}

/// What every gate returns: the violations it found, or why it could not run.
pub type GateResult = Result<Vec<Violation>, GateError>;

/// The implementation shape shared by every gate.
pub type GateFn = fn(&GateCtx, &[String]) -> GateResult;

/// One registry row: everything the deliveries need to know about a gate.
#[derive(Debug)]
pub struct GateSpec {
    /// The gate's identity.
    pub id: GateId,
    /// The display name pre-commit shows.
    pub name: &'static str,
    /// The subject paths this gate judges, as include globs with
    /// `{docs_root}` left templated.
    ///
    /// Every row states them, under
    /// `release:a-delivered-gate-reads-what-the-convention-owns`. An empty
    /// list judges everything the excludes leave, and a row that states one
    /// carries a comment saying why.
    pub include: &'static [&'static str],
    /// The `types:` scope, when the gate takes one.
    ///
    /// Pre-commit applies it in addition to the rendered patterns. `sdd
    /// gate` does not, which is why `--explain` prints it rather than
    /// folding it into the answer.
    pub types: Option<&'static str>,
    /// The subject paths this gate never judges, as exclude globs with
    /// `{docs_root}` left templated.
    pub exclude: &'static [&'static str],
    /// Whether the gate runs regardless of which files changed.
    pub always_run: bool,
    /// Every rule the gate can cite in a finding.
    pub cites: &'static [RuleId],
    /// The implementation.
    pub run: GateFn,
}

/// Look up one gate's registry row.
#[must_use]
pub fn spec(id: GateId) -> &'static GateSpec {
    let index = GateId::ALL.iter().position(|g| *g == id).unwrap_or(0);
    &GATES[index]
}

/// The delivered gate set, in [`GateId::ALL`] order.
pub static GATES: &[GateSpec] = &[
    GateSpec {
        id: GateId::AdrCitesALiveRule,
        name: "decision record citations resolve",
        include: &[r"{docs_root}/decisions/*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: adr_cites_a_live_rule::CITES,
        run: adr_cites_a_live_rule::run,
    },
    GateSpec {
        id: GateId::AdrFilenameShape,
        name: "decision record filename shape",
        include: &[r"{docs_root}/decisions/*.md"],
        types: None,
        exclude: &[],
        always_run: false,
        cites: adr_filename_shape::CITES,
        run: adr_filename_shape::run,
    },
    GateSpec {
        id: GateId::AdrWordCap,
        name: "decision record word cap",
        include: &[r"{docs_root}/decisions/*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: adr_word_cap::CITES,
        run: adr_word_cap::run,
    },
    GateSpec {
        id: GateId::AgentsDigestSize,
        name: "agent digest size",
        include: &[r"**/AGENTS.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: agents_digest_size::CITES,
        run: agents_digest_size::run,
    },
    GateSpec {
        id: GateId::ChapterSizeCap,
        name: "chapter and catalog size",
        include: &[r"**/*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: chapter_size_cap::CITES,
        run: chapter_size_cap::run,
    },
    GateSpec {
        id: GateId::ComparisonDatedTables,
        name: "comparison tables are dated",
        include: &[r"**/COMPARISON-*.md"],
        types: None,
        exclude: &[],
        always_run: false,
        cites: comparison_dated_tables::CITES,
        run: comparison_dated_tables::run,
    },
    GateSpec {
        id: GateId::ComparisonEscapedPipes,
        name: "comparison table pipes are escaped",
        include: &[r"**/COMPARISON-*.md"],
        types: None,
        exclude: &[],
        always_run: false,
        cites: comparison_escaped_pipes::CITES,
        run: comparison_escaped_pipes::run,
    },
    GateSpec {
        id: GateId::ComparisonLegend,
        name: "comparison legend",
        include: &[r"**/COMPARISON-*.md"],
        types: None,
        exclude: &[],
        always_run: false,
        cites: comparison_legend::CITES,
        run: comparison_legend::run,
    },
    GateSpec {
        id: GateId::ComparisonOneReferencePerCell,
        name: "one reference per comparison cell",
        include: &[r"**/COMPARISON-*.md"],
        types: None,
        exclude: &[],
        always_run: false,
        cites: comparison_one_reference_per_cell::CITES,
        run: comparison_one_reference_per_cell::run,
    },
    GateSpec {
        id: GateId::ComparisonVerdictWord,
        name: "comparison verdict word",
        include: &[r"**/COMPARISON-*.md"],
        types: None,
        exclude: &[],
        always_run: false,
        cites: comparison_verdict_word::CITES,
        run: comparison_verdict_word::run,
    },
    GateSpec {
        id: GateId::GateMessageCitesARule,
        name: "gate messages cite a rule",
        // Judges the registry itself, not a path in the tree: the
        // subject is every gate row, and the specs it resolves them
        // against are support.
        include: &[],
        types: None,
        exclude: &[],
        always_run: true,
        cites: gate_message_cites_a_rule::CITES,
        run: gate_message_cites_a_rule::run,
    },
    GateSpec {
        id: GateId::InstanceManifest,
        name: "instance manifest",
        include: &[r".spec-driven-docs/manifest.json"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: instance_manifest::CITES,
        run: instance_manifest::run,
    },
    GateSpec {
        id: GateId::KiBugzillaReportWidth,
        name: "Bugzilla report width",
        include: &[r"{docs_root}/reference/known-issues/*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: ki_bugzilla_report_width::CITES,
        run: ki_bugzilla_report_width::run,
    },
    GateSpec {
        id: GateId::KiCheckedDate,
        name: "known issue last-check date",
        include: &[r"{docs_root}/reference/known-issues/*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: ki_checked_date::CITES,
        run: ki_checked_date::run,
    },
    GateSpec {
        id: GateId::KiFilenameShape,
        name: "known issue filename shape",
        include: &[r"{docs_root}/reference/known-issues/*.md"],
        types: None,
        exclude: &[],
        always_run: false,
        cites: ki_filename_shape::CITES,
        run: ki_filename_shape::run,
    },
    GateSpec {
        id: GateId::KiFiling,
        name: "known issue filing state",
        include: &[r"{docs_root}/reference/known-issues/*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: ki_filing::CITES,
        run: ki_filing::run,
    },
    GateSpec {
        id: GateId::KiMechanismWalkthrough,
        name: "known issue mechanism walkthrough",
        include: &[r"{docs_root}/reference/known-issues/*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: ki_mechanism_walkthrough::CITES,
        run: ki_mechanism_walkthrough::run,
    },
    GateSpec {
        id: GateId::KiReportBody,
        name: "known issue report body",
        include: &[r"{docs_root}/reference/known-issues/*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: ki_report_body::CITES,
        run: ki_report_body::run,
    },
    GateSpec {
        id: GateId::KiRetireWhen,
        name: "known issue retirement condition",
        include: &[r"{docs_root}/reference/known-issues/*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: ki_retire_when::CITES,
        run: ki_retire_when::run,
    },
    GateSpec {
        id: GateId::KiState,
        name: "known issue state",
        include: &[r"{docs_root}/reference/known-issues/*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: ki_state::CITES,
        run: ki_state::run,
    },
    GateSpec {
        id: GateId::NoPersonalPath,
        name: "no personal path",
        // Judges the whole project. Whether a string is a real person's
        // home directory does not depend on which conventions a project
        // follows, so a false positive is nearly impossible and the value
        // is entirely in breadth. v0.6.5 anchored this to the documentation
        // root over two register collisions, which a leak check does not
        // have: a rendered release block carries no home directory. A
        // project that needs a path exempt reserves it
        // (ADR-a-project-declares-what-its-gates-read).
        include: &[],
        types: Some("text"),
        exclude: &[],
        always_run: false,
        cites: no_personal_path::CITES,
        run: no_personal_path::run,
    },
    GateSpec {
        id: GateId::NoSelfNarration,
        name: "documents state the present",
        include: &[r"{docs_root}/**/*.md"],
        types: Some("markdown"),
        exclude: &[r"{docs_root}/decisions/**"],
        always_run: false,
        cites: no_self_narration::CITES,
        run: no_self_narration::run,
    },
    GateSpec {
        id: GateId::ProseStaysUnwrapped,
        name: "prose lines stay unwrapped",
        include: &[r"{docs_root}/**/*.md"],
        types: Some("markdown"),
        exclude: &[r"**/CHANGELOG.md"],
        always_run: false,
        cites: prose_stays_unwrapped::CITES,
        run: prose_stays_unwrapped::run,
    },
    GateSpec {
        id: GateId::SpecChangeIsTyped,
        name: "spec changes are typed",
        // Judges whatever the project's declared plan zone holds, and the
        // zone is the project's own choice of path, so no canon pattern can
        // name it. The declaration already bounds this gate by naming the
        // zone; `reserved:` still reaches inside it.
        include: &[],
        types: None,
        exclude: &[],
        always_run: true,
        cites: spec_change_is_typed::CITES,
        run: spec_change_is_typed::run,
    },
    GateSpec {
        id: GateId::SpecRequirementParts,
        name: "spec requirement parts",
        include: &[r"{docs_root}/specs/SPEC-*.md"],
        types: None,
        exclude: &[],
        always_run: false,
        cites: spec_requirement_parts::CITES,
        run: spec_requirement_parts::run,
    },
    GateSpec {
        id: GateId::SpecRuleIdUnique,
        name: "spec rule IDs are unique",
        include: &[r"{docs_root}/specs/SPEC-*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: spec_rule_id_unique::CITES,
        run: spec_rule_id_unique::run,
    },
    GateSpec {
        id: GateId::SpecSizeCap,
        name: "spec size cap",
        include: &[r"{docs_root}/specs/SPEC-*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: spec_size_cap::CITES,
        run: spec_size_cap::run,
    },
    GateSpec {
        id: GateId::SpecVerifyHooksExist,
        name: "spec hook references exist",
        include: &[r"{docs_root}/specs/SPEC-*.md"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: spec_verify_hooks_exist::CITES,
        run: spec_verify_hooks_exist::run,
    },
    GateSpec {
        id: GateId::SuppressionNamesItsCase,
        name: "suppressions name a known issue",
        // Judges every source file in the project, because a
        // suppression can be written in any of them. The known-issue
        // records it resolves a case against are support.
        include: &[],
        types: None,
        exclude: &[],
        always_run: true,
        cites: suppression_names_its_case::CITES,
        run: suppression_names_its_case::run,
    },
    GateSpec {
        id: GateId::TrackingRegistry,
        name: "tracking registry is valid and current",
        include: &[r"{docs_root}/reference/tracking.yaml"],
        types: None,
        exclude: &[],
        always_run: true,
        cites: tracking_registry::CITES,
        run: tracking_registry::run,
    },
];

/// The directories every repository walk prunes: vendored or generated trees
/// a consumer cannot be asked to author.
pub const PRUNED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".venv",
    "vendor",
    "third-party",
    "target",
    "dist",
];

/// Count the newline-terminated lines of a text, as `wc -l` does.
#[must_use]
pub fn line_count(text: &str) -> usize {
    text.matches('\n').count()
}

/// Read a repository-relative text file for a gate.
///
/// # Errors
///
/// [`GateError::Io`] naming the path when the file cannot be read.
pub fn read_text(ctx: &GateCtx, relative: impl AsRef<Utf8Path>) -> Result<String, GateError> {
    let relative = relative.as_ref();
    std::fs::read_to_string(ctx.path(relative)).map_err(|source| GateError::io(relative, source))
}

/// Every value a front-matter key carries, in the order the keys appear.
///
/// The scan is the leading `---` block alone, so a `state:` line in the
/// prose below it is text about the record rather than the record's own
/// field. A key stated twice yields two entries, which is what makes
/// "exactly one" decidable.
#[must_use]
pub fn front_matter_values(text: &str, key: &str) -> Vec<String> {
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Vec::new();
    }
    lines
        .take_while(|line| *line != "---")
        .filter_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix(':'))
        })
        .map(|value| value.trim().to_string())
        .collect()
}

/// The traversal pruner: [`PRUNED_DIRS`] as an `ignore` override.
///
/// `Override` is the right tool here and the wrong one in
/// [`crate::domain::path_filter`]. Pruning wants one boolean per directory
/// and no provenance, which is exactly what it gives.
fn pruner(root: &Utf8Path) -> ignore::overrides::Override {
    let mut builder = ignore::overrides::OverrideBuilder::new(root.as_std_path());
    for dir in PRUNED_DIRS {
        // `!` marks an exclude in `Override`'s own grammar, which is not
        // the restricted grammar `PathFilter` carries.
        let _ = builder.add(&format!("!{dir}/**"));
        let _ = builder.add(&format!("!{dir}"));
    }
    builder
        .build()
        .unwrap_or_else(|_| ignore::overrides::Override::empty())
}

/// Walk the repository and yield every file as a `./`-prefixed
/// repository-relative path in sorted order.
///
/// The walk prunes [`PRUNED_DIRS`] and honours the repository's committed
/// `.gitignore`. It honours no machine-local ignore source: `.git/info/exclude`,
/// the user's global excludes file, and ignore files above the repository
/// root are all disabled, because a gate whose answer depends on whose
/// checkout it runs in is not a gate.
#[must_use]
pub fn walk_files(ctx: &GateCtx) -> Vec<Utf8PathBuf> {
    let root = ctx.repo_root.as_std_path();
    let mut files: Vec<Utf8PathBuf> = ignore::WalkBuilder::new(root)
        .standard_filters(false)
        .git_ignore(true)
        .git_exclude(false)
        .git_global(false)
        .ignore(false)
        .parents(false)
        .require_git(false)
        .hidden(false)
        .overrides(pruner(&ctx.repo_root))
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let relative = entry.path().strip_prefix(root).ok()?.to_str()?;
            Some(Utf8PathBuf::from(format!("./{relative}")))
        })
        .collect();
    files.sort();
    ctx.subjects(files)
}

#[cfg(test)]
pub(crate) mod tests_support {
    /// A repository holding one known-issue record with the given `state:`
    /// value and `retire_when:` line.
    pub fn ki_fixture_state(state: &str, retire_line: &str) -> tempfile::TempDir {
        ki_record(&format!(
            "---\nupstream: https://example.invalid/issues\nstate: {state}\nfiling: gathering\n{retire_line}---\n# Vendor issue\n## How it works\nRun.\n"
        ))
    }

    /// A repository holding one known-issue record with the given `state:`
    /// value and `checked:` line.
    pub fn ki_fixture_checked(state: &str, checked_line: &str) -> tempfile::TempDir {
        ki_record(&format!(
            "---\nupstream: https://example.invalid/issues\nstate: {state}\nfiling: gathering\nretire_when: release >= 2.0\n{checked_line}---\n# Vendor issue\n## How it works\nRun.\n"
        ))
    }

    /// A repository holding one known-issue record with a conforming
    /// frontmatter and the given body.
    pub fn ki_fixture_body(body: &str) -> tempfile::TempDir {
        ki_record(&format!(
            "---\nupstream: https://example.invalid/issues\nstate: masked\nfiling: gathering\nretire_when: release >= 2.0\n---\n{body}"
        ))
    }

    /// A repository holding one filed known-issue record with the given
    /// `upstream:` value and body.
    pub fn ki_fixture_upstream(upstream: &str, body: &str) -> tempfile::TempDir {
        ki_fixture_filing("filed", upstream, body)
    }

    /// A repository holding one known-issue record with the given `filing:`
    /// value, `upstream:` value and body.
    pub fn ki_fixture_filing(filing: &str, upstream: &str, body: &str) -> tempfile::TempDir {
        ki_record(&format!(
            "---\nupstream: {upstream}\nstate: masked\nfiling: {filing}\nretire_when: release >= 2.0\n---\n{body}"
        ))
    }

    fn ki_record(text: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let records = dir.path().join("_docs/reference/known-issues");
        std::fs::create_dir_all(&records).unwrap();
        std::fs::write(records.join("KI-vendor.md"), text).unwrap();
        dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A repository holding one file at each named path.
    fn tree(paths: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a scratch directory");
        for (path, body) in paths {
            let full = dir.path().join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).expect("the parent exists");
            }
            std::fs::write(&full, body).expect("the file is written");
        }
        dir
    }

    fn walked(dir: &tempfile::TempDir) -> Vec<String> {
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
            .expect("the scratch path is UTF-8");
        walk_files(&GateCtx::new(root))
            .into_iter()
            .map(|p| p.to_string())
            .collect()
    }

    #[test]
    fn walk_files_skips_a_gitignored_file() {
        let dir = tree(&[
            (".gitignore", "generated.md\n"),
            ("generated.md", "x\n"),
            ("kept.md", "x\n"),
        ]);
        let files = walked(&dir);
        assert!(files.contains(&"./kept.md".to_string()));
        assert!(
            !files.contains(&"./generated.md".to_string()),
            "a git-ignored file still reached a walking gate: {files:?}"
        );
    }

    #[test]
    fn walk_files_ignores_a_machine_local_exclude_file() {
        // The hostile case. A machine-local exclude must not hide a governed
        // file, or one operator's checkout reports a violation another's
        // does not.
        let dir = tree(&[
            (".git/info/exclude", "governed.md\n"),
            ("governed.md", "x\n"),
        ]);
        assert!(
            walked(&dir).contains(&"./governed.md".to_string()),
            "a machine-local exclude hid a governed file"
        );
    }

    #[test]
    fn walk_files_still_prunes_the_pruned_dirs() {
        let dir = tree(&[
            ("target/debug/artifact", "x\n"),
            ("node_modules/pkg/index.js", "x\n"),
            ("src/main.rs", "x\n"),
        ]);
        let files = walked(&dir);
        assert_eq!(files, vec!["./src/main.rs".to_string()]);
    }

    #[test]
    fn walk_files_yields_dotted_paths() {
        let dir = tree(&[(".markdownlint/base.yaml", "x\n")]);
        assert!(walked(&dir).contains(&"./.markdownlint/base.yaml".to_string()));
    }

    #[test]
    fn registry_covers_every_gate_exactly_once_in_order() {
        assert_eq!(GATES.len(), GateId::ALL.len());
        for (row, id) in GATES.iter().zip(GateId::ALL) {
            assert_eq!(row.id, *id);
            assert_eq!(spec(*id).id, *id);
        }
    }

    #[test]
    fn every_gate_declares_the_rules_it_cites() {
        for row in GATES {
            assert!(!row.cites.is_empty(), "{} cites nothing", row.id);
        }
    }

    #[test]
    fn cited_rules_resolve_in_the_embedded_specs() {
        let defined = crate::embedded::spec_rule_ids();
        for row in GATES {
            for rule in row.cites {
                assert!(
                    defined.contains(rule.as_str()),
                    "{}: {rule} is undefined",
                    row.id
                );
            }
        }
    }
}
