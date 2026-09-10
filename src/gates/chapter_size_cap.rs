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
const CHAPTER_ZONES: &[&str] = &["./method", "./comparison-docs"];

fn cap_for(file: &Utf8Path) -> usize {
    if file.file_name().is_some_and(|name| {
        name.ends_with("-gates.md")
            || name.ends_with("-checklist.md")
            || matches!(
                name,
                "gates.md" | "checklist.md" | "glossary.md" | "README.md" | "SOURCES.md"
            )
    }) {
        300
    } else {
        200
    }
}

fn is_chapter(file: &Utf8Path) -> bool {
    let Some(name) = file.file_name() else {
        return false;
    };
    if name == "AGENTS.md" {
        return false;
    }
    if name == "glossary.md" || name == "README.md" {
        return true;
    }
    file.extension() == Some("md")
        && file
            .parent()
            .is_some_and(|parent| CHAPTER_ZONES.contains(&parent.as_str()))
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
            if line_count(&read_text(ctx, Utf8Path::new(&file))?) <= cap_for(Utf8Path::new(&file)) {
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
        if !is_chapter(&file) {
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
        if line_count(&read_text(ctx, &file)?) > cap_for(&file) {
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
        write(&dir, "method/chapter.md", "# Chapter\n");
        write(&dir, "node_modules/pkg/README.md", &"line\n".repeat(400));
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn rejects_a_chapter_over_cap() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "method/chapter.md", &"line\n".repeat(201));
        assert_eq!(
            run_in(&dir),
            vec!["FAIL docs-format:chapter-stays-within-200-lines ./method/chapter.md".to_string()]
        );
    }

    #[test]
    fn catalogs_get_the_larger_cap() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "README.md", &"line\n".repeat(300));
        write(&dir, "instance/README.md", &"line\n".repeat(300));
        write(&dir, "method/README.md", &"line\n".repeat(300));
        write(&dir, "method/gates.md", &"line\n".repeat(300));
        write(&dir, "method/checklist.md", &"line\n".repeat(300));
        write(&dir, "method/glossary.md", &"line\n".repeat(300));
        write(&dir, "method/08-gates.md", &"line\n".repeat(300));
        write(&dir, "method/08-checklist.md", &"line\n".repeat(300));
        write(&dir, "comparison-docs/SOURCES.md", &"line\n".repeat(300));
        assert!(run_in(&dir).is_empty());

        for path in [
            "README.md",
            "comparison-docs/SOURCES.md",
            "instance/README.md",
            "method/08-checklist.md",
            "method/08-gates.md",
            "method/README.md",
            "method/checklist.md",
            "method/gates.md",
            "method/glossary.md",
        ] {
            write(&dir, path, &"line\n".repeat(301));
        }
        assert_eq!(run_in(&dir).len(), 9);
    }

    #[test]
    fn debt_exempts_an_oversize_chapter() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "method/debt-chapter.md", &"line\n".repeat(201));
        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "method/debt-chapter.md\n",
        );
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn debt_expires_when_the_chapter_fits_even_unterminated() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "method/debt-chapter.md", "# fits\n");
        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "method/debt-chapter.md\n",
        );
        assert!(run_in(&dir)[0].contains("delist ./method/debt-chapter.md: now fits"));

        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "method/debt-chapter.md",
        );
        assert!(run_in(&dir)[0].contains("now fits"));
    }

    #[test]
    fn debt_expires_when_the_chapter_is_deleted_even_unterminated() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "method/missing-chapter.md\n",
        );
        assert!(run_in(&dir)[0].contains("delist ./method/missing-chapter.md: deleted"));

        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "method/missing-chapter.md",
        );
        assert!(run_in(&dir)[0].contains("deleted"));
    }

    #[test]
    fn rejects_a_slug_named_chapter_in_a_zone() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            "comparison-docs/slug-chapter.md",
            &"line\n".repeat(201),
        );
        assert_eq!(
            run_in(&dir),
            vec![
                "FAIL docs-format:chapter-stays-within-200-lines ./comparison-docs/slug-chapter.md"
                    .to_string()
            ]
        );
    }

    #[test]
    fn ignores_a_slug_named_markdown_file_outside_every_zone() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "reference/slug-document.md", &"line\n".repeat(201));
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn ignores_a_markdown_file_nested_below_a_chapter_zone() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "method/nested/slug-chapter.md", &"line\n".repeat(201));
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn judges_a_glossary_outside_every_zone() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "reference/glossary.md", &"line\n".repeat(301));
        assert_eq!(
            run_in(&dir),
            vec![
                "FAIL docs-format:chapter-stays-within-200-lines ./reference/glossary.md"
                    .to_string()
            ]
        );
    }

    #[test]
    fn ignores_agents_md_in_a_chapter_zone() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "method/AGENTS.md", &"line\n".repeat(301));
        write(&dir, "comparison-docs/AGENTS.md", &"line\n".repeat(301));
        assert!(run_in(&dir).is_empty());
    }
}
