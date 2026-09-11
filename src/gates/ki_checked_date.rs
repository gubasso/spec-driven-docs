//! Gate: a record whose handling depends on an upstream state carries the
//! date that state was last confirmed, and no other record carries one.
//!
//! A masked record states the condition that removes its workaround, and a
//! monitoring record states the upstream it watches. Neither says when
//! anyone last looked, so a mask outlives its bug and the next reader takes
//! the workaround for a design choice. The other states describe handling
//! this project owns, where there is no upstream observation to date.
//!
//! The gate judges presence, shape, and a date that is not in the future.
//! It never judges age: an old date over an upstream that has not moved is
//! an accurate record, and failing it teaches people to touch the date
//! rather than to check the condition. Whether the observation is recent
//! enough is review's business.

use jiff::civil::Date;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::ki_record::{STATES, axis};
use crate::gates::paths::ki_records_judged;
use crate::gates::{GateCtx, GateResult, Violation, front_matter_values, read_text};
use crate::services::tracking::today_utc;

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::RecordRecordsItsLastCheck];

const RULE: RuleId = RuleId::RecordRecordsItsLastCheck;

/// The states whose handling waits on something outside this project.
const DATED_STATES: [&str; 2] = ["masked", "monitoring"];

/// The ten-character ISO calendar date, and nothing else.
///
/// `jiff` accepts shapes this key does not want, so the accepted set is
/// stated here rather than delegated.
fn iso_date(value: &str) -> Option<Date> {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return None;
    }
    value.parse::<Date>().ok()
}

fn detail(state: &str, values: &[String], as_of: Date) -> Option<String> {
    let dated = DATED_STATES.contains(&state);
    match (dated, values) {
        (false, []) => None,
        (false, _) => Some(format!("a {state} record states a checked:")),
        (true, []) => Some(format!("a {state} record states no checked:")),
        (true, [value]) => match iso_date(value) {
            None if value.is_empty() => Some(format!("a {state} record states an empty checked:")),
            None => Some(format!("checked: {value} is not an ISO YYYY-MM-DD date")),
            Some(date) if date > as_of => Some(format!("checked: {value} is in the future")),
            Some(_) => None,
        },
        (true, many) => Some(format!("{} checked: lines, expected one", many.len())),
    }
}

fn judge(ctx: &GateCtx, args: &[String], as_of: Date) -> GateResult {
    let mut violations = Vec::new();
    for record in ki_records_judged(ctx, args)? {
        let text = read_text(ctx, &record)?;
        // A record whose state is missing or invalid is `ki-state`'s to
        // report; judging its date here would name the same defect twice
        // under a rule that does not own it.
        let Some(state) = axis(&text, "state", &STATES) else {
            continue;
        };
        let values = front_matter_values(&text, "checked");
        if let Some(detail) = detail(&state, &values, as_of) {
            violations.push(Violation::Finding(Finding::on_file(RULE, &record, detail)));
        }
    }
    Ok(violations)
}

/// Judge every known-issue record under the resolved roots.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    judge(ctx, args, today_utc())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::tests_support::ki_fixture_checked;

    const AS_OF: &str = "2026-06-30";

    fn run_on(state: &str, checked_line: &str) -> Vec<String> {
        let dir = ki_fixture_checked(state, checked_line);
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        judge(&ctx, &[], AS_OF.parse().unwrap())
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_a_masked_record_with_a_past_date() {
        assert!(run_on("masked", "checked: 2026-06-18\n").is_empty());
    }

    #[test]
    fn accepts_a_monitoring_record_dated_today() {
        assert!(run_on("monitoring", "checked: 2026-06-30\n").is_empty());
    }

    #[test]
    fn accepts_an_undated_state_carrying_none() {
        for state in ["investigating", "mitigated"] {
            assert!(run_on(state, "").is_empty(), "state {state}");
        }
    }

    #[test]
    fn rejects_a_dated_state_carrying_none() {
        for state in DATED_STATES {
            let out = run_on(state, "");
            assert_eq!(out.len(), 1, "state {state}");
            assert!(out[0].starts_with("FAIL known-issues:a-record-records-its-last-check "));
            assert!(out[0].ends_with(&format!(": a {state} record states no checked:")));
        }
    }

    #[test]
    fn rejects_an_empty_date() {
        let out = run_on("masked", "checked:\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": a masked record states an empty checked:"));
    }

    #[test]
    fn rejects_a_date_that_is_not_iso() {
        for value in ["2026-6-9", "June 1", "2026-06-18T00:00:00Z", "2026-02-30"] {
            let out = run_on("masked", &format!("checked: {value}\n"));
            assert_eq!(out.len(), 1, "value {value}");
            assert!(out[0].ends_with(&format!(": checked: {value} is not an ISO YYYY-MM-DD date")));
        }
    }

    #[test]
    fn rejects_a_date_in_the_future() {
        let out = run_on("masked", "checked: 2026-07-01\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": checked: 2026-07-01 is in the future"));
    }

    #[test]
    fn rejects_an_undated_state_carrying_one() {
        let out = run_on("mitigated", "checked: 2026-06-18\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": a mitigated record states a checked:"));
    }

    #[test]
    fn rejects_two_dates() {
        let out = run_on("masked", "checked: 2026-06-18\nchecked: 2026-06-19\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": 2 checked: lines, expected one"));
    }

    #[test]
    fn leaves_an_unjudgeable_state_to_the_state_gate() {
        assert!(run_on("closed", "").is_empty());
    }

    #[test]
    fn the_real_clock_accepts_a_past_date() {
        let dir = ki_fixture_checked("masked", "checked: 2020-01-01\n");
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        assert!(run(&ctx, &[]).unwrap().is_empty());
    }
}
