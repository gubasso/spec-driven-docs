//! Gate: every known-issue record walks its mechanism step by step.
//!
//! A record that only names the defect is unfalsifiable to everyone but its
//! author. The `## How it works` heading is what the gate can see, matched
//! anchored and case-sensitively so a record that buries the phrase in prose
//! does not clear it; the walkthrough being a run rather than a restatement
//! is held by review.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::RecordWalksTheMechanism];

/// Judge every known-issue record under the resolved roots.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    let mut bad = String::new();
    for record in ki_records(ctx, args) {
        if !read_text(ctx, &record)?
            .lines()
            .any(|line| line == "## How it works")
        {
            bad.push(' ');
            bad.push_str(record.as_str());
        }
    }
    if bad.is_empty() {
        Ok(vec![])
    } else {
        Ok(vec![Violation::Finding(Finding::global(
            RuleId::RecordWalksTheMechanism,
            bad.trim_start().to_string(),
        ))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::tests_support::ki_fixture_body;

    #[test]
    fn accepts_the_anchored_heading() {
        let dir = ki_fixture_body("# Vendor issue\n## How it works\nRun.\n");
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        assert!(run(&ctx, &[]).unwrap().is_empty());
    }

    #[test]
    fn rejects_a_record_without_the_walkthrough() {
        let dir = ki_fixture_body("# Vendor issue\n## Mechanism\nRun.\n");
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        let out: Vec<String> = run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("FAIL known-issues:a-record-walks-the-mechanism: "));
    }
}
