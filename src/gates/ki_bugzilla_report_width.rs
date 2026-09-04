//! Gate: a Bugzilla report body fits in 79 columns.
//!
//! Bugzilla renders a comment as preformatted plain text and reflows
//! nothing, so a line past that width wraps where Bugzilla chooses — and the
//! aligned table or annotated excerpt carrying the argument does not survive
//! it. The rule is Bugzilla's, not every tracker's: the tracker is
//! recognised from the whole `upstream:` value, case-insensitively, with
//! `show_bug.cgi` counting alongside the product name because a Bugzilla
//! instance is routinely branded something else. The body sits in a fence —
//! text outside one belongs to the markdown formatter, at a width it
//! chooses rather than the tracker's. Width is measured in display columns.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::BugzillaReportBodyFitsReportWidth];

const RULE: RuleId = RuleId::BugzillaReportBodyFitsReportWidth;
const WIDTH: usize = 79;

fn is_bugzilla_reference(line: &str) -> bool {
    let lower = line.to_lowercase();
    let Some(_) = lower.trim_start().strip_prefix("upstream:") else {
        return false;
    };
    ["bugzilla", "show_bug.cgi", "bsc#", "boo#", "bnc#"]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn columns(line: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(line)
}

fn judge(record: &str, text: &str, violations: &mut Vec<Violation>) {
    let mut report = false;
    let mut fence = false;
    let mut bugzilla = false;
    for (number, line) in text.lines().enumerate() {
        if is_bugzilla_reference(line) {
            bugzilla = true;
        }
        if line == "## Report" {
            report = true;
            continue;
        }
        if !fence && line.starts_with("## ") {
            report = false;
        }
        if report && line.starts_with("```") {
            fence = !fence;
            continue;
        }
        if !bugzilla {
            continue;
        }
        if report && fence && columns(line) > WIDTH {
            violations.push(Violation::Finding(Finding::on_line(
                RULE,
                record,
                number + 1,
                format!("{} columns", columns(line)),
            )));
        }
        if report && !fence && !line.is_empty() {
            violations.push(Violation::Finding(Finding::on_line(
                RULE,
                record,
                number + 1,
                "body outside a fence",
            )));
        }
    }
}

/// Judge every known-issue record under the resolved roots.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for record in ki_records(ctx, args)? {
        let text = read_text(ctx, &record)?;
        judge(record.as_str(), &text, &mut violations);
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::tests_support::ki_fixture_upstream;

    const BUGZILLA: &str = "https://bugzilla.example/show_bug.cgi?id=123";

    fn run_on(upstream: &str, body: &str) -> Vec<String> {
        let dir = ki_fixture_upstream(upstream, body);
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn fenced(line: &str) -> String {
        format!("# V\n## How it works\nRun.\n## Report\n```text\n{line}\n```\n")
    }

    #[test]
    fn accepts_a_fitting_fenced_body() {
        assert!(run_on(BUGZILLA, &fenced("body")).is_empty());
    }

    #[test]
    fn rejects_an_over_wide_line_with_its_column_count() {
        let out = run_on(BUGZILLA, &fenced(&"x".repeat(100)));
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("known-issues:a-bugzilla-report-body-fits-in-79-columns"));
        assert!(out[0].ends_with(": 100 columns"));
    }

    #[test]
    fn recognises_a_rebranded_tracker_and_an_indented_capitalised_key() {
        let over = fenced(&"x".repeat(100));
        assert_eq!(
            run_on("https://bugs.kde.org/show_bug.cgi?id=123", &over).len(),
            1
        );

        let dir = ki_fixture_upstream("placeholder", &over);
        let record = dir.path().join("_docs/reference/known-issues/KI-vendor.md");
        let text = std::fs::read_to_string(&record)
            .unwrap()
            .replace("upstream: placeholder", &format!("  Upstream: {BUGZILLA}"));
        std::fs::write(&record, text).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        assert_eq!(run(&ctx, &[]).unwrap().len(), 1);
    }

    #[test]
    fn rejects_body_text_outside_a_fence() {
        let out = run_on(BUGZILLA, "# V\n## Report\nloose text\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": body outside a fence"));
    }

    #[test]
    fn a_non_bugzilla_tracker_is_out_of_scope() {
        assert!(run_on("https://github.com/x/y/issues/1", &fenced(&"x".repeat(100))).is_empty());
    }
}
