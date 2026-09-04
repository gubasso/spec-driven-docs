//! Gate: a known-issue record carries one filing state.
//!
//! The filing state says where the case stands upstream, which the state
//! axis never answers: a masked workaround says nothing about whether the
//! evidence behind it is ticket-ready. With the field judged, a grep for
//! `filing: ready` answers which cases someone can file today, and
//! `filing: filed` is what binds a record to its report body.

use crate::domain::rule_id::RuleId;
use crate::gates::ki_record::{FILINGS, judge};
use crate::gates::{GateCtx, GateResult};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::RecordCarriesOneFilingState];

/// Judge every known-issue record under the resolved roots.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    judge(
        ctx,
        args,
        "filing",
        &FILINGS,
        RuleId::RecordCarriesOneFilingState,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::tests_support::ki_fixture_filing;

    fn run_on(filing: &str) -> Vec<String> {
        let dir = ki_fixture_filing(
            filing,
            "https://example.invalid/issues/1234",
            "# V\n## Report\nT\n",
        );
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_every_filing_state_the_method_defines() {
        for filing in FILINGS {
            assert!(run_on(filing).is_empty(), "{filing} was rejected");
        }
    }

    #[test]
    fn rejects_a_filing_state_the_method_does_not_define() {
        let out = run_on("open");
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("FAIL known-issues:a-record-carries-one-filing-state "));
        assert!(out[0].ends_with(": filing: open is not one of gathering, ready, filed, deferred"));
    }
}
