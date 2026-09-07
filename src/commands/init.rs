//! `init` subcommand: runtime-shape.
//!
//! Projects the flags into the installer's options and prints its report.
//! Install semantics live in `services::installer`.

use crate::cli::init::InitArgs;
use crate::context::AppContext;
use crate::domain::manifest::{PlanZone, parse_docs_scratch};
use crate::error::AppError;
use crate::output;
use crate::services::installer::{InitOptions, init};

/// Install the payload into a target repository.
///
/// # Errors
///
/// Whatever the installer refuses; see [`init`].
pub fn run(_ctx: &AppContext, args: InitArgs) -> Result<(), AppError> {
    let plan_zone = args
        .plan_zone
        .as_deref()
        .map(PlanZone::parse)
        .transpose()
        .map_err(|error| AppError::Usage(format!("--plan-zone: {error}")))?;
    let docs_scratch = args
        .docs_scratch
        .as_deref()
        .map(parse_docs_scratch)
        .transpose()
        .map_err(|error| AppError::Usage(format!("--docs-scratch: {error}")))?;
    let outcome = init(&InitOptions {
        target: args.target,
        profile: args.profile,
        apply: args.apply,
        dry_run: args.dry_run,
        plan_zone,
        docs_scratch,
    })?;
    for line in &outcome.lines {
        output::line(line);
    }
    Ok(())
}
