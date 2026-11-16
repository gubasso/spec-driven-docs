//! Gate: agent digests stay within their line budgets, or within the
//! ceiling its project recorded for one.
//!
//! The root `AGENTS.md` is loaded into every session, so its budget is 100
//! lines; a subtree digest gets 150. The budget counts the lines the project
//! writes: a managed block, the region between `<!-- BEGIN <label> -->` and
//! `<!-- END <label> -->` that a tool rewrites at each upgrade, is not the
//! project's to shorten. Each block is held to 20 lines instead, so a block
//! the project writes itself buys no more than that. Vendored trees are
//! pruned — a gate a consumer cannot satisfy short of deleting
//! `node_modules/` is a gate they turn off. What a digest says is review's
//! business; only size is judged.

use crate::domain::debt::Measurement;
use crate::domain::finding::Finding;
use crate::domain::gate_id::GateId;
use crate::domain::paths::AGENTS_DIGEST_PATH;
use crate::domain::rule_id::RuleId;
use crate::gates::budget;
use crate::gates::{GateCtx, GateError, GateResult, Violation, line_count, read_text, walk_files};

/// The most lines one managed block holds, its markers included.
pub const BLOCK_CAP: usize = 20;

/// How one digest divides between the project and the managed blocks.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Split {
    /// Lines outside every managed block.
    pub project: usize,
    /// Lines inside the managed blocks, their markers included.
    pub blocks: usize,
    /// Each block over [`BLOCK_CAP`], as its BEGIN line and its length.
    pub oversized: Vec<(usize, usize)>,
    /// The first marker that does not pair, as its line and the reason.
    /// Where one is set, every line counts as the project's.
    pub broken: Option<(usize, &'static str)>,
}

/// The label of a whole-line marker, where `line` is one.
fn marker<'a>(line: &'a str, word: &str) -> Option<&'a str> {
    line.trim()
        .strip_prefix("<!-- ")?
        .strip_suffix(" -->")?
        .strip_prefix(word)?
        .strip_prefix(' ')
        .filter(|label| !label.is_empty())
}

/// Divide a digest into the project's lines and the managed blocks.
///
/// A marker is a whole line, never one inside a fenced code block, so a
/// digest that quotes a marker does not open a block. An END closes only
/// the BEGIN of the same label. A block left open, a block opened inside
/// another, and an END with no BEGIN break the split, and a broken split
/// counts every line as the project's, so no marker hides the rest of the
/// file from the budget.
///
/// SATISFIES docs-format:author-instructions-stay-within-budget
#[must_use]
pub fn split(text: &str) -> Split {
    let mut split = Split::default();
    let mut fence: Option<&str> = None;
    let mut open: Option<(usize, &str)> = None;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let trimmed = line.trim_start();
        if let Some(opened) = fence {
            if trimmed.starts_with(opened) {
                fence = None;
            }
        } else if trimmed.starts_with("```") {
            fence = Some("```");
        } else if trimmed.starts_with("~~~") {
            fence = Some("~~~");
        } else if let Some(label) = marker(line, "BEGIN") {
            if open.is_some() {
                split.broken = Some((number, "a BEGIN marker opens inside another block"));
                break;
            }
            open = Some((number, label));
        } else if let Some(label) = marker(line, "END") {
            match open {
                Some((begin, opened)) if opened == label => {
                    let length = number - begin + 1;
                    split.blocks += length;
                    if length > BLOCK_CAP {
                        split.oversized.push((begin, length));
                    }
                    open = None;
                }
                _ => {
                    split.broken = Some((number, "an END marker closes no BEGIN of its label"));
                    break;
                }
            }
        }
    }
    if split.broken.is_none()
        && let Some((begin, _)) = open
    {
        split.broken = Some((begin, "a BEGIN marker has no END"));
    }
    let total = line_count(text);
    if split.broken.is_some() {
        split.blocks = 0;
        split.oversized.clear();
    }
    split.project = total.saturating_sub(split.blocks);
    split
}

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::AuthorInstructionsStayWithinBudget];

const RULE: RuleId = RuleId::AuthorInstructionsStayWithinBudget;

/// Measure every agent digest: one `lines` count per file.
///
/// # Errors
///
/// [`GateError::Io`] when a digest cannot be read.
pub fn measure(ctx: &GateCtx) -> Result<Vec<Measurement>, GateError> {
    let mut measurements = Vec::new();
    for file in walk_files(ctx) {
        if file.file_name() != Some(AGENTS_DIGEST_PATH) {
            continue;
        }
        let cap = if file.as_str() == format!("./{AGENTS_DIGEST_PATH}") {
            100
        } else {
            150
        };
        let lines = split(&read_text(ctx, &file)?).project;
        measurements.push(Measurement::count(
            GateId::AgentsDigestSize,
            file.as_str(),
            "lines",
            lines,
            cap,
        ));
    }
    Ok(measurements)
}

/// Judge every agent digest in the repository.
///
/// # Errors
///
/// [`GateError::Io`] when a digest cannot be read, and [`GateError::Debt`]
/// when the debt file cannot be trusted.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    if !ctx.path(AGENTS_DIGEST_PATH).is_file() {
        return Ok(vec![Violation::Layout(format!(
            "no root {AGENTS_DIGEST_PATH}"
        ))]);
    }
    let debt = budget::read_debt(ctx)?;
    let measurements = measure(ctx)?;
    let mut splits = Vec::new();
    for measurement in &measurements {
        let text = read_text(ctx, camino::Utf8Path::new(&measurement.path))?;
        splits.push((measurement.path.clone(), split(&text)));
    }
    let blocks_of = |path: &str| {
        splits
            .iter()
            .find(|(held, _)| held == path)
            .map_or(0, |(_, split)| split.blocks)
    };
    let mut violations = Vec::new();
    for (path, split) in &splits {
        if let Some((line, reason)) = split.broken {
            violations.push(Violation::Finding(Finding::on_line(
                RULE,
                path.as_str(),
                line,
                format!("{reason}, so every line counts as the project's"),
            )));
        }
        for (line, length) in &split.oversized {
            violations.push(Violation::Finding(Finding::on_line(
                RULE,
                path.as_str(),
                *line,
                format!("a managed block of {length} lines, cap is {BLOCK_CAP}"),
            )));
        }
    }
    violations.extend(budget::judge(
        &debt,
        GateId::AgentsDigestSize,
        RULE,
        &measurements,
        |m| {
            let crate::domain::debt::Measured::Count { value, budget } = m.value else {
                return Violation::Finding(Finding::on_file(RULE, m.path.as_str(), ""));
            };
            Violation::Finding(Finding::on_file(
                RULE,
                m.path.as_str(),
                format!(
                    "{value} lines of the project's own, budget is {budget}; the managed blocks hold {} more",
                    blocks_of(&m.path)
                ),
            ))
        },
    ));
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &tempfile::TempDir, path: &str, lines: usize) {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "line\n".repeat(lines)).unwrap();
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
    fn accepts_digests_within_budget() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "AGENTS.md", 100);
        write(&dir, "method/AGENTS.md", 150);
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn rejects_a_root_digest_over_its_budget() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "AGENTS.md", 101);
        let out = run_in(&dir);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(
            out[0]
                .starts_with("FAIL docs-format:author-instructions-stay-within-budget ./AGENTS.md"),
            "{out:?}"
        );
        assert!(out[0].contains("101 lines of the project's own"), "{out:?}");
    }

    /// A managed block of `inner` lines between its markers.
    fn block(label: &str, inner: usize) -> String {
        format!(
            "<!-- BEGIN {label} -->\n{}<!-- END {label} -->\n",
            "managed\n".repeat(inner)
        )
    }

    fn write_text(dir: &tempfile::TempDir, path: &str, text: &str) {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// VERIFIES docs-format:author-instructions-stay-within-budget
    #[test]
    fn the_budget_counts_only_the_lines_outside_the_managed_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let own = |lines: usize| "line\n".repeat(lines);
        let text = format!(
            "{}{}{}{}",
            own(60),
            block("release-kit", 15),
            own(40),
            block("spec-driven-docs docs", 13)
        );
        write_text(&dir, "AGENTS.md", &text);
        assert!(run_in(&dir).is_empty(), "{:?}", run_in(&dir));
        assert_eq!(
            split(&text),
            Split {
                project: 100,
                blocks: 32,
                ..Split::default()
            }
        );

        write_text(&dir, "AGENTS.md", &format!("{text}line\n"));
        let out = run_in(&dir);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(
            out[0].contains(
                "101 lines of the project's own, budget is 100; the managed blocks hold 32 more"
            ),
            "{out:?}"
        );
    }

    /// VERIFIES docs-format:author-instructions-stay-within-budget
    #[test]
    fn a_marker_that_does_not_pair_fails_and_hides_nothing() {
        let open = format!("line\n<!-- BEGIN release-kit -->\n{}", "line\n".repeat(99));
        assert_eq!(split(&open).broken, Some((2, "a BEGIN marker has no END")));
        assert_eq!(split(&open).project, 101);

        let nested = format!("<!-- BEGIN a -->\n{}<!-- END a -->\n", block("b", 1));
        assert_eq!(
            split(&nested).broken,
            Some((2, "a BEGIN marker opens inside another block"))
        );
        let mismatched = "<!-- BEGIN a -->\nx\n<!-- END b -->\n";
        assert_eq!(
            split(mismatched).broken,
            Some((3, "an END marker closes no BEGIN of its label"))
        );
        assert!(split("<!-- END a -->\n").broken.is_some());

        let dir = tempfile::tempdir().unwrap();
        write_text(&dir, "AGENTS.md", &open);
        let out = run_in(&dir);
        assert!(
            out.iter()
                .any(|line| line.contains(":2") && line.contains("has no END")),
            "{out:?}"
        );
        assert!(
            out.iter()
                .any(|line| line.contains("101 lines of the project's own")),
            "{out:?}"
        );
    }

    /// VERIFIES docs-format:author-instructions-stay-within-budget
    #[test]
    fn a_managed_block_over_its_cap_fails() {
        let dir = tempfile::tempdir().unwrap();
        write_text(&dir, "AGENTS.md", &format!("line\n{}", block("notes", 19)));
        let out = run_in(&dir);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(
            out[0].contains("a managed block of 21 lines, cap is 20"),
            "{out:?}"
        );
        write_text(&dir, "AGENTS.md", &format!("line\n{}", block("notes", 18)));
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn a_marker_inside_a_fence_or_inside_a_line_opens_nothing() {
        let text = "```markdown\n<!-- BEGIN release-kit -->\n```\nSee `<!-- BEGIN x -->` here.\n~~~\n<!-- END x -->\n~~~\n";
        assert_eq!(
            split(text),
            Split {
                project: 7,
                ..Split::default()
            }
        );
    }

    #[test]
    fn ignores_vendored_digests() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "AGENTS.md", 1);
        write(&dir, "node_modules/pkg/AGENTS.md", 400);
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn a_missing_root_digest_is_a_layout_failure() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(run_in(&dir), vec!["FAIL no root AGENTS.md".to_string()]);
    }

    #[test]
    fn a_recorded_ceiling_carries_an_oversize_digest() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "AGENTS.md", 120);
        std::fs::create_dir_all(dir.path().join(".spec-driven-docs")).unwrap();
        std::fs::write(
            dir.path().join(".spec-driven-docs/debt.yaml"),
            "schema_version: 1\nagents-digest-size:\n  AGENTS.md:\n    lines:\n      ceiling: 120\n",
        )
        .unwrap();
        assert!(run_in(&dir).is_empty());
    }
}
