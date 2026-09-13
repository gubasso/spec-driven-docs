//! What a release asks of an instance that takes it.
//!
//! A changelog is written for a reader deciding whether to upgrade. A plan
//! is read by somebody who already decided. The two answer different
//! questions, so the bundle owes the second one as data a plan can filter
//! rather than prose an operator reads whole.
//!
//! One file per release that needs one, never per release. The index is
//! the complete coverage ledger from the capability floor onward, so a
//! missing prose file cannot strand an interval and a missing entry is a
//! release-time failure rather than a silent gap.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::version::CanonVersion;
use crate::plan::decision::{AnswerSchema, Choice, Decision, Selections};
use crate::plan::readiness::{Evaluation, Precondition, Requirement};

/// Where the ledger sits inside a bundle.
pub const INDEX_PATH: &str = "guidance/index.toml";

/// The ledger schema this engine reads.
pub const INDEX_SCHEMA: &str = "sdd.guidance-index/1";

/// The step-file schema this engine reads.
pub const SCHEMA: &str = "sdd.guidance/1";

/// The word an index entry uses when a release asks nothing.
pub const NONE: &str = "none";

/// Guidance this engine cannot read.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum GuidanceError {
    /// The bytes are not the shape they claim.
    #[error("{path} does not parse: {reason}")]
    Malformed {
        /// Which file.
        path: String,
        /// What the parser found.
        reason: String,
    },

    /// The file is written in a schema this engine does not read.
    #[error("{path} declares schema {found}, and this engine reads {expected}")]
    UnknownSchema {
        /// Which file.
        path: String,
        /// What it declares.
        found: String,
        /// What this engine reads.
        expected: String,
    },

    /// The file contradicts itself.
    #[error("{path} is inconsistent: {reason}")]
    Inconsistent {
        /// Which file.
        path: String,
        /// What is wrong.
        reason: String,
    },
}

/// What one release asks, in kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepKind {
    /// An adopted seed the projection now lands.
    SeedAdded,
    /// A rule ID that no longer resolves.
    RuleRetired,
    /// A managed file whose bytes changed.
    ManagedChanged,
    /// A key the project can now set.
    DeclarationKeyAdded,
    /// A gate whose judged set grew.
    GateWidened,
}

/// Who takes one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Actor {
    /// The plan's own operations already carry it.
    Plan,
    /// A person takes it.
    Operator,
}

/// The destination vocabulary a step filters against.
///
/// Closed, because a step naming a destination nothing recognizes cannot
/// be filtered and would reach every target.
pub const DESTINATIONS: &[&str] = &[
    "specs",
    "decisions",
    "reference",
    "guides",
    "declaration",
    "debt",
    "markdownlint",
    "hooks-config",
    "agents-digest",
    "plan-zone",
];

/// One thing a release asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// A slug that stays put for the life of the release.
    pub id: String,
    /// What kind of change it is.
    pub kind: StepKind,
    /// Whether a target that ignores it fails a gate or loses a route.
    pub breaking: bool,
    /// Which destinations it applies to.
    pub destinations: Vec<String>,
    /// Who takes it.
    pub actor: Actor,
    /// The prose body, relative to the release's own directory.
    pub text: String,
}

impl Step {
    /// The decision identifier this step carries.
    ///
    /// Derived from the release and the step's own slug, never from
    /// display text, so a reworded body moves no plan's identity.
    #[must_use]
    pub fn decision_id(&self, release: &CanonVersion) -> String {
        format!("guidance:{release}:{}", self.id)
    }
}

/// What one release asks, whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guidance {
    /// Always [`SCHEMA`] once parsed.
    pub schema: String,
    /// The release this describes.
    pub release: CanonVersion,
    /// Every step, in the order the release wrote them.
    pub steps: Vec<Step>,
}

impl Guidance {
    /// Read one release's steps.
    ///
    /// # Errors
    ///
    /// [`GuidanceError`] for bytes that do not parse, a schema this engine
    /// does not read, a duplicate step identifier, an empty or unknown
    /// destination, or a body the bundle does not carry.
    pub fn parse(path: &str, bytes: &[u8], bodies: &[String]) -> Result<Self, GuidanceError> {
        let malformed = |reason: String| GuidanceError::Malformed {
            path: path.to_string(),
            reason,
        };
        let text = std::str::from_utf8(bytes).map_err(|source| malformed(source.to_string()))?;
        let held: Self = toml::from_str(text).map_err(|source| malformed(source.to_string()))?;
        if held.schema != SCHEMA {
            return Err(GuidanceError::UnknownSchema {
                path: path.to_string(),
                found: held.schema,
                expected: SCHEMA.to_string(),
            });
        }
        let inconsistent = |reason: String| GuidanceError::Inconsistent {
            path: path.to_string(),
            reason,
        };
        let mut seen: Vec<&str> = Vec::new();
        for step in &held.steps {
            if seen.contains(&step.id.as_str()) {
                return Err(inconsistent(format!("{} appears twice", step.id)));
            }
            seen.push(&step.id);
            if step.destinations.is_empty() {
                return Err(inconsistent(format!("{} names no destination", step.id)));
            }
            for destination in &step.destinations {
                if !DESTINATIONS.contains(&destination.as_str()) {
                    return Err(inconsistent(format!(
                        "{} names the destination {destination}, which is not one this engine filters against",
                        step.id
                    )));
                }
            }
            let body = format!("guidance/{}/{}", held.release, step.text);
            if !bodies.contains(&body) {
                return Err(inconsistent(format!(
                    "{} names the body {body}, which the bundle does not carry",
                    step.id
                )));
            }
        }
        Ok(held)
    }

    /// Every step that reaches one target's destinations.
    #[must_use]
    pub fn filtered(&self, held: &[String]) -> Vec<&Step> {
        self.steps
            .iter()
            .filter(|step| {
                step.destinations
                    .iter()
                    .any(|destination| held.contains(destination))
            })
            .collect()
    }
}

/// One release's coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexEntry {
    /// Which release.
    pub version: CanonVersion,
    /// The file that carries its steps, or [`NONE`].
    pub guidance: String,
}

/// The complete coverage ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    /// Always [`INDEX_SCHEMA`] once parsed.
    pub schema: String,
    /// The lowest release this ledger covers.
    pub capability_floor: CanonVersion,
    /// One entry per covered release.
    pub releases: Vec<IndexEntry>,
}

impl Index {
    /// Read the ledger.
    ///
    /// # Errors
    ///
    /// [`GuidanceError`] for bytes that do not parse, a schema this engine
    /// does not read, or a release listed twice.
    pub fn parse(bytes: &[u8]) -> Result<Self, GuidanceError> {
        let malformed = |reason: String| GuidanceError::Malformed {
            path: INDEX_PATH.to_string(),
            reason,
        };
        let text = std::str::from_utf8(bytes).map_err(|source| malformed(source.to_string()))?;
        let held: Self = toml::from_str(text).map_err(|source| malformed(source.to_string()))?;
        if held.schema != INDEX_SCHEMA {
            return Err(GuidanceError::UnknownSchema {
                path: INDEX_PATH.to_string(),
                found: held.schema,
                expected: INDEX_SCHEMA.to_string(),
            });
        }
        let mut seen: Vec<CanonVersion> = Vec::new();
        for entry in &held.releases {
            if seen.contains(&entry.version) {
                return Err(GuidanceError::Inconsistent {
                    path: INDEX_PATH.to_string(),
                    reason: format!("{} appears twice", entry.version),
                });
            }
            seen.push(entry.version);
        }
        Ok(held)
    }

    /// What the ledger says about one release.
    #[must_use]
    pub fn entry(&self, version: CanonVersion) -> Option<&IndexEntry> {
        self.releases.iter().find(|entry| entry.version == version)
    }

    /// Every release in the half-open interval, in order.
    #[must_use]
    pub fn interval(&self, from: Option<CanonVersion>, to: CanonVersion) -> Vec<&IndexEntry> {
        let mut found: Vec<&IndexEntry> = self
            .releases
            .iter()
            .filter(|entry| from.is_none_or(|held| entry.version > held) && entry.version <= to)
            .collect();
        found.sort_by_key(|entry| entry.version);
        found
    }
}

/// How much of an interval the ledger covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Coverage {
    /// Every release in the interval has an entry.
    Complete,
    /// The recorded release predates the floor.
    Partial,
}

/// What the ledger says about one interval.
#[must_use]
pub fn coverage(index: &Index, from: Option<CanonVersion>) -> Coverage {
    match from {
        Some(recorded) if recorded < index.capability_floor => Coverage::Partial,
        _ => Coverage::Complete,
    }
}

/// The preconditions and decisions one interval's guidance carries.
#[derive(Debug, Clone, Default)]
pub struct Briefing {
    /// Everything that must hold first.
    pub preconditions: Vec<Precondition>,
    /// Everything the operator accepts knowingly.
    pub decisions: Vec<Decision>,
    /// Every step that reaches this target, as `<release>:<id>`.
    pub applicable: Vec<String>,
    /// How many steps the target's destinations excluded.
    pub excluded: usize,
}

/// Read one interval's guidance against one target.
///
/// A breaking step is a decision the operator accepts with the step's own
/// body in front of them. An additive step is reported and blocks nothing.
/// Partial coverage is a decision rather than a gap, because the operator
/// can still say they know what is missing.
#[must_use]
pub fn brief(
    index: &Index,
    files: &[(CanonVersion, Guidance)],
    from: Option<CanonVersion>,
    destinations: &[String],
    selections: &Selections,
) -> Briefing {
    let mut briefing = Briefing::default();
    for (release, guidance) in files {
        let applicable = guidance.filtered(destinations);
        briefing.excluded += guidance.steps.len() - applicable.len();
        for step in applicable {
            let id = step.decision_id(release);
            briefing.applicable.push(id.clone());
            if !step.breaking {
                continue;
            }
            let selected = selections.get(&id).cloned();
            briefing.decisions.push(Decision {
                question: format!("{release} asks: {}", step.id.replace('-', " ")),
                schema: AnswerSchema::Choice {
                    choices: vec![
                        Choice {
                            id: "accepted".to_string(),
                            consequence: format!(
                                "read guidance/{release}/{} and take the step",
                                step.text
                            ),
                        },
                        Choice {
                            id: "not-applicable".to_string(),
                            consequence: "this target does not carry what the step is about"
                                .to_string(),
                        },
                    ],
                },
                depends_on: Vec::new(),
                selected,
                id,
            });
        }
    }
    if coverage(index, from) == Coverage::Partial {
        let id = "guidance-coverage".to_string();
        let selected = selections.get(&id).cloned();
        briefing.preconditions.push(Precondition {
            id: "guidance-is-covered".to_string(),
            statement: "every release in the interval carries its guidance".to_string(),
            requirement: Requirement::DecisionRequired,
            evaluation: selected.as_ref().map_or_else(
                || Evaluation::NotObserved {
                    reason: format!(
                        "the target records a release below {}, which this engine does not brief",
                        index.capability_floor
                    ),
                },
                |_| Evaluation::Satisfied,
            ),
            resolved_by: Some(id.clone()),
            evidence_refs: vec!["release".to_string()],
        });
        briefing.decisions.push(Decision {
            question: format!(
                "guidance starts at {}; proceed without what came before?",
                index.capability_floor
            ),
            schema: AnswerSchema::Choice {
                choices: vec![
                    Choice {
                        id: "accepted".to_string(),
                        consequence: "the plan proceeds with the guidance it has".to_string(),
                    },
                    Choice {
                        id: "refuse".to_string(),
                        consequence: "nothing lands; read the changelog first".to_string(),
                    },
                ],
            },
            depends_on: Vec::new(),
            selected,
            id,
        });
    }
    for decision in &briefing.decisions {
        if decision.id == "guidance-coverage" {
            continue;
        }
        briefing.preconditions.push(Precondition {
            id: format!("step:{}", decision.id),
            statement: decision.question.clone(),
            requirement: Requirement::DecisionRequired,
            evaluation: decision.selected.as_ref().map_or_else(
                || Evaluation::Unsatisfied {
                    reason: "the operator has not accepted this step".to_string(),
                },
                |_| Evaluation::Satisfied,
            ),
            resolved_by: Some(decision.id.clone()),
            evidence_refs: vec!["release".to_string()],
        });
    }
    briefing
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn version(value: &str) -> CanonVersion {
        value.parse().unwrap()
    }

    fn bodies() -> Vec<String> {
        vec!["guidance/0.7.0/one.md".to_string()]
    }

    const ONE: &str = r#"
schema = "sdd.guidance/1"
release = "0.7.0"

[[steps]]
id = "one"
kind = "rule-retired"
breaking = true
destinations = ["specs"]
actor = "operator"
text = "one.md"
"#;

    #[test]
    fn a_step_declares_every_part_and_parses() {
        let held = Guidance::parse("guidance/0.7.0.toml", ONE.as_bytes(), &bodies()).unwrap();
        assert_eq!(held.release, version("0.7.0"));
        assert_eq!(held.steps[0].kind, StepKind::RuleRetired);
        assert_eq!(held.steps[0].actor, Actor::Operator);
        assert!(held.steps[0].breaking);
    }

    #[test]
    fn a_decision_id_derives_from_the_release_and_the_step_id() {
        let held = Guidance::parse("guidance/0.7.0.toml", ONE.as_bytes(), &bodies()).unwrap();
        assert_eq!(
            held.steps[0].decision_id(&version("0.7.0")),
            "guidance:0.7.0:one"
        );
        // A body rewritten under the same name keeps the identifier: the
        // decision is the step, not the prose that explains it.
        let reworded = ONE.replace("kind = \"rule-retired\"", "kind = \"gate-widened\"");
        let again = Guidance::parse("guidance/0.7.0.toml", reworded.as_bytes(), &bodies()).unwrap();
        assert_eq!(
            again.steps[0].decision_id(&version("0.7.0")),
            held.steps[0].decision_id(&version("0.7.0"))
        );
    }

    #[test]
    fn guidance_refuses_what_it_cannot_filter_or_resolve() {
        let cases = [
            (
                ONE.replace("kind = \"rule-retired\"", "kind = \"invented\""),
                "parse",
            ),
            (
                ONE.replace("destinations = [\"specs\"]", "destinations = []"),
                "destination",
            ),
            (
                ONE.replace("destinations = [\"specs\"]", "destinations = [\"nowhere\"]"),
                "destination",
            ),
            (
                ONE.replace("text = \"one.md\"", "text = \"absent.md\""),
                "body",
            ),
            (format!("{ONE}extra = 1\n"), "parse"),
            (format!("{ONE}{ONE}"), "parse"),
        ];
        for (text, _) in cases {
            assert!(
                Guidance::parse("guidance/0.7.0.toml", text.as_bytes(), &bodies()).is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn a_duplicate_step_identifier_refuses() {
        let doubled = format!(
            "{ONE}\n[[steps]]\nid = \"one\"\nkind = \"seed-added\"\nbreaking = false\ndestinations = [\"specs\"]\nactor = \"plan\"\ntext = \"one.md\"\n"
        );
        let error =
            Guidance::parse("guidance/0.7.0.toml", doubled.as_bytes(), &bodies()).unwrap_err();
        assert!(error.to_string().contains("twice"), "{error}");
    }

    #[test]
    fn a_step_is_filtered_against_the_targets_destinations() {
        let held = Guidance::parse("guidance/0.7.0.toml", ONE.as_bytes(), &bodies()).unwrap();
        assert_eq!(held.filtered(&["specs".to_string()]).len(), 1);
        assert_eq!(held.filtered(&["debt".to_string()]).len(), 0);
    }

    fn index() -> Index {
        Index::parse(
            br#"
schema = "sdd.guidance-index/1"
capability_floor = "0.6.6"

[[releases]]
version = "0.6.6"
guidance = "none"

[[releases]]
version = "0.7.0"
guidance = "0.7.0.toml"

[[releases]]
version = "0.7.1"
guidance = "none"
"#,
        )
        .unwrap()
    }

    #[test]
    fn the_interval_is_derived_from_the_ledger_alone() {
        let held = index();
        let found: Vec<String> = held
            .interval(Some(version("0.6.6")), version("0.7.1"))
            .iter()
            .map(|entry| entry.version.to_string())
            .collect();
        assert_eq!(found, ["0.7.0", "0.7.1"]);
        assert_eq!(held.interval(None, version("0.6.6")).len(), 1);
        assert_eq!(held.entry(version("0.7.0")).unwrap().guidance, "0.7.0.toml");
        assert_eq!(held.entry(version("0.7.1")).unwrap().guidance, NONE);
        assert!(held.entry(version("9.9.9")).is_none());
    }

    #[test]
    fn a_release_below_the_floor_is_partial_coverage() {
        let held = index();
        assert_eq!(coverage(&held, Some(version("0.6.5"))), Coverage::Partial);
        assert_eq!(coverage(&held, Some(version("0.6.6"))), Coverage::Complete);
        assert_eq!(coverage(&held, None), Coverage::Complete);
    }

    #[test]
    fn an_additive_upgrade_needs_no_decision_and_a_breaking_one_does() {
        let additive = ONE.replace("breaking = true", "breaking = false");
        let held = Guidance::parse("guidance/0.7.0.toml", additive.as_bytes(), &bodies()).unwrap();
        let briefing = brief(
            &index(),
            &[(version("0.7.0"), held)],
            Some(version("0.6.6")),
            &["specs".to_string()],
            &Selections::new(),
        );
        assert!(briefing.decisions.is_empty());
        assert_eq!(briefing.applicable, ["guidance:0.7.0:one"]);

        let breaking = Guidance::parse("guidance/0.7.0.toml", ONE.as_bytes(), &bodies()).unwrap();
        let briefing = brief(
            &index(),
            &[(version("0.7.0"), breaking.clone())],
            Some(version("0.6.6")),
            &["specs".to_string()],
            &Selections::new(),
        );
        assert_eq!(briefing.decisions.len(), 1);
        assert_eq!(briefing.preconditions.len(), 1);
        assert_eq!(
            briefing.preconditions[0].verdict(),
            crate::plan::readiness::Readiness::NeedsDecision
        );

        let mut selections = Selections::new();
        selections.insert("guidance:0.7.0:one".to_string(), "accepted".to_string());
        let briefing = brief(
            &index(),
            &[(version("0.7.0"), breaking)],
            Some(version("0.6.6")),
            &["specs".to_string()],
            &selections,
        );
        assert_eq!(
            briefing.preconditions[0].verdict(),
            crate::plan::readiness::Readiness::Ready
        );
    }

    #[test]
    fn a_filtered_out_step_is_counted_rather_than_hidden() {
        let held = Guidance::parse("guidance/0.7.0.toml", ONE.as_bytes(), &bodies()).unwrap();
        let briefing = brief(
            &index(),
            &[(version("0.7.0"), held)],
            Some(version("0.6.6")),
            &["debt".to_string()],
            &Selections::new(),
        );
        assert_eq!(briefing.excluded, 1);
        assert!(briefing.applicable.is_empty());
        assert!(briefing.decisions.is_empty());
    }

    #[test]
    fn partial_coverage_is_a_decision_a_selection_resolves() {
        let briefing = brief(
            &index(),
            &[],
            Some(version("0.6.5")),
            &["specs".to_string()],
            &Selections::new(),
        );
        assert_eq!(briefing.preconditions.len(), 1);
        assert_eq!(
            briefing.preconditions[0].verdict(),
            crate::plan::readiness::Readiness::NeedsDecision
        );
        let mut selections = Selections::new();
        selections.insert("guidance-coverage".to_string(), "accepted".to_string());
        let briefing = brief(
            &index(),
            &[],
            Some(version("0.6.5")),
            &["specs".to_string()],
            &selections,
        );
        assert_eq!(
            briefing.preconditions[0].verdict(),
            crate::plan::readiness::Readiness::Ready
        );
    }
}
