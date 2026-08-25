//! Gate: a record filed upstream carries the body it was filed with.
//!
//! Once `upstream:` names one issue rather than a tracker, the report exists
//! in two places, and only one of them is under review here. What the gate
//! can see is the pairing: a specific upstream reference and a `## Report`
//! section. That the section is the filed text, in the tracker's markup, is
//! held by review.

use std::sync::LazyLock;

use regex::Regex;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::FiledRecordCarriesItsReport];

/// A specific item: an id introduced by `/`, `#` or `=` anywhere in the
/// value. The `=` form is a canonical Bugzilla link's `show_bug.cgi?id=`,
/// and the right edge is unanchored because a citation routinely carries
/// more than the id — a comment anchor, a status note, or a second link.
static FILED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[ \t]*[Uu]pstream:.*[/#=][0-9]+").unwrap_or_else(|_| unreachable!())
});

/// Judge every known-issue record under the resolved roots.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    let mut bad = String::new();
    for record in ki_records(ctx, args) {
        let text = read_text(ctx, &record)?;
        if text.lines().any(|line| FILED.is_match(line))
            && !text.lines().any(|line| line == "## Report")
        {
            bad.push(' ');
            bad.push_str(record.as_str());
        }
    }
    if bad.is_empty() {
        Ok(vec![])
    } else {
        Ok(vec![Violation::Finding(Finding::global(
            RuleId::FiledRecordCarriesItsReport,
            bad.trim_start().to_string(),
        ))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::tests_support::ki_fixture_upstream;

    fn run_on(upstream: &str, body: &str) -> Vec<String> {
        let dir = ki_fixture_upstream(upstream, body);
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn a_tracker_reference_needs_no_report() {
        assert!(run_on("https://example.invalid/issues", "# V\n").is_empty());
    }

    #[test]
    fn a_filed_reference_demands_the_report_section() {
        for upstream in [
            "https://example.invalid/issues/123",
            "https://example.invalid/issues/123 (open)",
            "https://example.invalid/issues/123#c4",
            "https://bugzilla.example/show_bug.cgi?id=123",
        ] {
            let out = run_on(upstream, "# V\n");
            assert_eq!(out.len(), 1, "upstream {upstream:?}");
            assert!(out[0].starts_with("FAIL known-issues:a-filed-record-carries-its-report: "));
        }
    }

    #[test]
    fn a_filed_record_with_its_report_passes() {
        assert!(
            run_on(
                "https://example.invalid/issues/123",
                "# V\n## Report\n```text\nbody\n```\n"
            )
            .is_empty()
        );
    }
}
