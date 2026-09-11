//! Gate: a cited known-issue case resolves to a record.
//!
//! A suppression over a defect this project does not own names the
//! `KI-<slug>` case that justifies it. The name is worth nothing where it
//! resolves to no record, so this gate reads the one token this convention
//! defines and checks it against the records the instance keeps. A commit
//! that deletes a record while a citation still names it is the failure this
//! exists to catch.
//!
//! The gate judges no suppression syntax. Whether a suppression states a
//! reason belongs to the linter that honors it, where that linter has a rule
//! for it: clippy has `clippy::allow_attributes_without_reason` and `ESLint`
//! has `eslint-comments/require-description`. Where it has none, as Ruff
//! does not, the reason is a review obligation. A delivered gate parses no
//! grammar this convention does not define, so no table of comment openers,
//! filename suffixes, or per-tool reason positions lives here.
//!
//! SATISFIES spec-to-code:a-suppression-names-its-case
//!
//! The scan reads every file the walk yields and needs no knowledge of the
//! language it is reading. The registry row excludes the documentation root,
//! because a specification, a chapter, and a record each write the token
//! while teaching it. A project whose test fixtures write the token reserves
//! those paths in its own declaration.
//!
//! The gate runs always rather than over the staged files. Pre-commit selects
//! staged files with `--diff-filter=ACMRTUXB`, which omits deletions, so a
//! filter would hand this check an empty list on exactly the commit it is
//! for.

use std::collections::BTreeSet;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, GateError, GateResult, Violation, walk_files};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::SuppressionNamesItsCase];

const CASE: RuleId = RuleId::SuppressionNamesItsCase;

/// Whether a byte run reads as binary, judged by a NUL near its start.
fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(4096).any(|&b| b == 0)
}

/// Whether a token starting at `index` opens on a word boundary, so a longer
/// word that ends in the token is not read as the token.
const fn on_a_boundary(text: &str, index: usize) -> bool {
    index == 0
        || !text.as_bytes()[index - 1].is_ascii_alphanumeric()
            && text.as_bytes()[index - 1] != b'-'
            && text.as_bytes()[index - 1] != b'_'
}

/// Every `KI-<slug>` case one line cites.
fn cited_cases(line: &str) -> impl Iterator<Item = String> + '_ {
    line.match_indices("KI-")
        .filter(|(index, _)| on_a_boundary(line, *index))
        .filter_map(|(index, _)| {
            let rest = &line[index + 3..];
            let slug: String = rest
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                .collect();
            // The slug closes on a boundary too, so `KI-vendor-quirkXYZ` is
            // one unknown case rather than a known one with a suffix.
            let closes = rest[slug.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
            (!slug.is_empty() && closes).then(|| format!("KI-{slug}"))
        })
}

/// One citation, and where a reader opens it.
struct Citation {
    case: String,
    file: String,
    number: usize,
}

/// Every case cited by the files this gate judges.
fn citations(ctx: &GateCtx) -> Result<Vec<Citation>, GateError> {
    let mut found = Vec::new();
    for file in walk_files(ctx) {
        let bytes =
            std::fs::read(ctx.path(&file)).map_err(|source| GateError::io(file.clone(), source))?;
        if looks_binary(&bytes) {
            continue;
        }
        // Decoded loosely, because the token is ASCII and a single stray
        // byte elsewhere in the file must not hide it. Rejecting the whole
        // file there would narrow a scan this rule states unconditionally.
        let text = String::from_utf8_lossy(&bytes);
        let name = file.as_str().trim_start_matches("./").to_string();
        for (index, line) in text.lines().enumerate() {
            found.extend(cited_cases(line).map(|case| Citation {
                case,
                file: name.clone(),
                number: index + 1,
            }));
        }
    }
    Ok(found)
}

/// Judge every cited case against the records the instance keeps.
///
/// # Errors
///
/// [`GateError::Io`] when a candidate file or a record root cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    let known: BTreeSet<String> = ki_records(ctx, args)?
        .iter()
        .filter_map(|record| {
            record
                .file_name()
                .map(|name| name.trim_end_matches(".md").to_string())
        })
        .collect();
    Ok(citations(ctx)?
        .into_iter()
        .filter(|citation| !known.contains(&citation.case))
        .map(|citation| {
            Violation::Finding(Finding::on_line(
                CASE,
                &citation.file,
                citation.number,
                format!("{} resolves to no record", citation.case),
            ))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::path_filter::{Layer, PathFilter, Pattern};

    /// A context over `dir` that excludes one glob, as a project declaration
    /// does.
    fn excluding(dir: &tempfile::TempDir, glob: &str) -> GateCtx {
        let filter =
            PathFilter::build(Vec::new(), vec![Pattern::new(glob, Layer::Project)]).unwrap();
        GateCtx::with_filter(dir.path().to_str().unwrap(), filter)
    }

    /// A repository holding one file at each named path.
    fn tree(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let full = dir.path().join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, text).unwrap();
        }
        dir
    }

    fn run_in(dir: &tempfile::TempDir) -> Vec<String> {
        run(&GateCtx::new(dir.path().to_str().unwrap()), &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// The record this repository's fixtures resolve against.
    const RECORD: (&str, &str) = (
        "_docs/reference/known-issues/KI-vendor-replays.md",
        "# Vendor replays\n",
    );

    /// The scan needs no filename suffix table, so it reaches a language no
    /// table ever named and a file with no suffix at all.
    #[test]
    fn an_absent_record_fails_whatever_file_cites_it() {
        for name in [
            "src/client.rs",
            "deploy/pipeline.yaml",
            "notes.txt",
            "scripts/publish",
            "Makefile",
        ] {
            let dir = tree(&[(name, "mask for KI-vendor-replays\n")]);
            let out = run_in(&dir);
            assert_eq!(out.len(), 1, "{name} reported {out:?}");
            assert_eq!(
                out[0],
                format!(
                    "FAIL spec-to-code:a-suppression-names-its-case {name}:1: \
                     KI-vendor-replays resolves to no record"
                )
            );
        }
    }

    /// An empty zone is the state the check exists to catch, not a reason to
    /// report nothing.
    #[test]
    fn a_repository_keeping_no_records_still_fails_a_citation() {
        let dir = tree(&[("src/client.rs", "// KI-vendor-replays\n")]);
        assert!(!dir.path().join("_docs/reference/known-issues").exists());
        assert_eq!(run_in(&dir).len(), 1);
    }

    #[test]
    fn a_citation_whose_record_exists_passes() {
        let dir = tree(&[("src/client.rs", "// KI-vendor-replays\n"), RECORD]);
        assert!(run_in(&dir).is_empty());
    }

    /// The deliberate loss. A suppression carrying neither a case nor a
    /// reason is another tool's judgment now, and this asserts that it was
    /// chosen rather than overlooked.
    #[test]
    fn a_suppression_with_neither_a_case_nor_a_reason_is_not_this_gates_business() {
        let dir = tree(&[("src/client.rs", "#[allow(dead_code)]\nfn unused() {}\n")]);
        assert!(run_in(&dir).is_empty());
    }

    /// Every site of one fabricated case is reported, because each is a
    /// separate citation a reader has to repair.
    #[test]
    fn each_site_of_one_absent_case_is_reported() {
        let dir = tree(&[
            ("src/a.rs", "// KI-gone\n"),
            ("src/b.rs", "x\n// KI-gone\n"),
        ]);
        let out = run_in(&dir);
        assert_eq!(out.len(), 2, "{out:?}");
        assert!(out[0].contains("src/a.rs:1"));
        assert!(out[1].contains("src/b.rs:2"));
    }

    /// The registry row excludes the documentation root, where a spec, a
    /// chapter, and a record all write the token while teaching it.
    #[test]
    fn an_excluded_path_leaves_the_subject_set() {
        let dir = tree(&[("_docs/specs/SPEC-spec-to-code.md", "cite KI-vendor-500\n")]);
        assert_eq!(run_in(&dir).len(), 1, "the unfiltered scan reads it");
        assert!(run(&excluding(&dir, "_docs/**"), &[]).unwrap().is_empty());
    }

    /// The records stay readable under a filter, because they are support
    /// rather than subject. Excluding them would be an off switch.
    #[test]
    fn the_records_resolve_a_case_even_where_they_are_excluded() {
        let dir = tree(&[("src/client.rs", "// KI-vendor-replays\n"), RECORD]);
        let ctx = excluding(&dir, "_docs/reference/known-issues/**");
        assert!(run(&ctx, &[]).unwrap().is_empty());
    }

    #[test]
    fn a_token_reads_only_on_its_own_word_boundaries() {
        assert_eq!(
            cited_cases("names KI-vendor-500 here").collect::<Vec<_>>(),
            vec!["KI-vendor-500".to_string()]
        );
        // A longer word ending in the token is not the token.
        assert!(cited_cases("WIKI-vendor").next().is_none());
        // The slug closes on a boundary, so this is one unknown case.
        assert_eq!(
            cited_cases("KI-vendorXYZ").collect::<Vec<_>>(),
            Vec::<String>::new()
        );
        // A bare prefix names no case.
        assert!(cited_cases("KI- alone").next().is_none());
    }

    #[test]
    fn a_binary_file_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("blob.bin"), b"KI-gone\0\0\0").unwrap();
        assert!(run_in(&dir).is_empty());
    }

    /// One Latin-1 byte elsewhere in a text file must not hide an ASCII
    /// citation. The scan is unconditional, and only a binary file is out.
    #[test]
    fn a_citation_survives_a_byte_this_process_cannot_decode() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"\xe9\n// KI-gone\n").unwrap();
        let out = run_in(&dir);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].contains("notes.txt:2"));
    }
}
