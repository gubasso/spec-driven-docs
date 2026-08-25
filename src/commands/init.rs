//! `init` subcommand: runtime-shape.
//!
//! Projects the flags into the installer's options and prints its report.
//! Install semantics live in `services::installer`.

use crate::cli::init::InitArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::installer::{InitOptions, init};

/// Install the payload into a target repository.
///
/// # Errors
///
/// Whatever the installer refuses; see [`init`].
pub fn run(_ctx: &AppContext, args: InitArgs) -> Result<(), AppError> {
    let outcome = init(&InitOptions {
        target: args.target,
        profile: args.profile,
        apply: args.apply,
        dry_run: args.dry_run,
    })?;
    for line in &outcome.lines {
        output::line(line);
    }
    Ok(())
}
