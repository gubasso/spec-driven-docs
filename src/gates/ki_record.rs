//! What every known-issue gate reads: a record's one-word axes.
//!
//! `state:` and `filing:` are judged the same way against different
//! vocabularies, so the judgement lives here once and each gate supplies
//! its key, its values, and the rule it cites. The gates that only consult
//! an axis, rather than judge it, read [`axis`] and get `None` back where
//! the axis gate already has something to say.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, GateResult, Violation, front_matter_values, read_text};

/// The values `state:` accepts.
pub const STATES: [&str; 4] = ["investigating", "mitigated", "masked", "monitoring"];

/// The values `filing:` accepts.
pub const FILINGS: [&str; 4] = ["gathering", "ready", "filed", "deferred"];

/// The one value a record's axis carries, or `None` where it carries no
/// value, more than one, or a word outside the vocabulary.
#[must_use]
pub fn axis(text: &str, key: &str, allowed: &[&str]) -> Option<String> {
    match front_matter_values(text, key).as_slice() {
        [value] if allowed.contains(&value.as_str()) => Some(value.clone()),
        _ => None,
    }
}

/// Judge one axis across every record under the resolved roots.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record cannot be read.
pub fn judge(
    ctx: &GateCtx,
    args: &[String],
    key: &str,
    allowed: &[&str],
    rule: RuleId,
) -> GateResult {
    let mut violations = Vec::new();
    for record in ki_records(ctx, args)? {
        let text = read_text(ctx, &record)?;
        let values = front_matter_values(&text, key);
        let detail = match values.as_slice() {
            [value] if allowed.contains(&value.as_str()) => continue,
            [] => format!("no {key}: line, expected one of {}", allowed.join(", ")),
            [value] => format!("{key}: {value} is not one of {}", allowed.join(", ")),
            many => format!("{} {key}: lines, expected one", many.len()),
        };
        violations.push(Violation::Finding(Finding::on_file(rule, &record, detail)));
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run_on(state: &str) -> Vec<String> {
        run_on_record(&format!("---\nstate: {state}\n---\n# V\n"))
    }

    fn run_on_record(text: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        let records = dir.path().join("_docs/reference/known-issues");
        std::fs::create_dir_all(&records).unwrap();
        std::fs::write(records.join("KI-vendor.md"), text).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        judge(&ctx, &[], "state", &STATES, RuleId::RecordCarriesOneState)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_a_value_from_the_vocabulary() {
        assert!(run_on("monitoring").is_empty());
    }

    #[test]
    fn rejects_a_word_outside_the_vocabulary() {
        let out = run_on("closed");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("known-issues:a-record-carries-one-state"));
        assert!(out[0].ends_with(
            ": state: closed is not one of investigating, mitigated, masked, monitoring"
        ));
    }

    #[test]
    fn rejects_a_record_that_states_no_axis() {
        let out = run_on_record("---\nupstream: https://example.invalid/issues\n---\n# V\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(
            ": no state: line, expected one of investigating, mitigated, masked, monitoring"
        ));
    }

    #[test]
    fn rejects_a_record_that_states_the_axis_twice() {
        let out = run_on_record("---\nstate: masked\nstate: monitoring\n---\n# V\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": 2 state: lines, expected one"));
    }

    #[test]
    fn axis_reads_the_one_value_and_nothing_else() {
        let text = "---\nstate: masked\n---\nstate: monitoring\n";
        assert_eq!(axis(text, "state", &STATES), Some("masked".to_string()));
        assert_eq!(axis("state: masked\n", "state", &STATES), None);
        assert_eq!(axis("---\nstate: closed\n---\n", "state", &STATES), None);
    }
}
