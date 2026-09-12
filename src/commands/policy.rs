//! `policy` subcommand: runtime-shape.
//!
//! One operator-invoked verb offers to bring an older adopted specification
//! into agreement with the project's configuration. It previews first and
//! writes only on request. The boundary is the actor, not the file: an
//! automatic write during install or upgrade is forbidden, and an operator
//! asking for a specific, previewed change is not. What needs reconciling,
//! and how each specification is rewritten, is `services::policy`'s
//! business; this handler resolves the target, prints the plan, and owns
//! the apply.

use camino::{Utf8Path, Utf8PathBuf};

use crate::cli::policy::{PolicyArgs, PolicyVerb, ReconcileArgs};
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::policy::{Action, Plan};

fn resolve_target(ctx: &AppContext, target: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    if target.is_absolute() {
        Ok(target.to_path_buf())
    } else if target == "." {
        Ok(ctx.cwd.clone())
    } else {
        Err(AppError::Usage("target must be absolute or .".to_string()))
    }
}

fn print_plan(plan: &Plan) {
    match &plan.action {
        Action::Seed { destination, .. } => {
            output::line(format!(
                "{destination}: absent; write the seed, which defines `{}`",
                plan.reconciliation.sentinel.rule
            ));
        }
        Action::Append {
            destination, block, ..
        } => {
            output::line(format!(
                "{destination}: append `{}` to its Requirements section:",
                plan.reconciliation.sentinel.rule
            ));
            for line in block.lines() {
                output::line(format!("  {line}"));
            }
        }
        Action::Checklist { destination, block } => {
            output::line(format!(
                "{destination}: not in a shape this command rewrites; add `{}` by hand:",
                plan.reconciliation.sentinel.rule
            ));
            for line in block.lines() {
                output::line(format!("  {line}"));
            }
        }
    }
}

fn reconcile(ctx: &AppContext, args: &ReconcileArgs) -> Result<(), AppError> {
    let target = resolve_target(ctx, &args.target)?;
    let manifest = crate::services::verifier::read_manifest(&target)?;
    let plans = crate::services::policy::plan(&target, manifest.docs_root)?;
    if plans.is_empty() {
        output::line("OK every active declaration is authorized by a local specification");
        return Ok(());
    }
    for plan in &plans {
        print_plan(plan);
    }
    let checklist: Vec<String> = plans
        .iter()
        .filter_map(|plan| match &plan.action {
            Action::Checklist { destination, .. } => Some(destination.to_string()),
            Action::Seed { .. } | Action::Append { .. } => None,
        })
        .collect();
    if !args.apply {
        output::line("DRY RUN: no files written");
        return Ok(());
    }
    // A specification this command cannot rewrite safely stops the whole
    // apply. Printing a correct checklist is a better outcome than a clever
    // rewrite that loses an edit the project made, and writing the others
    // while one waits would leave the operator with two states to track.
    if !checklist.is_empty() {
        return Err(AppError::Refused(format!(
            "nothing written; add the rule by hand to: {}",
            checklist.join(", ")
        )));
    }
    for written in crate::services::policy::apply_all(&target, &plans)? {
        output::line(format!("OK wrote {written}"));
    }
    Ok(())
}

/// Reconcile.
///
/// # Errors
///
/// [`AppError::Refused`] when a specification is not in a shape the
/// command rewrites, a destination leaves the target, or a rewrite would
/// not parse, with the target restored; manifest and I/O errors when the
/// instance cannot be read.
pub fn run(ctx: &AppContext, args: PolicyArgs) -> Result<(), AppError> {
    match args.verb {
        PolicyVerb::Reconcile(args) => reconcile(ctx, &args),
    }
}
