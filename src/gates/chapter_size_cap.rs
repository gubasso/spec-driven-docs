//! Gate: a chapter stays within its line cap, and the debt list shrinks by
//! itself.
//!
//! Chapters get 200 lines; catalogs — gates, checklists, glossaries, READMEs
//! — get 300. A debt entry exempts one oversize file, and expires the moment
//! the file fits or disappears, so the list can only shrink. Vendored trees
//! are pruned. Which caps exist is the format spec's business; this gate
//! only counts.

use camino::Utf8Path;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, line_count, read_text, walk_files};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::ChapterStaysWithinLineCap];

const RULE: RuleId = RuleId::ChapterStaysWithinLineCap;
const DEBT: &str = ".spec-driven-docs/chapter-size-debt.txt";

fn cap_for(file: &str) -> usize {
    if file.ends_with("-gates.md")
        || file.ends_with("-checklist.md")
        || file.ends_with("glossary.md")
        || file.ends_with("README.md")
    {
        300
    } else {
        200
    }
}

// The shell glob this matches was case-sensitive; `.MD` is not a chapter.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn is_chapter(name: &str) -> bool {
    if name == "glossary.md" || name == "README.md" {
        return true;
    }
    let bytes = name.as_bytes();
    name.ends_with(".md")
        && bytes.len() > 3
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b'-'
}

/// Judge every chapter and catalog, honoring the debt list.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a matched file cannot be read.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    let mut debt_entries: Vec<String> = Vec::new();

    if ctx.path(DEBT).is_file() {
        for entry in read_text(ctx, DEBT)?.lines() {
            if entry.is_empty() || entry.starts_with('#') {
                continue;
            }
            let file = if entry.starts_with("./") {
                entry.to_string()
            } else {
                format!("./{entry}")
            };
            if !ctx.path(&file).is_file() {
                violations.push(Violation::Finding(Finding::on_file(
                    RULE,
                    format!("delist {file}"),
                    "deleted",
                )));
                continue;
            }
            if line_count(&read_text(ctx, Utf8Path::new(&file))?) <= cap_for(&file) {
                violations.push(Violation::Finding(Finding::on_file(
                    RULE,
                    format!("delist {file}"),
                    "now fits",
                )));
            }
            debt_entries.push(file);
        }
    }

    for file in walk_files(ctx) {
        let Some(name) = file.file_name() else {
            continue;
        };
        if !is_chapter(name) {
            continue;
        }
        let as_listed = file.as_str();
        let bare = as_listed.trim_start_matches("./");
        if debt_entries
            .iter()
            .any(|entry| entry == as_listed || entry.trim_start_matches("./") == bare)
        {
            continue;
        }
        if line_count(&read_text(ctx, &file)?) > cap_for(as_listed) {
            violations.push(Violation::Finding(Finding::on_file(RULE, file, "")));
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &tempfile::TempDir, path: &str, content: &str) {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn run_in(dir: &tempfile::TempDir) -> Vec<String> {
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_chapters_within_cap_and_ignores_vendored_trees() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "00-chapter.md", "# Chapter\n");
        write(&dir, "node_modules/pkg/README.md", &"line\n".repeat(400));
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn rejects_a_chapter_over_cap() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "00-chapter.md", &"line\n".repeat(201));
        assert_eq!(
            run_in(&dir),
            vec!["FAIL docs-format:chapter-stays-within-200-lines ./00-chapter.md".to_string()]
        );
    }

    #[test]
    fn catalogs_get_the_larger_cap() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "README.md", &"line\n".repeat(300));
        write(&dir, "08-gates.md", &"line\n".repeat(300));
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn debt_exempts_an_oversize_chapter() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "00-chapter.md", &"line\n".repeat(201));
        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "00-chapter.md\n",
        );
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn debt_expires_when_the_chapter_fits_even_unterminated() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "00-chapter.md", "# fits\n");
        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "00-chapter.md\n",
        );
        assert!(run_in(&dir)[0].contains("delist ./00-chapter.md: now fits"));

        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "00-chapter.md",
        );
        assert!(run_in(&dir)[0].contains("now fits"));
    }

    #[test]
    fn debt_expires_when_the_chapter_is_deleted_even_unterminated() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "missing.md\n",
        );
        assert!(run_in(&dir)[0].contains("delist ./missing.md: deleted"));

        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "missing.md",
        );
        assert!(run_in(&dir)[0].contains("deleted"));
    }
}
