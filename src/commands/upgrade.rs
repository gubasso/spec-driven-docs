//! `upgrade` subcommand: runtime-shape.
//!
//! Resolves the target, runs the upgrader, prints its report, and turns
//! conflicts or unfinished removals into the red exit. Upgrade semantics
//! live in `services::upgrader`.

use serde::Serialize;

use crate::cli::upgrade::UpgradeArgs;
use crate::context::AppContext;
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::output;
use crate::services::upgrader::{
    Citation, Conflict, Replaced, UpgradeOptions, UpgradeOutcome, upgrade,
};

/// The machine schema `sdd upgrade --json` declares.
pub const SCHEMA: &str = "sdd.upgrade/1";

/// One upgrade run, as `--json` prints it.
#[derive(Debug, Serialize)]
struct Report<'a> {
    schema: &'static str,
    target: &'a camino::Utf8Path,
    dry_run: bool,
    from: Option<CanonVersion>,
    to: CanonVersion,
    ok: bool,
    conflicts: &'a [Conflict],
    replaced: &'a [Replaced],
    retired_citations: &'a [Citation],
    lines: &'a [String],
}

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
    crate::commands::front::serves(crate::landing::classify::Intent::Upgrade, &target)?;
    let staged = args
        .stage
        .as_deref()
        .map(|root| crate::stage::read_choices(root, &target))
        .transpose()?;
    let outcome = upgrade(&UpgradeOptions {
        target: target.clone(),
        dry_run: args.dry_run,
        staged,
        replace: args.replace,
    })?;
    if args.json {
        output::json(&report(&target, args.dry_run, &outcome))?;
    } else {
        for line in &outcome.lines {
            output::line(line);
        }
    }
    if outcome.failures > 0 {
        return Err(AppError::Violations {
            count: outcome.failures,
        });
    }
    Ok(())
}

fn report<'a>(
    target: &'a camino::Utf8Path,
    dry_run: bool,
    outcome: &'a UpgradeOutcome,
) -> Report<'a> {
    Report {
        schema: SCHEMA,
        target,
        dry_run,
        from: outcome.from,
        to: CanonVersion::current(),
        ok: outcome.failures == 0,
        conflicts: &outcome.conflicts,
        replaced: &outcome.replaced,
        retired_citations: &outcome.retired_citations,
        lines: &outcome.lines,
    }
}
