//! What every budget gate does once it has measured: read the debt, judge
//! each measurement against its ceiling or its budget, and report the
//! entries the measurements no longer support.
//!
//! The four budget gates measure different things and print different
//! findings for a fresh violation, so each keeps its own message for that
//! case and hands it in as a closure. The debt comparison is one
//! implementation here, so no gate can honour a ceiling differently from
//! another. A gate never writes: a stale entry is a failure naming
//! `sdd debt tighten --apply`, because a stale ceiling that only warned would
//! permit regrowth up to it.

use crate::domain::debt::{Debt, DebtError, Measured, Measurement, Recorded, dimensions};
use crate::domain::finding::Finding;
use crate::domain::gate_id::GateId;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateError, Violation};

const TIGHTEN: &str = "run 'sdd debt tighten --apply'";

/// The debt the instance at this context's root carries.
///
/// # Errors
///
/// [`GateError::Debt`] when the file cannot be trusted or both formats are
/// present. A gate that cannot read its debt cannot run: a silent empty
/// debt would turn a red commit green.
pub(crate) fn read_debt(ctx: &GateCtx) -> Result<Debt, GateError> {
    Debt::read(&ctx.repo_root).map_err(GateError::Debt)
}

fn label(gate: GateId, dimension: &str) -> &'static str {
    dimensions(gate)
        .iter()
        .find(|spec| spec.name == dimension)
        .map_or("", |spec| spec.label)
}

/// Judge every measurement, and every recorded entry the measurements do
/// not support.
///
/// `fresh` renders the finding for a measurement that violates its budget
/// with no entry recorded, which is the message the gate printed before
/// debt existed and the one its tests hold.
pub(crate) fn judge(
    debt: &Debt,
    gate: GateId,
    rule: RuleId,
    measurements: &[Measurement],
    fresh: impl Fn(&Measurement) -> Violation,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for measurement in measurements {
        let recorded = debt.recorded(gate, &measurement.path, measurement.dimension);
        let noun = label(gate, measurement.dimension);
        let stale = |reason: String| {
            Violation::Finding(Finding::on_file(
                rule,
                measurement.path.as_str(),
                format!("{reason}; {TIGHTEN}"),
            ))
        };
        match (recorded, measurement.value) {
            (None, _) => {
                if measurement.violates() {
                    violations.push(fresh(measurement));
                }
            }
            (Some(Recorded::Ceiling(ceiling)), Measured::Count { value, budget }) => {
                if value <= budget {
                    violations.push(stale(format!(
                        "{value} {noun} is within the budget of {budget} and a ceiling of {ceiling} is recorded"
                    )));
                } else if value > ceiling {
                    violations.push(Violation::Finding(Finding::on_file(
                        rule,
                        measurement.path.as_str(),
                        format!("{value} {noun}, recorded ceiling is {ceiling}"),
                    )));
                } else if value < ceiling {
                    violations.push(stale(format!(
                        "{value} {noun} is below the recorded ceiling of {ceiling}"
                    )));
                }
            }
            (Some(Recorded::Exception), Measured::Flag(holds)) => {
                if !holds {
                    violations.push(stale(format!(
                        "the recorded exception for {noun} is corrected"
                    )));
                }
            }
            (Some(Recorded::Ceiling(_)), Measured::Flag(_))
            | (Some(Recorded::Exception), Measured::Count { .. }) => {
                violations.push(stale(format!(
                    "the recorded entry for {noun} is not of the kind this gate measures"
                )));
            }
        }
    }
    // An entry the gate did not measure is a path it no longer judges:
    // deleted, moved, or excluded. It is dead weight the ratchet removes.
    for (path, dimension, _) in debt.recorded_for(gate) {
        let measured = measurements
            .iter()
            .any(|m| crate::domain::debt::normalize(&m.path) == path && m.dimension == dimension);
        if !measured {
            violations.push(Violation::Finding(Finding::on_file(
                rule,
                path.as_str(),
                format!(
                    "a {} entry is recorded and this gate measures no such path; {TIGHTEN}",
                    label(gate, dimension)
                ),
            )));
        }
    }
    violations
}

impl From<DebtError> for GateError {
    fn from(error: DebtError) -> Self {
        Self::Debt(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh(m: &Measurement) -> Violation {
        Violation::Finding(Finding::on_file(
            RuleId::ChapterStaysWithinLineCap,
            m.path.as_str(),
            "fresh",
        ))
    }

    fn debt(ceiling: usize) -> Debt {
        let mut debt = Debt::default();
        debt.record(
            GateId::ChapterSizeCap,
            "method/a.md",
            "lines",
            Recorded::Ceiling(ceiling),
        );
        debt
    }

    fn lines(value: usize) -> Vec<Measurement> {
        vec![Measurement::count(
            GateId::ChapterSizeCap,
            "./method/a.md",
            "lines",
            value,
            200,
        )]
    }

    fn judged(debt: &Debt, measurements: &[Measurement]) -> Vec<String> {
        judge(
            debt,
            GateId::ChapterSizeCap,
            RuleId::ChapterStaysWithinLineCap,
            measurements,
            fresh,
        )
        .iter()
        .map(ToString::to_string)
        .collect()
    }

    #[test]
    fn a_measurement_at_its_ceiling_passes() {
        assert!(judged(&debt(300), &lines(300)).is_empty());
    }

    #[test]
    fn a_measurement_above_its_ceiling_fails_citing_the_budget_rule() {
        let out = judged(&debt(300), &lines(301));
        assert_eq!(
            out,
            vec![
                "FAIL docs-format:chapter-stays-within-200-lines ./method/a.md: 301 lines, recorded ceiling is 300"
                    .to_string()
            ]
        );
    }

    #[test]
    fn a_measurement_below_its_ceiling_fails_naming_tighten() {
        let out = judged(&debt(300), &lines(250));
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("below the recorded ceiling of 300"));
        assert!(out[0].contains("sdd debt tighten --apply"));
    }

    #[test]
    fn a_measurement_within_the_budget_with_an_entry_fails_naming_tighten() {
        let out = judged(&debt(300), &lines(150));
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("within the budget"));
        assert!(out[0].contains("sdd debt tighten --apply"));
    }

    #[test]
    fn a_fresh_violation_prints_the_gates_own_message() {
        assert_eq!(
            judged(&Debt::default(), &lines(201)),
            vec![
                "FAIL docs-format:chapter-stays-within-200-lines ./method/a.md: fresh".to_string()
            ]
        );
        assert!(judged(&Debt::default(), &lines(200)).is_empty());
    }

    #[test]
    fn an_entry_the_gate_did_not_measure_fails_naming_tighten() {
        let out = judged(&debt(300), &[]);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("method/a.md"));
        assert!(out[0].contains("measures no such path"));
    }
}
