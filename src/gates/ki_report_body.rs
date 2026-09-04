//! Gate: a filed known-issue record carries the body it was filed with,
//! and the issue it was filed as.
//!
//! Once `filing:` reads `filed`, the report exists in two places and only
//! one of them is under review here. What the gate can see is the pairing:
//! a named upstream issue and a `## Report` section. That the section is
//! the filed text, in the tracker's markup, is held by review.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::ki_record::{FILINGS, axis};
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, GateResult, Violation, front_matter_values, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::FiledRecordCarriesItsReport];

const RULE: RuleId = RuleId::FiledRecordCarriesItsReport;

/// Judge every known-issue record under the resolved roots.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for record in ki_records(ctx, args)? {
        let text = read_text(ctx, &record)?;
        if axis(&text, "filing", &FILINGS).as_deref() != Some("filed") {
            continue;
        }
        let named = front_matter_values(&text, "upstream")
            .iter()
            .any(|value| !value.is_empty());
        if !named {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                &record,
                "a filed record names no upstream:",
            )));
        }
        if !text.lines().any(|line| line == "## Report") {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                &record,
                "a filed record carries no ## Report section",
            )));
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::tests_support::ki_fixture_filing;

    fn run_on(filing: &str, upstream: &str, body: &str) -> Vec<String> {
        let dir = ki_fixture_filing(filing, upstream, body);
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn an_unfiled_record_needs_no_report() {
        for filing in ["gathering", "ready", "deferred"] {
            assert!(
                run_on(filing, "https://example.invalid/issues", "# V\n").is_empty(),
                "filing {filing:?}"
            );
        }
    }

    #[test]
    fn a_filed_record_demands_the_report_section() {
        let out = run_on("filed", "https://example.invalid/issues/123", "# V\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("FAIL known-issues:a-filed-record-carries-its-report "));
        assert!(out[0].ends_with(": a filed record carries no ## Report section"));
    }

    #[test]
    fn a_filed_record_demands_the_issue_it_was_filed_as() {
        let out = run_on("filed", "", "# V\n## Report\n```text\nbody\n```\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": a filed record names no upstream:"));
    }

    #[test]
    fn a_filed_record_with_its_report_passes() {
        assert!(
            run_on(
                "filed",
                "https://example.invalid/issues/123",
                "# V\n## Report\n```text\nbody\n```\n"
            )
            .is_empty()
        );
    }
}
