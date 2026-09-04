//! Gate: a known-issue record carries one state.
//!
//! The state says how this project handles the defect, and it is the field
//! every other reading of the record hangs from: a masked record owes a
//! retire condition where a mitigated one owes none. A word outside the
//! vocabulary answers with a handling the method never defined, so the
//! value is judged here and consulted everywhere else.

use crate::domain::rule_id::RuleId;
use crate::gates::ki_record::{STATES, judge};
use crate::gates::{GateCtx, GateResult};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::RecordCarriesOneState];

/// Judge every known-issue record under the resolved roots.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    judge(ctx, args, "state", &STATES, RuleId::RecordCarriesOneState)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::tests_support::ki_fixture_state;

    fn run_on(state: &str) -> Vec<String> {
        let dir = ki_fixture_state(state, "retire_when: release >= 2.0\n");
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_every_state_the_method_defines() {
        for state in STATES {
            assert!(run_on(state).is_empty(), "{state} was rejected");
        }
    }

    #[test]
    fn rejects_a_state_the_method_does_not_define() {
        let out = run_on("closed");
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("FAIL known-issues:a-record-carries-one-state "));
    }
}
