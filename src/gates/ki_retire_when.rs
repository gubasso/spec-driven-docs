//! Gate: every known-issue record carries the condition under which it is
//! removed.
//!
//! A record whose workaround has no exit becomes permanent by default, and
//! the next reader takes it for a design choice. The key must carry a value:
//! an empty `retire_when:` states no condition. Whether the condition is a
//! good one is review's business.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::RecordCarriesItsRetirementCondition];

/// Judge every known-issue record under the resolved roots.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    let mut bad = String::new();
    for record in ki_records(ctx, args) {
        let carries = read_text(ctx, &record)?.lines().any(|line| {
            line.strip_prefix("retire_when:")
                .is_some_and(|value| !value.trim().is_empty())
        });
        if !carries {
            bad.push(' ');
            bad.push_str(record.as_str());
        }
    }
    if bad.is_empty() {
        Ok(vec![])
    } else {
        Ok(vec![Violation::Finding(Finding::global(
            RuleId::RecordCarriesItsRetirementCondition,
            bad.trim_start().to_string(),
        ))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::tests_support::ki_fixture;

    #[test]
    fn accepts_a_record_with_a_condition() {
        let dir = ki_fixture("retire_when: release >= 2.0\n");
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        assert!(run(&ctx, &[]).unwrap().is_empty());
    }

    #[test]
    fn rejects_a_missing_or_empty_condition() {
        for frontmatter in ["retire_when:\n", ""] {
            let dir = ki_fixture(frontmatter);
            let ctx = GateCtx::new(dir.path().to_str().unwrap());
            let out: Vec<String> = run(&ctx, &[])
                .unwrap()
                .iter()
                .map(ToString::to_string)
                .collect();
            assert_eq!(out.len(), 1, "frontmatter {frontmatter:?}");
            assert!(
                out[0].starts_with("FAIL known-issues:a-record-carries-its-retirement-condition: ")
            );
            assert!(out[0].contains("KI-vendor.md"));
        }
    }
}
