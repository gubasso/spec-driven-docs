//! The delivered gates: every check an instance wires as a pre-commit hook.
//!
//! This module owns the registry — identity, display name, hook wiring,
//! citable rules, and implementation for each gate — so a gate cannot exist
//! unwired: the exhaustive match over [`GateId`] is the declaration. Gate
//! implementations live one per file below; rendering the registry into
//! pre-commit YAML and running one gate from the command line live in
//! `services` and `commands`.

pub mod paths;

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
pub mod ki_filename_shape;
pub mod ki_mechanism_walkthrough;
pub mod ki_report_body;
pub mod ki_retire_when;
pub mod no_self_narration;
pub mod prose_stays_unwrapped;
pub mod spec_requirement_parts;
pub mod spec_rule_id_unique;
pub mod spec_size_cap;
pub mod spec_verify_hooks_exist;
pub mod suppression_names_its_case;

use std::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;

use crate::domain::finding::Finding;
use crate::domain::gate_id::GateId;
use crate::domain::rule_id::RuleId;

/// Where a gate runs: the repository root pre-commit invoked it from.
#[derive(Debug, Clone)]
pub struct GateCtx {
    /// The repository root; every path a gate reads or reports is relative to it.
    pub repo_root: Utf8PathBuf,
}

impl GateCtx {
    /// A context rooted at the given repository.
    #[must_use]
    pub fn new(repo_root: impl Into<Utf8PathBuf>) -> Self {
        Self {
            repo_root: repo_root.into(),
        }
    }

    /// Resolve a repository-relative path for reading.
    #[must_use]
    pub fn path(&self, relative: impl AsRef<Utf8Path>) -> Utf8PathBuf {
        self.repo_root.join(relative)
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
    /// The default `files:` pattern, with `{docs_root}` left templated.
    pub files: Option<&'static str>,
    /// The `types:` scope, when the gate takes one.
    pub types: Option<&'static str>,
    /// The default `exclude:` pattern, with `{docs_root}` left templated.
    pub exclude: Option<&'static str>,
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
        id: GateId::AdrFilenameShape,
        name: "decision record filename shape",
        files: Some(r"^{docs_root}/decisions/.*\.md$"),
        types: None,
        exclude: None,
        always_run: false,
        cites: adr_filename_shape::CITES,
        run: adr_filename_shape::run,
    },
    GateSpec {
        id: GateId::AdrWordCap,
        name: "decision record word cap",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: adr_word_cap::CITES,
        run: adr_word_cap::run,
    },
    GateSpec {
        id: GateId::AgentsDigestSize,
        name: "agent digest size",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: agents_digest_size::CITES,
        run: agents_digest_size::run,
    },
    GateSpec {
        id: GateId::ChapterSizeCap,
        name: "chapter and catalog size",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: chapter_size_cap::CITES,
        run: chapter_size_cap::run,
    },
    GateSpec {
        id: GateId::ComparisonDatedTables,
        name: "comparison tables are dated",
        files: Some(r"(^|/)COMPARISON-[a-z0-9-]+\.md$"),
        types: None,
        exclude: None,
        always_run: false,
        cites: comparison_dated_tables::CITES,
        run: comparison_dated_tables::run,
    },
    GateSpec {
        id: GateId::ComparisonEscapedPipes,
        name: "comparison table pipes are escaped",
        files: Some(r"(^|/)COMPARISON-[a-z0-9-]+\.md$"),
        types: None,
        exclude: None,
        always_run: false,
        cites: comparison_escaped_pipes::CITES,
        run: comparison_escaped_pipes::run,
    },
    GateSpec {
        id: GateId::ComparisonLegend,
        name: "comparison legend",
        files: Some(r"(^|/)COMPARISON-[a-z0-9-]+\.md$"),
        types: None,
        exclude: None,
        always_run: false,
        cites: comparison_legend::CITES,
        run: comparison_legend::run,
    },
    GateSpec {
        id: GateId::ComparisonOneReferencePerCell,
        name: "one reference per comparison cell",
        files: Some(r"(^|/)COMPARISON-[a-z0-9-]+\.md$"),
        types: None,
        exclude: None,
        always_run: false,
        cites: comparison_one_reference_per_cell::CITES,
        run: comparison_one_reference_per_cell::run,
    },
    GateSpec {
        id: GateId::ComparisonVerdictWord,
        name: "comparison verdict word",
        files: Some(r"(^|/)COMPARISON-[a-z0-9-]+\.md$"),
        types: None,
        exclude: None,
        always_run: false,
        cites: comparison_verdict_word::CITES,
        run: comparison_verdict_word::run,
    },
    GateSpec {
        id: GateId::GateMessageCitesARule,
        name: "gate messages cite a rule",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: gate_message_cites_a_rule::CITES,
        run: gate_message_cites_a_rule::run,
    },
    GateSpec {
        id: GateId::InstanceManifest,
        name: "instance manifest",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: instance_manifest::CITES,
        run: instance_manifest::run,
    },
    GateSpec {
        id: GateId::KiBugzillaReportWidth,
        name: "Bugzilla report width",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: ki_bugzilla_report_width::CITES,
        run: ki_bugzilla_report_width::run,
    },
    GateSpec {
        id: GateId::KiFilenameShape,
        name: "known issue filename shape",
        files: Some(r"^{docs_root}/reference/known-issues/.*\.md$"),
        types: None,
        exclude: None,
        always_run: false,
        cites: ki_filename_shape::CITES,
        run: ki_filename_shape::run,
    },
    GateSpec {
        id: GateId::KiMechanismWalkthrough,
        name: "known issue mechanism walkthrough",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: ki_mechanism_walkthrough::CITES,
        run: ki_mechanism_walkthrough::run,
    },
    GateSpec {
        id: GateId::KiReportBody,
        name: "known issue report body",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: ki_report_body::CITES,
        run: ki_report_body::run,
    },
    GateSpec {
        id: GateId::KiRetireWhen,
        name: "known issue retirement condition",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: ki_retire_when::CITES,
        run: ki_retire_when::run,
    },
    GateSpec {
        id: GateId::NoSelfNarration,
        name: "documents state the present",
        files: None,
        types: Some("markdown"),
        exclude: Some("^{docs_root}/decisions/"),
        always_run: false,
        cites: no_self_narration::CITES,
        run: no_self_narration::run,
    },
    GateSpec {
        id: GateId::ProseStaysUnwrapped,
        name: "prose lines stay unwrapped",
        files: None,
        types: Some("markdown"),
        exclude: Some(r"(?:^|/)CHANGELOG\.md$"),
        always_run: false,
        cites: prose_stays_unwrapped::CITES,
        run: prose_stays_unwrapped::run,
    },
    GateSpec {
        id: GateId::SpecRequirementParts,
        name: "spec requirement parts",
        files: Some(r"^{docs_root}/specs/SPEC-.*\.md$"),
        types: None,
        exclude: None,
        always_run: false,
        cites: spec_requirement_parts::CITES,
        run: spec_requirement_parts::run,
    },
    GateSpec {
        id: GateId::SpecRuleIdUnique,
        name: "spec rule IDs are unique",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: spec_rule_id_unique::CITES,
        run: spec_rule_id_unique::run,
    },
    GateSpec {
        id: GateId::SpecSizeCap,
        name: "spec size cap",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: spec_size_cap::CITES,
        run: spec_size_cap::run,
    },
    GateSpec {
        id: GateId::SpecVerifyHooksExist,
        name: "spec hook references exist",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: spec_verify_hooks_exist::CITES,
        run: spec_verify_hooks_exist::run,
    },
    GateSpec {
        id: GateId::SuppressionNamesItsCase,
        name: "suppressions name a known issue",
        files: None,
        types: None,
        exclude: None,
        always_run: true,
        cites: suppression_names_its_case::CITES,
        run: suppression_names_its_case::run,
    },
];

/// The directories every repository walk prunes: vendored or generated trees
/// a consumer cannot be asked to author.
pub const PRUNED_DIRS: &[&str] = &[".git", "node_modules", ".venv", "vendor", "target", "dist"];

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

/// Walk the repository, pruning [`PRUNED_DIRS`], and yield every file as a
/// `./`-prefixed repository-relative path in sorted order.
#[must_use]
pub fn walk_files(ctx: &GateCtx) -> Vec<Utf8PathBuf> {
    let root = ctx.repo_root.as_std_path();
    let mut files: Vec<Utf8PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            !(entry.file_type().is_dir()
                && entry.depth() > 0
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| PRUNED_DIRS.contains(&name)))
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let relative = entry.path().strip_prefix(root).ok()?.to_str()?;
            Some(Utf8PathBuf::from(format!("./{relative}")))
        })
        .collect();
    files.sort();
    files
}

#[cfg(test)]
pub(crate) mod tests_support {
    /// A repository holding one known-issue record with the given
    /// `retire_when:` line and a conforming body.
    pub fn ki_fixture(retire_line: &str) -> tempfile::TempDir {
        ki_record(&format!(
            "---\nupstream: https://example.invalid/issues\n{retire_line}---\n# Vendor issue\n## How it works\nRun.\n"
        ))
    }

    /// A repository holding one known-issue record with a conforming
    /// frontmatter and the given body.
    pub fn ki_fixture_body(body: &str) -> tempfile::TempDir {
        ki_record(&format!(
            "---\nupstream: https://example.invalid/issues\nretire_when: release >= 2.0\n---\n{body}"
        ))
    }

    /// A repository holding one known-issue record with the given
    /// `upstream:` value and body.
    pub fn ki_fixture_upstream(upstream: &str, body: &str) -> tempfile::TempDir {
        ki_record(&format!(
            "---\nupstream: {upstream}\nretire_when: release >= 2.0\n---\n{body}"
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
