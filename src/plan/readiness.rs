//! Whether a plan may be applied, and what stands in the way.
//!
//! Readiness is derived, never asserted. Each precondition carries a
//! requirement, each is evaluated against what was observed, and the plan's
//! readiness is the worst of them. A gap is honest and is not permission: a
//! precondition nobody could evaluate blocks where ownership says it must,
//! and never quietly passes.

use serde::{Deserialize, Serialize};

/// How much one precondition weighs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Requirement {
    /// Worth reporting and never worth stopping for.
    Advisory,
    /// The operator decides, and the plan waits until they have.
    DecisionRequired,
    /// The apply is unsafe until this holds.
    Required,
}

/// What the observation found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Evaluation {
    /// It holds.
    Satisfied,
    /// Nobody could tell, and why.
    NotObserved {
        /// What stopped the observation.
        reason: String,
    },
    /// It does not hold, and why.
    Unsatisfied {
        /// What was found instead.
        reason: String,
    },
}

impl Evaluation {
    /// Whether this evaluation lets an apply proceed.
    #[must_use]
    pub const fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied)
    }
}

/// Whether a plan may be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Readiness {
    /// Every precondition holds. Apply may proceed.
    Ready,
    /// The operator has a decision to make first.
    NeedsDecision,
    /// Something must change in the target or the request first.
    Blocked,
}

/// One thing that must hold before an apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Precondition {
    /// A stable identifier, so a caller can route on it.
    pub id: String,
    /// One sentence a person reads.
    pub statement: String,
    /// How much it weighs.
    pub requirement: Requirement,
    /// What was found.
    pub evaluation: Evaluation,
    /// The decision that resolves it, where one does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<String>,
    /// What the plan cited to evaluate it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
}

impl Precondition {
    /// What this one precondition alone permits.
    #[must_use]
    pub const fn verdict(&self) -> Readiness {
        if self.evaluation.is_satisfied() {
            return Readiness::Ready;
        }
        match self.requirement {
            Requirement::Advisory => Readiness::Ready,
            Requirement::DecisionRequired => Readiness::NeedsDecision,
            Requirement::Required => Readiness::Blocked,
        }
    }
}

/// The worst verdict among the preconditions.
///
/// Worst rather than first, so the order the planner happens to append in
/// cannot decide whether a plan may run.
#[must_use]
pub fn readiness(preconditions: &[Precondition]) -> Readiness {
    preconditions
        .iter()
        .map(Precondition::verdict)
        .max()
        .unwrap_or(Readiness::Ready)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn precondition(requirement: Requirement, evaluation: Evaluation) -> Precondition {
        Precondition {
            id: "one".to_string(),
            statement: "one thing holds".to_string(),
            requirement,
            evaluation,
            resolved_by: None,
            evidence_refs: Vec::new(),
        }
    }

    fn unsatisfied() -> Evaluation {
        Evaluation::Unsatisfied {
            reason: "it does not".to_string(),
        }
    }

    fn not_observed() -> Evaluation {
        Evaluation::NotObserved {
            reason: "nobody could tell".to_string(),
        }
    }

    #[test]
    fn readiness_is_the_worst_precondition() {
        use Requirement::{Advisory, DecisionRequired, Required};

        let cases: &[(Requirement, Evaluation, Readiness)] = &[
            (Advisory, Evaluation::Satisfied, Readiness::Ready),
            (Advisory, unsatisfied(), Readiness::Ready),
            (Advisory, not_observed(), Readiness::Ready),
            (DecisionRequired, Evaluation::Satisfied, Readiness::Ready),
            (DecisionRequired, unsatisfied(), Readiness::NeedsDecision),
            (DecisionRequired, not_observed(), Readiness::NeedsDecision),
            (Required, Evaluation::Satisfied, Readiness::Ready),
            (Required, unsatisfied(), Readiness::Blocked),
            (Required, not_observed(), Readiness::Blocked),
        ];
        for (requirement, evaluation, expected) in cases {
            let held = precondition(*requirement, evaluation.clone());
            assert_eq!(held.verdict(), *expected, "{requirement:?} {evaluation:?}");
        }
    }

    #[test]
    fn one_blocked_precondition_blocks_the_plan_whatever_its_position() {
        let ready = precondition(Requirement::Advisory, Evaluation::Satisfied);
        let waiting = precondition(Requirement::DecisionRequired, unsatisfied());
        let blocked = precondition(Requirement::Required, unsatisfied());
        assert_eq!(readiness(&[]), Readiness::Ready);
        assert_eq!(readiness(std::slice::from_ref(&ready)), Readiness::Ready);
        assert_eq!(
            readiness(&[ready.clone(), waiting.clone()]),
            Readiness::NeedsDecision
        );
        assert_eq!(
            readiness(&[blocked.clone(), ready.clone(), waiting.clone()]),
            Readiness::Blocked
        );
        assert_eq!(readiness(&[waiting, ready, blocked]), Readiness::Blocked);
    }

    #[test]
    fn a_gap_is_honest_and_is_not_permission() {
        // A required precondition nobody could evaluate blocks. Reporting
        // it as satisfied would turn an unread target into a green light.
        let held = precondition(Requirement::Required, not_observed());
        assert_eq!(held.verdict(), Readiness::Blocked);
    }
}
