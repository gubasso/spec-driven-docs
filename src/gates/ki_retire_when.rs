//! Gate: a masked known-issue record carries the condition under which it
//! is removed, and a mitigated one carries none.
//!
//! A mask whose workaround has no exit becomes permanent by default, and
//! the next reader takes it for a design choice. A mitigation is the
//! opposite case: it is part of the design and stays after the upstream
//! fix, so a retire condition on one states an exit that never arrives.
//! The key must carry a value: an empty `retire_when:` states no
//! condition. Whether the condition is a good one is review's business.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::ki_record::{STATES, axis};
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, GateResult, Violation, front_matter_values, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::RecordCarriesItsRetirementCondition];

const RULE: RuleId = RuleId::RecordCarriesItsRetirementCondition;

/// Judge every known-issue record under the resolved roots.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for record in ki_records(ctx, args)? {
        let text = read_text(ctx, &record)?;
        // A record whose state is missing or invalid is `ki-state`'s to
        // report; judging its condition here would name the same defect
        // twice under a rule that does not own it.
        let Some(state) = axis(&text, "state", &STATES) else {
            continue;
        };
        let carries = front_matter_values(&text, "retire_when")
            .iter()
            .any(|value| !value.is_empty());
        let detail = match (state.as_str(), carries) {
            ("masked", false) => "a masked record states no retire_when:",
            ("mitigated", true) => "a mitigated record states a retire_when:",
            _ => continue,
        };
        violations.push(Violation::Finding(Finding::on_file(RULE, &record, detail)));
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::tests_support::ki_fixture_state;

    fn run_on(state: &str, retire_line: &str) -> Vec<String> {
        let dir = ki_fixture_state(state, retire_line);
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_a_masked_record_with_a_condition() {
        assert!(run_on("masked", "retire_when: release >= 2.0\n").is_empty());
    }

    #[test]
    fn accepts_a_mitigated_record_without_one() {
        assert!(run_on("mitigated", "").is_empty());
    }

    #[test]
    fn rejects_a_masked_record_with_a_missing_or_empty_condition() {
        for retire_line in ["retire_when:\n", ""] {
            let out = run_on("masked", retire_line);
            assert_eq!(out.len(), 1, "retire line {retire_line:?}");
            assert!(
                out[0].starts_with("FAIL known-issues:a-record-carries-its-retirement-condition ")
            );
            assert!(out[0].ends_with(": a masked record states no retire_when:"));
        }
    }

    #[test]
    fn rejects_a_mitigated_record_carrying_a_condition() {
        let out = run_on("mitigated", "retire_when: release >= 2.0\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": a mitigated record states a retire_when:"));
    }

    #[test]
    fn leaves_an_unjudgeable_state_to_the_state_gate() {
        assert!(run_on("closed", "").is_empty());
    }
}
