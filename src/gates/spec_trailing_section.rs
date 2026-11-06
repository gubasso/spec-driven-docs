//! Gate: after its requirements, a spec carries at most one section,
//! `## Unenforced`, whose table lists rules of that same spec.
//!
//! The lint shape holds the order up to `## Requirements`, and its trailing
//! `+` admits any later heading, so this gate reads what follows. A heading
//! before the requirements is the lint shape's business and is ignored here.
//! The walk is fence-aware: a heading inside a fenced block is text.

use std::collections::BTreeSet;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[
    RuleId::SpecFollowsTheHeadingShape,
    RuleId::UnenforcedRulesAreDeclared,
];

const SHAPE: RuleId = RuleId::SpecFollowsTheHeadingShape;
const DECLARED: RuleId = RuleId::UnenforcedRulesAreDeclared;
const ALLOWED: &str = "## Unenforced";
const HEADER: [&str; 2] = ["Rule", "Reviewer confirms"];

/// Where a trailing heading other than the allowed one moves.
fn resolution(heading: &str) -> &'static str {
    let title = heading.trim_start_matches('#').trim().to_ascii_lowercase();
    if title.starts_with("unenforced") {
        "rename it to `## Unenforced`"
    } else if title.starts_with("diagnostic") {
        "move the messages to reference"
    } else {
        "move it to reference, explanation, or a rule"
    }
}

/// The rule IDs this spec's own requirement headings declare.
fn declared_ids(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| line.strip_prefix("### `"))
        .filter_map(|rest| rest.split_once('`'))
        .map(|(id, _)| id.to_string())
        .collect()
}

fn cells(row: &str) -> Vec<String> {
    let inner = row.trim().trim_start_matches('|').trim_end_matches('|');
    inner
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn is_separator(row: &str) -> bool {
    cells(row)
        .iter()
        .all(|cell| !cell.is_empty() && cell.chars().all(|c| matches!(c, '-' | ':' | ' ')))
}

/// The one rule ID a first cell names as inline code, if it names one.
fn rule_id_cell(cell: &str) -> Option<&str> {
    let id = cell.strip_prefix('`')?.strip_suffix('`')?;
    let (domain, rule) = id.split_once(':')?;
    let slug = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    (slug(domain) && slug(rule)).then_some(id)
}

/// Judge the table of one `## Unenforced` section, given as numbered lines.
fn judge_table(
    file: &str,
    ids: &BTreeSet<String>,
    heading_line: usize,
    section: &[(usize, &str)],
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut rows = section
        .iter()
        .filter(|(_, line)| line.trim_start().starts_with('|'));
    let Some((line, header)) = rows.next() else {
        findings.push(Finding::on_line(
            DECLARED,
            file,
            heading_line,
            "`## Unenforced` carries no table; list each rule under the header `Rule | Reviewer confirms`",
        ));
        return findings;
    };
    if cells(header) != HEADER {
        findings.push(Finding::on_line(
            DECLARED,
            file,
            *line,
            format!(
                "the table header is `{}`; write `Rule | Reviewer confirms`",
                cells(header).join(" | ")
            ),
        ));
    }
    for (line, row) in rows {
        if is_separator(row) {
            continue;
        }
        let first = cells(row).into_iter().next().unwrap_or_default();
        match rule_id_cell(&first) {
            None => findings.push(Finding::on_line(
                DECLARED,
                file,
                *line,
                format!("the row names `{first}`, not one rule ID as inline code"),
            )),
            Some(id) if !ids.contains(id) => findings.push(Finding::on_line(
                DECLARED,
                file,
                *line,
                format!(
                    "the row names `{id}`, which no requirement of this spec declares; list it in the spec that states it"
                ),
            )),
            Some(_) => {}
        }
    }
    findings
}

/// Every finding over one spec's text.
#[must_use]
pub fn judge(file: &str, text: &str) -> Vec<Finding> {
    let ids = declared_ids(text);
    let mut findings = Vec::new();
    let mut fence: Option<&str> = None;
    let mut after_requirements = false;
    let mut allowed_seen = false;
    // The open `## Unenforced` section: its heading line and its lines.
    let mut open: Option<(usize, Vec<(usize, &str)>)> = None;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let trimmed = line.trim_start();
        if let Some(marker) = fence {
            if trimmed.starts_with(marker) {
                fence = None;
            }
            continue;
        }
        if trimmed.starts_with("```") {
            fence = Some("```");
            continue;
        }
        if trimmed.starts_with("~~~") {
            fence = Some("~~~");
            continue;
        }
        let level_two = line.starts_with("## ");
        if !after_requirements {
            after_requirements = level_two && line.trim_end() == "## Requirements";
            continue;
        }
        if level_two {
            if let Some((heading, section)) = open.take() {
                findings.extend(judge_table(file, &ids, heading, &section));
            }
            if line.trim_end() == ALLOWED && !allowed_seen {
                allowed_seen = true;
                open = Some((number, Vec::new()));
            } else if line.trim_end() == ALLOWED {
                findings.push(Finding::on_line(
                    SHAPE,
                    file,
                    number,
                    "a second `## Unenforced`; merge its rows into the first",
                ));
            } else {
                findings.push(Finding::on_line(
                    SHAPE,
                    file,
                    number,
                    format!(
                        "`{}` follows the requirements; {}",
                        line.trim_end(),
                        resolution(line)
                    ),
                ));
            }
        } else if let Some((_, section)) = open.as_mut() {
            section.push((number, line));
        }
    }
    if let Some((heading, section)) = open {
        findings.extend(judge_table(file, &ids, heading, &section));
    }
    findings
}

/// Judge every file pre-commit passed.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a file cannot be read.
pub fn run(ctx: &GateCtx, files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for file in files {
        let text = read_text(ctx, file)?;
        violations.extend(judge(file, &text).into_iter().map(Violation::Finding));
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = "# Sample Specification\n\n## Purpose\n\nRules.\n\n## Requirements\n\n### `sample:works` — Sample works\n\nThe sample MUST work.\n\n#### Scenario: Run\n\n- GIVEN input\n- WHEN run\n- THEN output\n\nVerify: reviewer confirms it works\n";

    fn with(tail: &str) -> String {
        format!("{SPEC}{tail}")
    }

    fn table(header: &str, first: &str) -> String {
        format!("\n## Unenforced\n\n| {header} |\n| --- | --- |\n| {first} | it works |\n")
    }

    fn details(text: &str) -> Vec<String> {
        judge("SPEC-sample.md", text)
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn no_trailing_section_passes() {
        assert!(details(SPEC).is_empty());
    }

    #[test]
    fn one_unenforced_section_of_this_spec_passes() {
        assert!(details(&with(&table("Rule | Reviewer confirms", "`sample:works`"))).is_empty());
    }

    #[test]
    fn an_unenforced_rules_heading_is_told_to_rename() {
        let found = details(&with("\n## Unenforced rules\n\nText.\n"));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("docs-specs:a-spec-follows-the-heading-shape"));
        assert!(found[0].contains("rename it to `## Unenforced`"));
    }

    #[test]
    fn a_diagnostics_section_is_told_to_move_to_reference() {
        let found = details(&with("\n## Diagnostics\n\nText.\n"));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("move the messages to reference"));
    }

    #[test]
    fn any_other_section_is_told_where_it_moves() {
        let found = details(&with("\n## Background\n\nText.\n"));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("SPEC-sample.md:21"));
        assert!(found[0].contains("reference, explanation, or a rule"));
    }

    #[test]
    fn a_second_unenforced_section_is_reported_on_the_second() {
        let one = table("Rule | Reviewer confirms", "`sample:works`");
        let found = details(&with(&format!("{one}{one}")));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("a second `## Unenforced`"));
        assert!(found[0].contains("SPEC-sample.md:27"));
    }

    #[test]
    fn a_heading_inside_a_fence_is_not_a_heading() {
        assert!(details(&with("\n```markdown\n## Background\n```\n")).is_empty());
        assert!(details(&with("\n~~~text\n## Background\n~~~\n")).is_empty());
    }

    #[test]
    fn a_heading_before_the_requirements_is_left_to_the_lint_shape() {
        let text = SPEC.replace("## Purpose", "## Background\n\n## Purpose");
        assert!(details(&text).is_empty());
    }

    #[test]
    fn a_table_with_another_header_is_reported() {
        let found = details(&with(&table(
            "Rule | Why no command decides it",
            "`sample:works`",
        )));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("docs-specs:unenforced-rules-are-declared"));
        assert!(found[0].contains("write `Rule | Reviewer confirms`"));
    }

    #[test]
    fn a_row_naming_another_specs_rule_is_reported() {
        let found = details(&with(&table("Rule | Reviewer confirms", "`other:works`")));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`other:works`, which no requirement of this spec declares"));
    }

    #[test]
    fn a_row_naming_an_undeclared_rule_of_this_spec_is_reported() {
        let found = details(&with(&table(
            "Rule | Reviewer confirms",
            "`sample:missing`",
        )));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`sample:missing`"));
    }

    #[test]
    fn a_row_that_names_no_rule_id_is_reported() {
        let found = details(&with(&table(
            "Rule | Reviewer confirms",
            "One term per concept",
        )));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("not one rule ID"));
    }

    #[test]
    fn an_unenforced_section_without_a_table_is_reported() {
        let found = details(&with("\n## Unenforced\n\nNone.\n"));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("carries no table"));
    }
}
