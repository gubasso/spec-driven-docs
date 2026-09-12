//! What the operator decides, and how they say it.
//!
//! A decision is workflow state the operator owns. The planner offers it,
//! the operator answers it by re-planning with `--set`, and the answer
//! joins the fingerprint, so an approval binds to the answers it was given
//! with. Nothing here guesses a default: a decision the operator has not
//! made is a decision the plan is waiting on.
//!
//! Decisions form an acyclic graph. Until an upstream decision is
//! selected, the planner omits the findings, decisions, and operations
//! that depend on it, rather than computing them against a value nobody
//! chose.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// What shape an answer takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AnswerSchema {
    /// One of a closed set of choice identifiers.
    Choice {
        /// Every identifier the operator may give.
        choices: Vec<Choice>,
    },
    /// A choice identifier, or a prefixed value the operator supplies.
    ChoiceOrValue {
        /// Every plain identifier the operator may give.
        choices: Vec<Choice>,
        /// Every prefix that introduces an operator-supplied value.
        prefixes: Vec<String>,
    },
}

/// One answer the operator may give, and what it means.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    /// The identifier, as `--set` spells it.
    pub id: String,
    /// What choosing it does.
    pub consequence: String,
}

/// One question the plan is waiting on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// A stable identifier, as `--set` spells it.
    pub id: String,
    /// The question, in one sentence.
    pub question: String,
    /// What an answer may look like.
    pub schema: AnswerSchema,
    /// Decisions that must be answered before this one is offered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// The answer the operator gave, where they gave one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<String>,
}

impl Decision {
    /// Whether an answer is one this decision accepts.
    #[must_use]
    pub fn accepts(&self, answer: &str) -> bool {
        match &self.schema {
            AnswerSchema::Choice { choices } => choices.iter().any(|choice| choice.id == answer),
            AnswerSchema::ChoiceOrValue { choices, prefixes } => {
                choices.iter().any(|choice| choice.id == answer)
                    || prefixes.iter().any(|prefix| {
                        answer
                            .strip_prefix(prefix.as_str())
                            .is_some_and(|rest| !rest.is_empty())
                    })
            }
        }
    }
}

/// The identifiers the planner offers.
pub mod id {
    /// Which profile a first landing takes.
    pub const PROFILE: &str = "profile";
    /// Where the planning tool writes its entry documents.
    pub const PLAN_ZONE: &str = "plan-zone";
    /// Where material that is not a statement yet is staged.
    pub const DOCS_SCRATCH: &str = "docs-scratch";
    /// Which writing source the project selects.
    pub const WRITING_STYLE: &str = "writing-style";
    /// Whether a migration sweeps the whole corpus.
    pub const MIGRATION_SCOPE: &str = "migration-scope";
    /// Whether the inherited violations are recorded as debt.
    pub const DEBT_BASELINE: &str = "debt-baseline";
    /// Whether an absent sentinel is appended to the project's specs.
    pub const RECONCILE_POLICY: &str = "reconcile-policy";
    /// Whether a yanked release is an acceptable destination.
    pub const ACCEPT_YANKED: &str = "accept-yanked-release";
}

/// An answer the planner cannot take.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AnswerError {
    /// The argument is not `<id>=<answer>`.
    #[error("--set takes <decision-id>=<answer>; '{0}' has no '='")]
    Malformed(String),

    /// The same decision was answered twice.
    #[error("--set {0} was given twice; one decision takes one answer")]
    Duplicate(String),

    /// No decision carries that identifier.
    #[error("--set {0}: this plan offers no decision with that id")]
    Unknown(String),

    /// The decision does not accept that answer.
    #[error("--set {id}={answer}: {id} does not offer that answer")]
    Rejected {
        /// The decision.
        id: String,
        /// What was given.
        answer: String,
    },
}

/// What the operator selected, by decision identifier.
pub type Selections = BTreeMap<String, String>;

/// Read repeatable `--set` arguments into selections.
///
/// Splitting on the first `=` is deliberate: a value may carry one, and a
/// path frequently does.
///
/// # Errors
///
/// [`AnswerError`] for an argument that is not a pair and for a decision
/// answered twice. Whether the decision exists and accepts the answer is
/// [`validate`], because that needs the plan the answers are for.
pub fn parse(arguments: &[String]) -> Result<Selections, AnswerError> {
    let mut selections = Selections::new();
    for argument in arguments {
        let (id, answer) = argument
            .split_once('=')
            .ok_or_else(|| AnswerError::Malformed(argument.clone()))?;
        if id.is_empty() {
            return Err(AnswerError::Malformed(argument.clone()));
        }
        if selections.contains_key(id) {
            return Err(AnswerError::Duplicate(id.to_string()));
        }
        selections.insert(id.to_string(), answer.trim().to_string());
    }
    Ok(selections)
}

/// Check every selection against the decisions a plan offers.
///
/// # Errors
///
/// [`AnswerError::Unknown`] for an identifier the plan does not offer, and
/// [`AnswerError::Rejected`] for an answer it does not accept. A stale
/// answer is one of those two: a decision the plan stopped offering, or an
/// answer it stopped accepting.
pub fn validate(offered: &[Decision], selections: &Selections) -> Result<(), AnswerError> {
    for (id, answer) in selections {
        let decision = offered
            .iter()
            .find(|decision| &decision.id == id)
            .ok_or_else(|| AnswerError::Unknown(id.clone()))?;
        if !decision.accepts(answer) {
            return Err(AnswerError::Rejected {
                id: id.clone(),
                answer: answer.clone(),
            });
        }
    }
    Ok(())
}

/// Whether the decisions form an acyclic graph.
///
/// # Errors
///
/// The identifier of a decision that is part of a cycle or that names a
/// prerequisite nothing offers.
pub fn acyclic(decisions: &[Decision]) -> Result<(), String> {
    for decision in decisions {
        for needed in &decision.depends_on {
            if !decisions.iter().any(|other| &other.id == needed) {
                return Err(format!(
                    "{} depends on {needed}, which this plan does not offer",
                    decision.id
                ));
            }
        }
    }
    let mut settled: Vec<&str> = Vec::new();
    while settled.len() < decisions.len() {
        let before = settled.len();
        for decision in decisions {
            if settled.contains(&decision.id.as_str()) {
                continue;
            }
            if decision
                .depends_on
                .iter()
                .all(|needed| settled.contains(&needed.as_str()))
            {
                settled.push(&decision.id);
            }
        }
        if settled.len() == before {
            let stuck: Vec<&str> = decisions
                .iter()
                .map(|decision| decision.id.as_str())
                .filter(|id| !settled.contains(id))
                .collect();
            return Err(format!(
                "these decisions form a cycle: {}",
                stuck.join(", ")
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn choice(id: &str) -> Choice {
        Choice {
            id: id.to_string(),
            consequence: format!("it does {id}"),
        }
    }

    fn scope() -> Decision {
        Decision {
            id: id::MIGRATION_SCOPE.to_string(),
            question: "how much of the corpus moves?".to_string(),
            schema: AnswerSchema::Choice {
                choices: vec![choice("sweep"), choice("incremental")],
            },
            depends_on: Vec::new(),
            selected: None,
        }
    }

    fn zone() -> Decision {
        Decision {
            id: id::PLAN_ZONE.to_string(),
            question: "where does the planning tool write?".to_string(),
            schema: AnswerSchema::ChoiceOrValue {
                choices: vec![choice("env"), choice("none")],
                prefixes: vec!["project:".to_string(), "untracked:".to_string()],
            },
            depends_on: vec![id::PROFILE.to_string()],
            selected: None,
        }
    }

    fn profile() -> Decision {
        Decision {
            id: id::PROFILE.to_string(),
            question: "which profile?".to_string(),
            schema: AnswerSchema::Choice {
                choices: vec![choice("codebase"), choice("knowledge-base")],
            },
            depends_on: Vec::new(),
            selected: None,
        }
    }

    #[test]
    fn a_closed_choice_takes_only_its_own_identifiers() {
        let held = scope();
        assert!(held.accepts("sweep"));
        assert!(!held.accepts("Sweep"));
        assert!(!held.accepts("project:docs/plan"));
    }

    #[test]
    fn a_parameterized_answer_takes_a_prefix_with_something_after_it() {
        let held = zone();
        assert!(held.accepts("env"));
        assert!(held.accepts("project:docs/plan"));
        assert!(held.accepts("untracked:.plans"));
        assert!(!held.accepts("project:"));
        assert!(!held.accepts("elsewhere"));
    }

    #[test]
    fn a_selection_splits_on_the_first_equals_so_a_value_may_carry_one() {
        let held = parse(&["plan-zone=project:docs/plan=1".to_string()]).unwrap();
        assert_eq!(held["plan-zone"], "project:docs/plan=1");
    }

    #[test]
    fn a_malformed_or_repeated_selection_is_refused() {
        assert_eq!(
            parse(&["nonsense".to_string()]).unwrap_err(),
            AnswerError::Malformed("nonsense".to_string())
        );
        assert!(matches!(
            parse(&["=x".to_string()]).unwrap_err(),
            AnswerError::Malformed(_)
        ));
        assert_eq!(
            parse(&["a=1".to_string(), "a=2".to_string()]).unwrap_err(),
            AnswerError::Duplicate("a".to_string())
        );
    }

    #[test]
    fn an_unknown_or_stale_answer_is_refused_against_the_plan() {
        let offered = vec![scope()];
        let selections = parse(&["migration-scope=sweep".to_string()]).unwrap();
        assert!(validate(&offered, &selections).is_ok());

        let unknown = parse(&["no-such-decision=x".to_string()]).unwrap();
        assert!(matches!(
            validate(&offered, &unknown).unwrap_err(),
            AnswerError::Unknown(_)
        ));

        let stale = parse(&["migration-scope=partial".to_string()]).unwrap();
        assert!(matches!(
            validate(&offered, &stale).unwrap_err(),
            AnswerError::Rejected { .. }
        ));
    }

    #[test]
    fn the_decision_dependency_graph_is_acyclic() {
        assert!(acyclic(&[profile(), zone()]).is_ok());
        // A prerequisite nothing offers is the same defect as a cycle: the
        // decision can never be reached.
        assert!(acyclic(&[zone()]).is_err());

        let mut one = profile();
        let mut two = zone();
        one.depends_on = vec![two.id.clone()];
        two.depends_on = vec![one.id.clone()];
        let error = acyclic(&[one, two]).unwrap_err();
        assert!(error.contains("cycle"), "{error}");
    }
}
