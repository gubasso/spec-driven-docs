//! The pure function from what was observed to what will be done.
//!
//! Nothing here reads a disk, reaches a network, or asks a clock. Every
//! one of those is an input, so the same inputs produce the same plan and
//! the same fingerprint. That is what lets an approval bind to a plan and
//! an apply prove it is still executing the plan that was approved.

use std::collections::BTreeMap;

use crate::domain::ownership::Sha256;
use crate::domain::profile::{ProfileId, resolve_destination};
use crate::domain::projection::Declaration;
use crate::plan::classify::{Classification, Signals, classify};
use crate::plan::decision::{self, AnswerSchema, Choice, Decision, Selections};
use crate::plan::evidence::{Ledger, Producer};
use crate::plan::finding::{Finding, FindingKind, StyleCandidate, detector};
use crate::plan::fingerprint::{Value, fingerprint};
use crate::plan::observe::{Observation, RecordedFile};
use crate::plan::operation::{Class, Operation, TargetPath, no_duplicate_destination};
use crate::plan::readiness::{Evaluation, Precondition, Readiness, Requirement, readiness};
use crate::plan::{
    Declared, DesiredState, Identity, ObservedState, PLAN_SCHEMA, Plan, Postcondition,
    ReleaseSource,
};

/// What the planner is given besides the observation.
#[derive(Debug, Clone)]
pub struct Inputs<'a> {
    /// What was observed at the target.
    pub observation: &'a Observation,
    /// What the destination release declares it lands.
    pub declaration: &'a Declaration,
    /// Each destination's bytes in the destination release, by source path.
    pub candidate: &'a BTreeMap<String, Sha256>,
    /// Each destination's bytes in the recorded release, where it could be
    /// read. Absent means the baseline was not observed.
    pub baseline: Option<&'a BTreeMap<String, Sha256>>,
    /// What the caller asked for, before resolution.
    pub selector: String,
    /// The release the selector resolved to.
    pub release: String,
    /// The digest over that release's content.
    pub release_sha256: Sha256,
    /// Where the release's facts came from.
    pub provenance: String,
    /// The registry checksum, where a registry served it.
    pub registry_checksum: Option<Sha256>,
    /// Whether the registry marks the release yanked.
    pub yanked: bool,
    /// What one landing would write, where the caller computed it.
    ///
    /// The installer owns that computation, so the planner takes its
    /// answer rather than deriving a second one that could disagree.
    pub proposed: Option<&'a [Operation]>,
    /// What the operator selected.
    pub selections: &'a Selections,
    /// The clock, as an input.
    pub now: String,
}

/// Compute one plan.
///
/// The order is fixed: classify from what was observed, read the findings,
/// offer the decisions, derive the operations the selected decisions allow,
/// evaluate the preconditions, and take the readiness from the worst of
/// them. The fingerprint is last, over exactly the parts that decide what
/// the apply would do.
#[must_use]
pub fn plan(inputs: &Inputs<'_>) -> Plan {
    let observation = inputs.observation;
    let (ledger, refs, release_ref) = ledger_of(inputs);

    let profile = chosen_profile(inputs);
    let classification = classification_of(inputs, profile);
    let findings = findings_of(inputs, profile, classification);
    let style_candidates = style_candidates_of(inputs, classification);
    let structural = findings
        .iter()
        .filter(|found| found.kind == FindingKind::Structural)
        .count();
    let decisions = decisions_of(inputs, classification, profile, structural, &findings);
    let operations = inputs.proposed.map_or_else(
        || derived_operations(inputs, profile, classification, &decisions),
        <[Operation]>::to_vec,
    );

    let preconditions =
        preconditions_of(inputs, classification, &operations, &decisions, structural);
    let verdict = readiness(&preconditions);

    let desired_state = DesiredState {
        selector: inputs.selector.clone(),
        release: inputs.release.clone(),
        release_sha256: inputs.release_sha256.clone(),
        profile,
        declared: Declared {
            payload_schema: inputs.declaration.payload_schema,
            managed: inputs.declaration.managed.len(),
            adopted: inputs.declaration.adopted.len(),
            sentinels: inputs.declaration.sentinels.len(),
        },
    };
    let observed_state = ObservedState {
        repository: observation.repository.clone(),
        installation: observation.installation.clone(),
        host: observation.host.clone(),
        corpus: observation.corpus.clone(),
        evidence_refs: refs,
    };
    let release = ReleaseSource {
        version: inputs.release.clone(),
        provenance: inputs.provenance.clone(),
        registry_checksum: inputs.registry_checksum.clone(),
        yanked: inputs.yanked,
        evidence_refs: vec![release_ref],
    };
    let digest = fingerprint(&inputs_projection(
        inputs,
        classification,
        &operations,
        &preconditions,
        &decisions,
    ));
    Plan {
        identity: Identity {
            schema: PLAN_SCHEMA.to_string(),
            plan_id: digest.to_string(),
            created_at: inputs.now.clone(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        classification,
        findings,
        style_candidates,
        desired_state,
        observed_state,
        release,
        operations,
        preconditions,
        decisions,
        postconditions: postconditions_of(classification),
        evidence: ledger.items,
        readiness: verdict,
        input_fingerprint: digest,
    }
}

/// Everything the plan observed, and the references its sections cite.
fn ledger_of(inputs: &Inputs<'_>) -> (Ledger, Vec<String>, String) {
    let mut ledger = Ledger::new();
    let mut refs = Vec::new();
    refs.push(ledger.record(
        "target",
        "the target's working tree",
        Producer::Disk,
        &inputs.now,
        None,
        "walked, skipping version control and build output",
    ));
    if let Some(installation) = inputs.observation.installation.as_ref() {
        refs.push(ledger.record(
            "record",
            "the instance record",
            Producer::Record,
            &inputs.now,
            Some(installation.record_sha256.clone()),
            "read and parsed",
        ));
        if let Some(digest) = installation.declaration_sha256.as_ref() {
            refs.push(ledger.record(
                "declaration",
                "the project's own declaration",
                Producer::Declaration,
                &inputs.now,
                Some(digest.clone()),
                "read from the target",
            ));
        }
    }
    let release_ref = ledger.record(
        "release",
        "the destination release",
        Producer::Bundle,
        &inputs.now,
        Some(inputs.release_sha256.clone()),
        "read through the release seam",
    );
    refs.push(ledger.record(
        "host",
        "the resolved user-scope paths",
        Producer::Host,
        &inputs.now,
        None,
        "read from the environment",
    ));
    (ledger, refs, release_ref)
}

/// The profile this plan lands, where one is settled.
fn chosen_profile(inputs: &Inputs<'_>) -> Option<ProfileId> {
    if let Some(installation) = inputs.observation.installation.as_ref() {
        return Some(installation.profile);
    }
    match inputs
        .selections
        .get(decision::id::PROFILE)
        .map(String::as_str)
    {
        Some("codebase") => Some(ProfileId::Codebase),
        Some("knowledge-base") => Some(ProfileId::KnowledgeBase),
        _ => None,
    }
}

fn classification_of(inputs: &Inputs<'_>, profile: Option<ProfileId>) -> Classification {
    let observation = inputs.observation;
    let drifted = observation
        .installation
        .as_ref()
        .is_some_and(crate::plan::observe::Installation::drifted);
    let at_destination = observation
        .installation
        .as_ref()
        .is_some_and(|installed| installed.canon_version.to_string() == inputs.release);
    let _ = profile;
    classify(Signals {
        invalid: observation.invalid.is_some(),
        installed: observation.installation.is_some(),
        at_destination,
        drifted,
        settled: observation.corpus.settled(),
    })
}

/// Every finding the program can prove.
fn findings_of(
    inputs: &Inputs<'_>,
    profile: Option<ProfileId>,
    classification: Classification,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    if classification != Classification::Migration {
        return findings;
    }
    let corpus = &inputs.observation.corpus;
    // The foreign-root detector needs the profile, so it waits for the
    // decision rather than judging against a default.
    if let Some(profile) = profile
        && let Some(docs_root) = inputs.declaration.docs_root(profile)
    {
        for root in &corpus.populated_doc_roots {
            if root == docs_root.as_str() {
                continue;
            }
            if let Ok(path) = TargetPath::new(root) {
                findings.push(Finding {
                    kind: FindingKind::Structural,
                    path,
                    rule: detector::FOREIGN_DOCS_ROOT.to_string(),
                    statement: format!(
                        "{root} holds documents and the {profile} profile keeps them under {docs_root}"
                    ),
                    measurement: None,
                });
            }
        }
        if corpus.settled()
            && !corpus.has_specs_directory
            && let Ok(path) = TargetPath::new(&format!("{docs_root}/specs"))
        {
            findings.push(Finding {
                kind: FindingKind::Structural,
                path,
                rule: detector::NO_SPECS_DIRECTORY.to_string(),
                statement: "the corpus is settled and no specifications directory holds its rules"
                    .to_string(),
                measurement: None,
            });
        }
    }
    for path in &corpus.spec_without_rule_id {
        findings.push(Finding {
            kind: FindingKind::Structural,
            path: path.clone(),
            rule: detector::SPEC_WITHOUT_RULE_ID.to_string(),
            statement: "the document is shaped like a specification and defines no rule ID"
                .to_string(),
            measurement: None,
        });
    }
    for path in &corpus.ordinal_named {
        findings.push(Finding {
            kind: FindingKind::Structural,
            path: path.clone(),
            rule: detector::ORDINAL_FILENAME.to_string(),
            statement: "the document is named by its position rather than its subject".to_string(),
            measurement: None,
        });
    }
    for path in &corpus.records_outside_decisions {
        findings.push(Finding {
            kind: FindingKind::Structural,
            path: path.clone(),
            rule: detector::RECORD_OUTSIDE_DECISIONS.to_string(),
            statement: "the decision record sits outside a decisions directory".to_string(),
            measurement: None,
        });
    }
    findings
}

/// Documents written before the instance, named and never judged.
fn style_candidates_of(inputs: &Inputs<'_>, classification: Classification) -> Vec<StyleCandidate> {
    if classification != Classification::Migration {
        return Vec::new();
    }
    inputs
        .observation
        .corpus
        .documents
        .iter()
        .map(|path| StyleCandidate {
            path: path.clone(),
            reason: "the document predates the instance, and no gate judges its prose".to_string(),
        })
        .collect()
}

fn choice(id: &str, consequence: &str) -> Choice {
    Choice {
        id: id.to_string(),
        consequence: consequence.to_string(),
    }
}

/// Every decision the plan is waiting on, in dependency order.
fn decisions_of(
    inputs: &Inputs<'_>,
    classification: Classification,
    profile: Option<ProfileId>,
    structural: usize,
    findings: &[Finding],
) -> Vec<Decision> {
    let mut decisions = Vec::new();
    let selected = |id: &str| inputs.selections.get(id).cloned();

    if inputs.observation.installation.is_none()
        && matches!(
            classification,
            Classification::Setup | Classification::Migration
        )
    {
        decisions.push(Decision {
            id: decision::id::PROFILE.to_string(),
            question: "which profile does this repository take?".to_string(),
            schema: AnswerSchema::Choice {
                choices: vec![
                    choice("codebase", "records live under docs/"),
                    choice("knowledge-base", "records live under _docs/"),
                ],
            },
            depends_on: Vec::new(),
            selected: selected(decision::id::PROFILE),
        });
    }

    // Everything below is profile-relative, so it waits for the profile.
    if profile.is_some() {
        if inputs.observation.installation.is_none() {
            let depends: Vec<String> = if decisions
                .iter()
                .any(|held| held.id == decision::id::PROFILE)
            {
                vec![decision::id::PROFILE.to_string()]
            } else {
                Vec::new()
            };
            decisions.push(Decision {
                id: decision::id::PLAN_ZONE.to_string(),
                question: "where does the planning tool write its entry documents?".to_string(),
                schema: AnswerSchema::ChoiceOrValue {
                    choices: vec![
                        choice("env", "wherever the plan-zone variable points"),
                        choice("none", "the project keeps no plan zone"),
                    ],
                    prefixes: vec!["project:".to_string(), "untracked:".to_string()],
                },
                depends_on: depends.clone(),
                selected: selected(decision::id::PLAN_ZONE),
            });
            decisions.push(Decision {
                id: decision::id::DOCS_SCRATCH.to_string(),
                question: "where does material that is not a statement yet stage?".to_string(),
                schema: AnswerSchema::ChoiceOrValue {
                    choices: vec![choice("none", "the project stages nothing")],
                    // A recorded scratch is a path. The variable overrides
                    // it at read time, so there is no `env` to record.
                    prefixes: vec!["project:".to_string(), "external:".to_string()],
                },
                depends_on: depends.clone(),
                selected: selected(decision::id::DOCS_SCRATCH),
            });
            decisions.push(Decision {
                id: decision::id::WRITING_STYLE.to_string(),
                question: "which writing source does the project select?".to_string(),
                schema: AnswerSchema::ChoiceOrValue {
                    choices: vec![
                        choice("builtin", "this convention's own chapter, served offline"),
                        choice("none", "no route and no conversion obligation"),
                    ],
                    prefixes: vec!["project:".to_string()],
                },
                depends_on: depends,
                selected: selected(decision::id::WRITING_STYLE),
            });
        }

        if classification == Classification::Migration {
            decisions.extend(migration_decisions(inputs, structural, findings));
        }
    }

    if inputs.yanked {
        decisions.push(Decision {
            id: decision::id::ACCEPT_YANKED.to_string(),
            question: format!(
                "the registry marks {} yanked; land it anyway?",
                inputs.release
            ),
            schema: AnswerSchema::Choice {
                choices: vec![
                    choice("accept", "the release lands, yanked and named as such"),
                    choice("refuse", "nothing lands; name another release"),
                ],
            },
            depends_on: Vec::new(),
            selected: selected(decision::id::ACCEPT_YANKED),
        });
    }
    decisions
}

/// What a migration asks before it moves anything.
fn migration_decisions(
    inputs: &Inputs<'_>,
    structural: usize,
    findings: &[Finding],
) -> Vec<Decision> {
    let selected = |id: &str| inputs.selections.get(id).cloned();
    let mut decisions = Vec::new();
    let mut choices = vec![choice(
        "sweep",
        "every durable fact moves into its owner, and the old convention retires",
    )];
    // Incremental is offered only where nothing structural would
    // make two conventions coexist.
    if structural == 0 {
        choices.push(choice(
            "incremental",
            "each document converts the next time somebody edits it",
        ));
    }
    decisions.push(Decision {
        id: decision::id::MIGRATION_SCOPE.to_string(),
        question: "how much of the corpus moves?".to_string(),
        schema: AnswerSchema::Choice { choices },
        depends_on: vec![decision::id::PROFILE.to_string()],
        selected: selected(decision::id::MIGRATION_SCOPE),
    });
    if findings
        .iter()
        .any(|found| found.kind == FindingKind::Budget)
    {
        decisions.push(Decision {
            id: decision::id::DEBT_BASELINE.to_string(),
            question: "are the inherited violations recorded as debt?".to_string(),
            schema: AnswerSchema::Choice {
                choices: vec![
                    choice(
                        "record",
                        "each inherited violation becomes a ceiling that only comes down",
                    ),
                    choice(
                        "skip",
                        "nothing is recorded, and each violation fails its gate",
                    ),
                ],
            },
            depends_on: vec![decision::id::MIGRATION_SCOPE.to_string()],
            selected: selected(decision::id::DEBT_BASELINE),
        });
    }
    decisions
}

/// Every write the plan derives for itself, where the caller gave none.
///
/// Until the profile is chosen, every destination is unknown, so the
/// planner offers no operation rather than one against a default nobody
/// selected.
fn derived_operations(
    inputs: &Inputs<'_>,
    profile: Option<ProfileId>,
    classification: Classification,
    decisions: &[Decision],
) -> Vec<Operation> {
    if profile.is_none() {
        return Vec::new();
    }
    operations_of(inputs, profile, classification, decisions)
}

/// Every write the plan will make.
fn operations_of(
    inputs: &Inputs<'_>,
    profile: Option<ProfileId>,
    classification: Classification,
    decisions: &[Decision],
) -> Vec<Operation> {
    let mut operations = Vec::new();
    if matches!(
        classification,
        Classification::Invalid | Classification::Current
    ) {
        return operations;
    }
    let Some(profile) = profile else {
        return operations;
    };
    let Some(docs_root) = inputs.declaration.docs_root(profile) else {
        return operations;
    };
    let held = crate::plan::observe::held_by_path(inputs.observation.installation.as_ref());
    let recorded: BTreeMap<&str, &RecordedFile> = inputs
        .observation
        .installation
        .iter()
        .flat_map(|installation| installation.adopted.iter())
        .map(|file| (file.path.as_str(), file))
        .collect();

    for projection in &inputs.declaration.managed {
        let Some(after) = inputs.candidate.get(&projection.source) else {
            continue;
        };
        let Ok(path) = TargetPath::new(&projection.destination) else {
            continue;
        };
        let before = held.get(path.as_str()).cloned();
        if before.as_ref() == Some(after) {
            continue;
        }
        operations.push(Operation::WriteFile {
            path,
            class: Class::Managed,
            before,
            after: after.clone(),
        });
    }

    for projection in &inputs.declaration.adopted {
        let Some(seed) = inputs.candidate.get(&projection.source) else {
            continue;
        };
        let destination = resolve_destination(&projection.destination, docs_root);
        let Ok(path) = TargetPath::new(destination.as_str()) else {
            continue;
        };
        match held.get(path.as_str()) {
            // An adopted file the target holds is the project's. Only the
            // baseline it is read against moves.
            Some(current) => {
                let baseline_before = recorded
                    .get(path.as_str())
                    .and_then(|file| file.baseline.clone())
                    .or_else(|| {
                        inputs
                            .baseline
                            .and_then(|held| held.get(&projection.source).cloned())
                    });
                let Some(baseline_before) = baseline_before else {
                    continue;
                };
                if &baseline_before == seed {
                    continue;
                }
                operations.push(Operation::KeepFile {
                    path,
                    held: current.clone(),
                    baseline_before,
                    baseline_after: seed.clone(),
                });
            }
            None => operations.push(Operation::WriteFile {
                path,
                class: Class::Adopted,
                before: None,
                after: seed.clone(),
            }),
        }
    }

    // The two operator-invoked writes into adopted state appear only when
    // their decision is selected, never because a version moved.
    let selected = |id: &str| {
        decisions
            .iter()
            .find(|decision| decision.id == id)
            .and_then(|decision| decision.selected.as_deref())
    };
    // The debt write waits for the budget findings that give it content:
    // an operation whose bytes nothing carries is an operation an apply
    // could not execute, and the plan does not offer one.
    let _ = selected(decision::id::DEBT_BASELINE);
    operations
}

/// Everything that must hold before the apply.
fn preconditions_of(
    inputs: &Inputs<'_>,
    classification: Classification,
    operations: &[Operation],
    decisions: &[Decision],
    structural: usize,
) -> Vec<Precondition> {
    let mut preconditions: Vec<Precondition> = Vec::new();
    macro_rules! require {
        ($id:expr, $statement:expr, $requirement:expr, $evaluation:expr) => {
            preconditions.push(Precondition {
                id: ($id).to_string(),
                statement: $statement,
                requirement: $requirement,
                evaluation: $evaluation,
                resolved_by: None,
                evidence_refs: Vec::new(),
            });
        };
    }

    if let Some(reason) = inputs.observation.invalid.as_ref() {
        require!(
            "record-is-readable",
            "the instance record parses".to_string(),
            Requirement::Required,
            Evaluation::Unsatisfied {
                reason: reason.clone(),
            }
        );
    }

    let waiting = decision_preconditions(decisions);

    let edited = edited_managed_files(inputs);
    if !edited.is_empty() {
        require!(
            "managed-files-are-unedited",
            "every managed file still holds what the record says".to_string(),
            Requirement::Required,
            Evaluation::Unsatisfied {
                reason: edited.join("; "),
            }
        );
    }

    if inputs.baseline.is_none() && inputs.observation.installation.is_some() {
        require!(
            "baseline-is-readable",
            "the recorded release's bundle can be read for baselines".to_string(),
            Requirement::Advisory,
            Evaluation::NotObserved {
                reason:
                    "the recorded release's bundle was not read, so an adopted baseline cannot move"
                        .to_string(),
            }
        );
    }

    // An incremental migration over a structural finding would start a
    // second convention beside the first, which is the harm the sweep
    // exists to stop.
    let scope = decisions
        .iter()
        .find(|decision| decision.id == decision::id::MIGRATION_SCOPE)
        .and_then(|decision| decision.selected.as_deref());
    if classification == Classification::Migration && scope == Some("incremental") && structural > 0
    {
        require!(
            "incremental-scope-has-no-structural-finding",
            "an incremental migration leaves no structural finding behind".to_string(),
            Requirement::Required,
            Evaluation::Unsatisfied {
                reason: format!(
                    "{structural} structural finding(s) would make two conventions coexist"
                ),
            }
        );
    }

    // A landing that wrote every projection and no record would leave a
    // target the verifier cannot read. Until the planner derives the
    // record and the two marked regions, a first landing goes through the
    // verb that does, and the plan says so rather than half-landing.
    if matches!(
        classification,
        Classification::Setup | Classification::Migration
    ) && !operations
        .iter()
        .any(|operation| matches!(operation, Operation::WriteRecord { .. }))
        && !operations.is_empty()
    {
        require!(
            "the-plan-records-the-instance",
            "a first landing writes the instance record and both marked regions".to_string(),
            Requirement::Required,
            Evaluation::NotObserved {
                reason: "this engine does not yet derive the record or the marked regions; land with 'sdd init --apply'".to_string(),
            }
        );
    }

    preconditions.extend(waiting);

    if let Err(clash) = no_duplicate_destination(operations) {
        require!(
            "no-destination-is-written-twice",
            "each destination is written by at most one operation".to_string(),
            Requirement::Required,
            Evaluation::Unsatisfied {
                reason: clash.to_string(),
            }
        );
    }
    preconditions
}

/// Every managed file the target no longer holds as the record says.
///
/// Collected in one pass rather than refused one at a time, so an operator
/// sees the whole conflict set in one run.
fn edited_managed_files(inputs: &Inputs<'_>) -> Vec<String> {
    let mut edited = Vec::new();
    let Some(installation) = inputs.observation.installation.as_ref() else {
        return edited;
    };
    for file in &installation.managed {
        match file.held.as_ref() {
            Some(found) if found == &file.recorded => {}
            Some(_) => edited.push(format!("{} was edited", file.path)),
            None => edited.push(format!("{} is gone", file.path)),
        }
    }
    edited
}

/// One precondition per decision the operator has not answered.
fn decision_preconditions(decisions: &[Decision]) -> Vec<Precondition> {
    decisions
        .iter()
        .map(|decision| Precondition {
            id: format!("decision:{}", decision.id),
            statement: decision.question.clone(),
            requirement: Requirement::DecisionRequired,
            evaluation: decision.selected.as_ref().map_or_else(
                || Evaluation::Unsatisfied {
                    reason: "the operator has not answered it".to_string(),
                },
                |_| Evaluation::Satisfied,
            ),
            resolved_by: Some(decision.id.clone()),
            evidence_refs: Vec::new(),
        })
        .collect()
}

/// What the apply proves once it has finished.
fn postconditions_of(classification: Classification) -> Vec<Postcondition> {
    if classification == Classification::Invalid {
        return Vec::new();
    }
    vec![
        Postcondition {
            id: "record-matches-the-tree".to_string(),
            statement: "every operation's destination holds the digest the plan named".to_string(),
        },
        Postcondition {
            id: "verification-passes".to_string(),
            statement: "sdd verify reports OK against the target".to_string(),
        },
    ]
}

/// Exactly the parts that decide what the apply would do.
///
/// Timestamps, presentation text, and advisory evidence are out: a plan
/// recomputed a second later must carry the same identity, or an approval
/// could never survive the moment it was given.
fn inputs_projection(
    inputs: &Inputs<'_>,
    classification: Classification,
    operations: &[Operation],
    preconditions: &[Precondition],
    decisions: &[Decision],
) -> Value {
    let operations = Value::List(
        operations
            .iter()
            // The record is a function of every other operation plus the
            // moment of installation. Its digest carries no independent
            // meaning, and the moment is exactly what the fingerprint
            // excludes, so a plan recomputed a second later keeps its id.
            .filter(|operation| !matches!(operation, Operation::WriteRecord { .. }))
            .map(|operation| {
                Value::map([
                    ("kind", Value::text(operation.kind())),
                    ("path", Value::text(operation.path().as_str())),
                    (
                        "before",
                        Value::maybe(operation.before().map(std::string::ToString::to_string)),
                    ),
                    (
                        "after",
                        Value::maybe(operation.after().map(std::string::ToString::to_string)),
                    ),
                ])
            })
            .collect(),
    );
    // Only a precondition that can change readiness or operations belongs
    // here. An advisory one cannot, by definition.
    let gates = Value::List(
        preconditions
            .iter()
            .filter(|precondition| precondition.requirement != Requirement::Advisory)
            .map(|precondition| {
                Value::map([
                    ("id", Value::text(precondition.id.as_str())),
                    (
                        "state",
                        Value::text(match precondition.evaluation {
                            Evaluation::Satisfied => "satisfied",
                            Evaluation::NotObserved { .. } => "not-observed",
                            Evaluation::Unsatisfied { .. } => "unsatisfied",
                        }),
                    ),
                ])
            })
            .collect(),
    );
    let selected = Value::Map(
        decisions
            .iter()
            .filter_map(|decision| {
                decision
                    .selected
                    .as_ref()
                    .map(|answer| (decision.id.clone(), Value::text(answer.as_str())))
            })
            .collect(),
    );
    Value::map([
        ("schema", Value::text(PLAN_SCHEMA)),
        ("classification", Value::text(classification.as_str())),
        ("release", Value::text(inputs.release.as_str())),
        (
            "release_sha256",
            Value::text(inputs.release_sha256.as_str()),
        ),
        (
            "target",
            Value::text(inputs.observation.repository.root.as_str()),
        ),
        (
            "record",
            Value::maybe(
                inputs
                    .observation
                    .installation
                    .as_ref()
                    .map(|installation| installation.record_sha256.to_string()),
            ),
        ),
        (
            "declaration",
            Value::maybe(
                inputs
                    .observation
                    .installation
                    .as_ref()
                    .and_then(|installation| installation.declaration_sha256.as_ref())
                    .map(std::string::ToString::to_string),
            ),
        ),
        ("operations", operations),
        ("preconditions", gates),
        ("decisions", selected),
    ])
}

/// Whether a plan may be applied, for a caller that has only the verdict.
#[must_use]
pub const fn is_ready(verdict: Readiness) -> bool {
    matches!(verdict, Readiness::Ready)
}
