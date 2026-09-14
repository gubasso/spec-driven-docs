//! `reconcile` subcommand: runtime-shape.
//!
//! One lifecycle over one plan. `plan` computes and stores it, `show`
//! renders a stored one, and `apply` executes exactly that plan or refuses
//! because its inputs moved. Every verb here takes the target lock, so two
//! writers cannot interleave over one repository.

use camino::{Utf8Path, Utf8PathBuf};

use crate::cli::reconcile::{ApplyArgs, PlanArgs, ReconcileArgs, ReconcileCommand, ShowArgs};
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::plan::Plan;
use crate::plan::apply::{Request, apply as execute};
use crate::plan::decision;
use crate::plan::readiness::Readiness;
use crate::plan::session::{
    STORE_WAIT, blobs_for, compute_plan, read_release, state_root, target_lock,
};
use crate::plan::store::{Result as ApplyResult, Store};
use crate::transaction::lock::Lock;

/// Run one reconcile verb.
///
/// # Errors
///
/// [`AppError::Usage`] for a target the arguments cannot mean and for a
/// decision answer the plan does not offer, [`AppError::Busy`] when
/// another process holds the target, and the resolver's refusals.
pub fn run(ctx: &AppContext, args: ReconcileArgs) -> Result<(), AppError> {
    match args.command {
        ReconcileCommand::Plan(plan) => run_plan(ctx, &plan),
        ReconcileCommand::Show(show) => run_show(&show),
        ReconcileCommand::Apply(apply) => run_apply(ctx, &apply),
    }
}

fn resolve_target(ctx: &AppContext, target: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    let named = if target.is_absolute() {
        target.to_owned()
    } else if target == "." {
        ctx.cwd.clone()
    } else {
        return Err(AppError::Usage("target must be absolute or .".to_string()));
    };
    // The lock is keyed by the path, so two spellings of one repository
    // would take two locks and interleave. Canonicalizing first makes one
    // repository one key, whichever symlink the operator typed.
    let resolved = std::fs::canonicalize(&named)
        .map_err(|_| AppError::Usage(format!("unresolved target: {named}")))?;
    Utf8PathBuf::from_path_buf(resolved)
        .map_err(|path| AppError::Usage(format!("target is not UTF-8: {}", path.display())))
}

fn run_plan(ctx: &AppContext, args: &PlanArgs) -> Result<(), AppError> {
    let target = resolve_target(ctx, &args.target)?;
    let selections =
        decision::parse(&args.set).map_err(|error| AppError::Usage(error.to_string()))?;
    let release = read_release(&args.to, args.offline)?;

    // Planning holds the target shared: several readers may plan at once,
    // and none of them may overlap an apply.
    let _lock = Lock::shared(&target_lock(&target)?, "reconcile plan")?;
    let (computed, blobs) = compute_plan(
        &target,
        &args.to,
        args.offline,
        &selections,
        &args.reserve,
        None,
        &release.borrow(),
    )?;
    decision::validate(&computed.decisions, &selections)
        .map_err(|error| AppError::Usage(error.to_string()))?;

    let store = Store::new(&state_root()?);
    // The store is global, so its own lock orders writers across targets.
    // The order is always target first, then store, on every path.
    store.create()?;
    if let Ok(_walk) = Lock::exclusive(&store.lock_path(), "plan store prune") {
        let _ = store.prune(jiff::Timestamp::now(), Some(&computed.identity.plan_id));
    }
    let _plan_lock = Lock::exclusive_waiting(
        &store.plan_lock_path(&computed.identity.plan_id)?,
        "reconcile plan",
        STORE_WAIT,
    )?;
    // Identical inputs while an executable plan exists reuse it, so an
    // operator who plans twice approves one thing.
    if !store.holds(&computed.identity.plan_id) {
        // Every byte the plan will write travels with it, so an apply
        // never has to go back to a bundle it may no longer read.
        let mut blobs = blobs;
        for (digest, bytes) in blobs_for(&computed, release.bundle.as_ref()) {
            blobs.entry(digest).or_insert(bytes);
        }
        store.put(&computed, &blobs)?;
    }

    if args.json {
        return output::json(&computed);
    }
    render(&computed);
    output::line(format!(
        "apply it with: sdd reconcile apply {}",
        computed.identity.plan_id
    ));
    Ok(())
}

fn run_show(args: &ShowArgs) -> Result<(), AppError> {
    let store = Store::new(&state_root()?);
    if let Ok(plan) = store.get(&args.plan_id) {
        if args.json {
            return output::json(&plan);
        }
        output::line("executable plan");
        render(&plan);
        return Ok(());
    }
    let result = store.latest_result(&args.plan_id).ok_or_else(|| {
        AppError::Refused(format!(
            "no plan and no result carry the id {}; run 'sdd reconcile plan' again",
            args.plan_id
        ))
    })?;
    if args.json {
        return output::json(&result);
    }
    output::line("latest result");
    output::line(format!("disposition: {:?}", result.disposition));
    output::line(format!("reason: {}", result.reason));
    Ok(())
}

fn run_apply(ctx: &AppContext, args: &ApplyArgs) -> Result<(), AppError> {
    let target = resolve_target(ctx, &args.target)?;
    let store = Store::new(&state_root()?);
    // The plan's own lock comes first, before the plan is even read: a
    // prune skips a fingerprint somebody holds, and reading first would
    // leave a window where the blobs this apply needs are swept.
    let _plan_lock = Lock::exclusive_waiting(
        &store.plan_lock_path(&args.plan_id)?,
        "reconcile apply",
        STORE_WAIT,
    )?;
    let stored = store.get(&args.plan_id)?;

    // Apply holds the target alone, through observation, execution, and
    // the result write.
    let _lock = Lock::exclusive(&target_lock(&target)?, "reconcile apply")?;
    let selections: decision::Selections = stored
        .decisions
        .iter()
        .filter_map(|decision| {
            decision
                .selected
                .as_ref()
                .map(|answer| (decision.id.clone(), answer.clone()))
        })
        .collect();
    // Apply resolves nothing. It reads the exact release the plan froze,
    // offline, by version rather than by the selector that found it: a
    // `latest` that moved between the plan and the apply would otherwise
    // make the apply resolve a different release and then report the
    // plan as stale, which is the network deciding rather than the plan.
    let frozen = if stored.desired_state.selector == "embedded" {
        // The embedded bundle is this binary's own and has no version to
        // look up. Naming its version instead would send the apply to the
        // registry for a release it already carries.
        "embedded".to_string()
    } else {
        stored.desired_state.release.clone()
    };
    let release = read_release(&frozen, true)?;
    let (recomputed, _) = compute_plan(
        &target,
        &stored.desired_state.selector,
        true,
        &selections,
        &stored.desired_state.reserved,
        None,
        &release.borrow(),
    )?;

    // The store is global, so its own lock orders writers across targets.
    // The order is always target first, then store, on every path.
    let result = execute(&Request {
        store: &store,
        target: &target,
        stored: &stored,
        recomputed: &recomputed,
        bundle: release.bundle.as_ref(),
        now: jiff::Timestamp::now().to_string(),
    })?;
    if args.json {
        return output::json(&result);
    }
    render_result(&result);
    Ok(())
}

fn render_result(result: &ApplyResult) {
    output::line(format!("{:?}: {}", result.disposition, result.reason));
    for operation in &result.operations {
        output::line(format!("  {} {}", operation.kind, operation.path));
    }
    for postcondition in &result.postconditions {
        let word = if postcondition.held { "held" } else { "FAILED" };
        output::line(format!("  {word}: {}", postcondition.id));
    }
}

fn render(plan: &Plan) {
    output::line(format!(
        "{} toward {} ({})",
        plan.classification, plan.desired_state.release, plan.desired_state.selector
    ));
    output::line(format!("plan {}", plan.identity.plan_id));
    let word = match plan.readiness {
        Readiness::Ready => "ready",
        Readiness::NeedsDecision => "needs a decision",
        Readiness::Blocked => "blocked",
    };
    output::line(format!("readiness: {word}"));
    if !plan.findings.is_empty() {
        output::line(format!("findings: {}", plan.findings.len()));
        for found in &plan.findings {
            output::line(format!(
                "  {} {}: {}",
                found.rule, found.path, found.statement
            ));
        }
    }
    if !plan.style_candidates.is_empty() {
        output::line(format!(
            "style candidates: {} (named, never judged)",
            plan.style_candidates.len()
        ));
    }
    output::line(format!("operations: {}", plan.operations.len()));
    for operation in &plan.operations {
        output::line(format!("  {} {}", operation.kind(), operation.path()));
    }
    for precondition in &plan.preconditions {
        if precondition.evaluation.is_satisfied() {
            continue;
        }
        output::line(format!(
            "  blocked by {}: {}",
            precondition.id, precondition.statement
        ));
    }
    for decision in &plan.decisions {
        if decision.selected.is_some() {
            continue;
        }
        output::line(format!("  decide --set {}=<answer>", decision.id));
    }
}
