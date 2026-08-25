//! `upgrade` subcommand: runtime-shape.
//!
//! Resolves the target, runs the upgrader, prints its report, and turns
//! conflicts or unfinished removals into the red exit. Upgrade semantics
//! live in `services::upgrader`.

use crate::cli::upgrade::UpgradeArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::upgrader::{UpgradeOptions, upgrade};

/// Upgrade an installed instance to this binary's version.
///
/// # Errors
///
/// [`AppError::Violations`] on conflicts or unfinished removals; whatever
/// the upgrader refuses otherwise.
pub fn run(ctx: &AppContext, args: UpgradeArgs) -> Result<(), AppError> {
    let target = if args.target.is_absolute() {
        args.target
    } else if args.target == "." {
        ctx.cwd.clone()
    } else {
        return Err(AppError::Usage("target must be absolute or .".to_string()));
    };
    let outcome = upgrade(&UpgradeOptions {
        target,
        dry_run: args.dry_run,
    })?;
    for line in &outcome.lines {
        output::line(line);
    }
    if outcome.failures > 0 {
        return Err(AppError::Violations {
            count: outcome.failures,
        });
    }
    Ok(())
}
