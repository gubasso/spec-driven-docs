//! Execute exactly one stored plan, or refuse because the world moved.
//!
//! An apply that recomputed its intent from the tree could act on
//! something the operator never saw. So it reads the plan that was
//! reviewed, re-observes the target under the exclusive lock, recomputes
//! the fingerprint, and compares. A difference is a refusal that names
//! what moved, never a silent re-plan.
//!
//! The execution is the transaction the skill installer already proved:
//! stage beside each destination, journal before the first replacement,
//! replace one file at a time, write the record last. A run that does not
//! finish leaves a journal the next invocation resolves before it plans
//! anything new.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::ownership::Sha256;
use crate::error::AppError;
use crate::plan::Plan;
use crate::plan::operation::Operation;
use crate::plan::readiness::{Readiness, Requirement};
use crate::plan::store::{
    Disposition, OperationOutcome, PostconditionOutcome, RESULT_SCHEMA, Result as ApplyResult,
    Store,
};
use crate::transaction::journal::{self, Entry, Journal};
use crate::transaction::stage::Stage;

/// What an apply is given.
pub struct Request<'a> {
    /// Where the plan lives.
    pub store: &'a Store,
    /// The repository the plan is for.
    pub target: &'a Utf8Path,
    /// The plan that was approved.
    pub stored: &'a Plan,
    /// The same plan, computed again from the same inputs, just now.
    pub recomputed: &'a Plan,
    /// The release the plan froze, for the verification postcondition.
    pub bundle: &'a dyn crate::release::ReleaseBundle,
    /// The clock, as an input.
    pub now: String,
}

/// Every semantic input that moved between the plan and the apply.
///
/// Collected in one pass rather than reported one at a time, so an
/// operator learns the whole difference from one refusal.
#[must_use]
pub fn moved(stored: &Plan, recomputed: &Plan) -> Vec<String> {
    let mut moved = Vec::new();
    if stored.classification != recomputed.classification {
        moved.push(format!(
            "the target is now {} and the plan described {}",
            recomputed.classification, stored.classification
        ));
    }
    if stored.desired_state.release_sha256 != recomputed.desired_state.release_sha256 {
        moved.push("the release the plan resolved is no longer the one it resolved".to_string());
    }
    let record = |plan: &Plan| {
        plan.observed_state
            .installation
            .as_ref()
            .map(|installation| installation.record_sha256.to_string())
    };
    if record(stored) != record(recomputed) {
        moved.push("the instance record changed".to_string());
    }
    let declaration = |plan: &Plan| {
        plan.observed_state
            .installation
            .as_ref()
            .and_then(|installation| installation.declaration_sha256.clone())
    };
    if declaration(stored) != declaration(recomputed) {
        moved.push("the project's declaration changed".to_string());
    }
    for operation in &stored.operations {
        // The record restates the others and carries the moment of
        // installation, so a difference in it alone is not the world
        // moving. The operations it summarizes are checked below.
        if matches!(operation, Operation::WriteRecord { .. }) {
            continue;
        }
        let found = recomputed
            .operations
            .iter()
            .find(|other| other.path() == operation.path());
        match found {
            Some(other) if other == operation => {}
            Some(_) => moved.push(format!(
                "{} no longer needs what the plan described",
                operation.path()
            )),
            None => moved.push(format!(
                "{} is no longer part of the plan",
                operation.path()
            )),
        }
    }
    for operation in &recomputed.operations {
        if matches!(operation, Operation::WriteRecord { .. }) {
            continue;
        }
        if !stored
            .operations
            .iter()
            .any(|other| other.path() == operation.path())
        {
            moved.push(format!("{} is newly part of the plan", operation.path()));
        }
    }
    if selected_answers(stored) != selected_answers(recomputed) {
        moved.push("a selected decision changed".to_string());
    }
    moved
}

/// What the operator answered, by decision.
fn selected_answers(plan: &Plan) -> BTreeMap<&str, &str> {
    plan.decisions
        .iter()
        .filter_map(|decision| {
            decision
                .selected
                .as_deref()
                .map(|answer| (decision.id.as_str(), answer))
        })
        .collect()
}

/// Why an apply refused, from the closed set.
fn refuse(reason: &str) -> AppError {
    AppError::Refused(reason.to_string())
}

/// Execute one stored plan.
///
/// # Errors
///
/// [`AppError::Refused`] when the plan is not ready, when a semantic input
/// moved, or when an operation could not be applied and the target was put
/// back. [`AppError::Unrecovered`] when a run could not be put back.
pub fn apply(request: &Request<'_>) -> std::result::Result<ApplyResult, AppError> {
    let Request {
        store,
        target,
        stored,
        recomputed,
        bundle,
        now,
    } = request;
    let directory = store.directory(&stored.identity.plan_id);

    // Recover before anything else. A run that did not finish is resolved
    // deterministically before another plan is even considered.
    journal::recover(&directory.journal)?;

    // The fingerprint is the identity an approval bound to, so it decides.
    // `moved` runs only to explain a difference the digest already proved,
    // and it never decides on its own: a projection field it does not
    // restate, the target's own path among them, would otherwise let a
    // plan approved for one repository execute against another.
    let mut differences = Vec::new();
    if stored.input_fingerprint != recomputed.input_fingerprint {
        differences = moved(stored, recomputed);
        if differences.is_empty() {
            differences.push(format!(
                "the plan's inputs no longer hash to {}",
                stored.identity.plan_id
            ));
        }
    }
    if !differences.is_empty() {
        let result = terminal(
            stored,
            now,
            Disposition::Invalidated,
            &format!(
                "the plan no longer describes the target: {}",
                differences.join("; ")
            ),
        );
        store.record(stored, &result)?;
        return Err(refuse(&result.reason));
    }

    match stored.readiness {
        Readiness::Ready => {}
        Readiness::NeedsDecision => {
            let waiting: Vec<&str> = stored
                .decisions
                .iter()
                .filter(|decision| decision.selected.is_none())
                .map(|decision| decision.id.as_str())
                .collect();
            return Err(refuse(&format!(
                "the plan waits on a decision: {}; answer it with --set and plan again",
                waiting.join(", ")
            )));
        }
        Readiness::Blocked => {
            let blocked: Vec<&str> = stored
                .preconditions
                .iter()
                .filter(|precondition| {
                    precondition.requirement == Requirement::Required
                        && !precondition.evaluation.is_satisfied()
                })
                .map(|precondition| precondition.id.as_str())
                .collect();
            return Err(refuse(&format!(
                "the plan is blocked by {}",
                blocked.join(", ")
            )));
        }
    }

    if stored.operations.is_empty() {
        // A plan with nothing to write still proves what it claims. A
        // target that already holds every byte can still fail its own
        // verification, and reporting success without looking would put
        // that claim in the result unchecked.
        let postconditions = prove(target, stored, *bundle);
        let failed: Vec<&PostconditionOutcome> =
            postconditions.iter().filter(|held| !held.held).collect();
        let (disposition, reason) = failed.first().map_or_else(
            || {
                (
                    Disposition::Succeeded,
                    "the target already holds what the plan describes".to_string(),
                )
            },
            |first| {
                (
                    Disposition::Retryable,
                    format!(
                        "apply aborted: the postcondition {} did not hold: {}",
                        first.id,
                        first.detail.clone().unwrap_or_default()
                    ),
                )
            },
        );
        let result = ApplyResult {
            postconditions,
            ..terminal(stored, now, disposition, &reason)
        };
        store.record(stored, &result)?;
        if disposition == Disposition::Succeeded {
            return Ok(result);
        }
        return Err(refuse(&result.reason));
    }

    execute(store, target, stored, *bundle, now)
}

/// Stage, journal, replace, and prove.
fn execute(
    store: &Store,
    target: &Utf8Path,
    plan: &Plan,
    bundle: &dyn crate::release::ReleaseBundle,
    now: &str,
) -> std::result::Result<ApplyResult, AppError> {
    let directory = store.directory(&plan.identity.plan_id);
    let (entries, planned) = stage_every_operation(store, target, plan)?;

    let mut journal = Journal::begin(&directory.journal, &directory.blobs, entries)?;
    let mut outcomes = Vec::new();
    let mut affected = Vec::new();
    for ((destination, bytes), operation) in planned.iter().zip(ordered(plan)) {
        // Every fallible step after the journal opened routes through
        // this one result. A `?` here would return with the journal
        // outstanding and the operations before it still applied, which
        // is the state the journal exists to prevent.
        let done = bytes
            .as_ref()
            .map_or_else(
                || remove(destination),
                |bytes| {
                    Stage::write(destination, bytes)
                        .and_then(|scratch| Stage::replace(&scratch, destination))
                },
            )
            .and_then(|()| journal.mark_done(destination));
        if let Err(cause) = done {
            let restored = journal.roll_back();
            let reason = format!("{} could not be written: {cause}", operation.path());
            let result = terminal(
                plan,
                now,
                if restored.is_ok() {
                    Disposition::Retryable
                } else {
                    Disposition::RecoveryRequired
                },
                &reason,
            );
            store.record(plan, &result)?;
            return Err(restored.err().map_or_else(
                || refuse(&format!("{reason}; the target was put back")),
                |failure| {
                    AppError::Unrecovered(format!(
                        "{reason}; the target could not be put back: {failure}"
                    ))
                },
            ));
        }
        outcomes.push(OperationOutcome {
            kind: operation.kind().to_string(),
            path: operation.path().as_str().to_string(),
            applied: true,
            refusal: None,
        });
        affected.push(operation.path().as_str().to_string());
    }

    let postconditions = prove(target, plan, bundle);
    let failed: Vec<&PostconditionOutcome> =
        postconditions.iter().filter(|held| !held.held).collect();
    if let Some(first) = failed.first() {
        let reason = format!(
            "apply aborted: the postcondition {} did not hold: {}",
            first.id,
            first.detail.clone().unwrap_or_default()
        );
        let restored = journal.roll_back();
        let result = ApplyResult {
            postconditions: postconditions.clone(),
            ..terminal(
                plan,
                now,
                if restored.is_ok() {
                    Disposition::Retryable
                } else {
                    Disposition::RecoveryRequired
                },
                &reason,
            )
        };
        store.record(plan, &result)?;
        return Err(refuse(&reason));
    }

    journal.finish()?;
    let result = ApplyResult {
        operations: outcomes,
        postconditions,
        affected,
        ..terminal(plan, now, Disposition::Succeeded, "every operation landed")
    };
    store.record(plan, &result)?;
    Ok(result)
}

/// One destination and the bytes to put there, or nothing where it goes.
type Staged = (Utf8PathBuf, Option<Vec<u8>>);

/// Back up every destination and read every byte the plan will write.
///
/// Everything is in hand before the journal exists, so a failure here has
/// nothing to roll back.
fn stage_every_operation(
    store: &Store,
    target: &Utf8Path,
    plan: &Plan,
) -> std::result::Result<(Vec<Entry>, Vec<Staged>), AppError> {
    let directory = store.directory(&plan.identity.plan_id);
    let stage = Stage::new(&directory.blobs)?;
    let mut entries = Vec::new();
    let mut planned: Vec<Staged> = Vec::new();
    for operation in ordered(plan) {
        // A validated target-relative path is not containment. A directory
        // along the way can be a symlink out of the repository, and a
        // rename through one writes wherever it points. The check runs
        // before anything is read or staged, so a refusal leaves the whole
        // target untouched.
        contained(target, operation.path().as_path())?;
        let destination = target.join(operation.path().as_path());
        let before = stage.back_up(&destination)?;
        match operation.after() {
            Some(after) => {
                let bytes = store.blob(&plan.identity.plan_id, after)?;
                entries.push(Entry::write(destination.clone(), before, after.clone()));
                planned.push((destination, Some(bytes)));
            }
            None => {
                if let Some(before) = before {
                    entries.push(Entry::remove(destination.clone(), before));
                    planned.push((destination, None));
                }
            }
        }
    }
    Ok((entries, planned))
}

/// Take one destination away, treating an absent one as already gone.
fn remove(destination: &Utf8Path) -> std::result::Result<(), AppError> {
    match std::fs::remove_file(destination) {
        Ok(()) => crate::transaction::sync_parent(destination).map_err(AppError::Io),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(AppError::Io(source)),
    }
}

/// Refuse a destination that leaves the target.
///
/// The same guard the landing verbs run, applied to every operation of
/// every kind: write, splice, and removal alike.
///
/// # Errors
///
/// [`AppError::Refused`] naming the destination and what was wrong.
fn contained(target: &Utf8Path, relative: &Utf8Path) -> std::result::Result<(), AppError> {
    crate::adapters::fs::check_destination(target, relative).map_err(|refusal| {
        AppError::Refused(match refusal {
            crate::adapters::fs::DestinationRefusal::SymlinkEscape => {
                format!("destination escapes the target through a symlink: {relative}")
            }
            crate::adapters::fs::DestinationRefusal::FileBlocksDirectory(blocked) => {
                format!("a file blocks a directory the plan needs: {blocked}")
            }
            crate::adapters::fs::DestinationRefusal::NotARegularFile => {
                format!("destination exists and is not a regular file: {relative}")
            }
        })
    })
}

/// The order the apply writes in.
///
/// The record is last, because it is the claim that the rest landed. A run
/// the process did not finish is rolled back whole, this operation with
/// it, so a target never carries a record for files it does not hold.
fn ordered(plan: &Plan) -> Vec<&Operation> {
    let mut ordered: Vec<&Operation> = plan
        .operations
        .iter()
        .filter(|operation| !matches!(operation, Operation::WriteRecord { .. }))
        .collect();
    ordered.extend(
        plan.operations
            .iter()
            .filter(|operation| matches!(operation, Operation::WriteRecord { .. })),
    );
    ordered
}

/// What the apply proves once every operation has landed.
fn prove(
    target: &Utf8Path,
    plan: &Plan,
    bundle: &dyn crate::release::ReleaseBundle,
) -> Vec<PostconditionOutcome> {
    plan.postconditions
        .iter()
        .map(|postcondition| match postcondition.id.as_str() {
            "record-matches-the-tree" => {
                let wrong: Vec<String> = plan
                    .operations
                    .iter()
                    .filter_map(|operation| {
                        let destination = target.join(operation.path().as_path());
                        let found = std::fs::read(&destination)
                            .ok()
                            .map(|bytes| Sha256::of(&bytes));
                        (found.as_ref() != operation.after()).then(|| operation.path().to_string())
                    })
                    .collect();
                PostconditionOutcome {
                    id: postcondition.id.clone(),
                    held: wrong.is_empty(),
                    detail: (!wrong.is_empty()).then(|| {
                        format!(
                            "these destinations do not hold the plan's digest: {}",
                            wrong.join(", ")
                        )
                    }),
                }
            }
            "verification-passes" => {
                let report = crate::services::verifier::verify(target, bundle);
                let detail = match &report {
                    Ok(report) if report.failures == 0 => None,
                    Ok(report) => Some(format!(
                        "sdd verify reports {} failure(s): {}",
                        report.failures,
                        report.lines.join("; ")
                    )),
                    Err(source) => Some(format!("sdd verify could not run: {source}")),
                };
                PostconditionOutcome {
                    id: postcondition.id.clone(),
                    held: detail.is_none(),
                    detail,
                }
            }
            // A postcondition nobody implemented is not a postcondition
            // that held. Reporting it as proved would put a claim in the
            // result that nothing behind it ever checked.
            other => PostconditionOutcome {
                id: other.to_string(),
                held: false,
                detail: Some(format!("{other} has no check behind it in this engine")),
            },
        })
        .collect()
}

/// One result, with everything but the outcome lists filled in.
fn terminal(plan: &Plan, now: &str, disposition: Disposition, reason: &str) -> ApplyResult {
    ApplyResult {
        schema: RESULT_SCHEMA.to_string(),
        plan_id: plan.identity.plan_id.clone(),
        fingerprint: plan.input_fingerprint.clone(),
        result_id: format!(
            "{}-{}",
            now.replace([':', '.'], "-"),
            disposition_slug(disposition)
        ),
        disposition,
        finished_at: now.to_string(),
        operations: Vec::new(),
        postconditions: Vec::new(),
        recovery_required: disposition == Disposition::RecoveryRequired,
        affected: Vec::new(),
        reason: reason.to_string(),
    }
}

const fn disposition_slug(disposition: Disposition) -> &'static str {
    match disposition {
        Disposition::Succeeded => "succeeded",
        Disposition::Invalidated => "invalidated",
        Disposition::Retryable => "retryable",
        Disposition::RecoveryRequired => "recovery-required",
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;
    use crate::plan::operation::{Class, TargetPath};

    fn write(path: &str, after: &[u8]) -> Operation {
        Operation::WriteFile {
            path: TargetPath::new(path).unwrap(),
            class: Class::Managed,
            before: None,
            after: Sha256::of(after),
        }
    }

    fn record(path: &str, after: &[u8]) -> Operation {
        Operation::WriteRecord {
            path: TargetPath::new(path).unwrap(),
            before: None,
            after: Sha256::of(after),
        }
    }

    fn plan_with(operations: Vec<Operation>) -> Plan {
        let mut plan = crate::plan::planner::plan(&crate::plan::planner::Inputs {
            observation: &crate::plan::observe::Observation {
                repository: crate::plan::observe::Repository {
                    root: Utf8PathBuf::from("/nowhere"),
                    version_controlled: true,
                    empty: true,
                },
                installation: None,
                invalid: None,
                host: crate::plan::observe::Host {
                    offline: true,
                    cache_root: None,
                },
                corpus: crate::plan::observe::Corpus::default(),
            },
            declaration: &crate::domain::profile::DECLARATION,
            candidate: &BTreeMap::new(),
            baseline: None,
            selector: "embedded".to_string(),
            release: "0.0.0".to_string(),
            release_sha256: Sha256::of(b"release"),
            provenance: "native".to_string(),
            registry_checksum: None,
            yanked: false,
            compatibility: None,
            interval: None,
            briefing: None,
            proposed: None,
            selections: &crate::plan::decision::Selections::new(),
            reserve: &[],
            declarations_settled: false,
            now: "2026-09-12T00:00:00Z".to_string(),
        });
        plan.operations = operations;
        plan
    }

    #[test]
    fn the_record_is_written_last() {
        let plan = plan_with(vec![
            record(".spec-driven-docs/manifest.json", b"record"),
            write("a.md", b"a"),
            write("b.md", b"b"),
        ]);
        let order: Vec<&str> = ordered(&plan)
            .iter()
            .map(|operation| operation.path().as_str())
            .collect();
        assert_eq!(order, ["a.md", "b.md", ".spec-driven-docs/manifest.json"]);
    }

    #[test]
    fn nothing_moved_reports_no_difference() {
        let plan = plan_with(vec![write("a.md", b"a")]);
        assert!(moved(&plan, &plan).is_empty());
    }

    #[test]
    fn a_changed_operation_is_named_by_its_destination() {
        let one = plan_with(vec![write("a.md", b"a")]);
        let two = plan_with(vec![write("a.md", b"different")]);
        let differences = moved(&one, &two);
        assert_eq!(differences.len(), 1);
        assert!(differences[0].contains("a.md"), "{differences:?}");
    }

    #[test]
    fn an_added_or_dropped_operation_is_named() {
        let one = plan_with(vec![write("a.md", b"a")]);
        let two = plan_with(vec![write("a.md", b"a"), write("b.md", b"b")]);
        assert!(moved(&one, &two).iter().any(|held| held.contains("newly")));
        assert!(
            moved(&two, &one)
                .iter()
                .any(|held| held.contains("no longer part"))
        );
    }
}
