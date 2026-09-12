//! Gate: a chapter stays within its line cap, or within the ceiling its
//! project recorded for it.
//!
//! Chapters get 200 lines; catalogs — gates, checklists, glossaries, READMEs
//! — get 300. A recorded ceiling in `.spec-driven-docs/debt.yaml` is judged
//! instead of the cap and only comes down. The older flat list at
//! `.spec-driven-docs/chapter-size-debt.txt` is still honoured with its skip
//! semantics until `sdd debt migrate --apply` converts it; both files
//! present is a failure rather than a precedence. Vendored trees are pruned.
//! Which caps exist is the format spec's business; this gate only counts.

use camino::Utf8Path;

use crate::domain::debt::{LEGACY_DEBT_PATH, Measurement, Presence};
use crate::domain::finding::Finding;
use crate::domain::gate_id::GateId;
use crate::domain::rule_id::RuleId;
use crate::gates::budget;
use crate::gates::{GateCtx, GateError, GateResult, Violation, line_count, read_text, walk_files};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::ChapterStaysWithinLineCap];

const RULE: RuleId = RuleId::ChapterStaysWithinLineCap;
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

/// Measure every chapter and catalog: one `lines` count per file.
///
/// # Errors
///
/// [`GateError::Io`] when a matched file cannot be read.
pub fn measure(ctx: &GateCtx) -> Result<Vec<Measurement>, GateError> {
    let mut measurements = Vec::new();
    for file in walk_files(ctx) {
        if !is_chapter(&file) {
            continue;
        }
        let lines = line_count(&read_text(ctx, &file)?);
        measurements.push(Measurement::count(
            GateId::ChapterSizeCap,
            file.as_str(),
            "lines",
            lines,
            cap_for(&file),
        ));
    }
    Ok(measurements)
}

/// The legacy list's entries, as `./`-prefixed paths, with the findings its
/// own expiry rules produce.
fn legacy_entries(
    ctx: &GateCtx,
    violations: &mut Vec<Violation>,
) -> Result<Vec<String>, GateError> {
    let mut entries = Vec::new();
    for entry in crate::domain::debt::legacy_list(&read_text(ctx, LEGACY_DEBT_PATH)?) {
        let file = format!("./{entry}");
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
        entries.push(file);
    }
    Ok(entries)
}

/// Judge every chapter and catalog against its cap or its recorded ceiling.
///
/// # Errors
///
/// [`GateError::Io`] when a matched file cannot be read, and
/// [`GateError::Debt`] when the debt file cannot be trusted.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    let debt = budget::read_debt(ctx)?;
    let legacy = if Presence::at(&ctx.repo_root).legacy {
        legacy_entries(ctx, &mut violations)?
    } else {
        Vec::new()
    };
    let measurements: Vec<Measurement> = measure(ctx)?
        .into_iter()
        .filter(|m| {
            let bare = m.path.trim_start_matches("./");
            !legacy
                .iter()
                .any(|entry| entry == &m.path || entry.trim_start_matches("./") == bare)
        })
        .collect();
    violations.extend(budget::judge(
        &debt,
        GateId::ChapterSizeCap,
        RULE,
        &measurements,
        |m| Violation::Finding(Finding::on_file(RULE, m.path.as_str(), "")),
    ));
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
    fn legacy_debt_exempts_an_oversize_chapter() {
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
    fn legacy_debt_expires_when_the_chapter_fits_even_unterminated() {
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
    fn legacy_debt_expires_when_the_chapter_is_deleted_even_unterminated() {
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
    fn a_recorded_ceiling_is_judged_instead_of_the_cap() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "method/legacy.md", &"line\n".repeat(250));
        write(
            &dir,
            ".spec-driven-docs/debt.yaml",
            "schema_version: 1\nchapter-size-cap:\n  method/legacy.md:\n    lines:\n      ceiling: 250\n",
        );
        assert!(run_in(&dir).is_empty());

        write(&dir, "method/legacy.md", &"line\n".repeat(251));
        let out = run_in(&dir);
        assert_eq!(
            out,
            vec![
                "FAIL docs-format:chapter-stays-within-200-lines ./method/legacy.md: 251 lines, recorded ceiling is 250"
                    .to_string()
            ]
        );

        write(&dir, "method/legacy.md", &"line\n".repeat(240));
        let out = run_in(&dir);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("sdd debt tighten --apply"), "{}", out[0]);
    }

    #[test]
    fn a_ceiling_never_lets_a_second_chapter_grow() {
        // The ratchet is per path: a ceiling on one chapter exempts no other.
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "method/legacy.md", &"line\n".repeat(250));
        write(&dir, "method/fresh.md", &"line\n".repeat(201));
        write(
            &dir,
            ".spec-driven-docs/debt.yaml",
            "schema_version: 1\nchapter-size-cap:\n  method/legacy.md:\n    lines:\n      ceiling: 250\n",
        );
        assert_eq!(
            run_in(&dir),
            vec!["FAIL docs-format:chapter-stays-within-200-lines ./method/fresh.md".to_string()]
        );
    }

    #[test]
    fn both_debt_formats_present_is_an_error_naming_migrate() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "method/legacy.md", &"line\n".repeat(250));
        write(&dir, ".spec-driven-docs/debt.yaml", "schema_version: 1\n");
        write(
            &dir,
            ".spec-driven-docs/chapter-size-debt.txt",
            "method/legacy.md\n",
        );
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        let error = run(&ctx, &[]).unwrap_err();
        assert!(
            error.to_string().contains("sdd debt migrate --apply"),
            "{error}"
        );
    }

    #[test]
    fn a_malformed_debt_file_stops_the_gate_rather_than_passing_it() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "method/legacy.md", &"line\n".repeat(250));
        write(
            &dir,
            ".spec-driven-docs/debt.yaml",
            "schema_version: 1\nchapter-size-cap:\n  method/legacy.md:\n    lines: 250\n",
        );
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        let error = run(&ctx, &[]).unwrap_err();
        assert!(error.to_string().contains("method/legacy.md"), "{error}");
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
