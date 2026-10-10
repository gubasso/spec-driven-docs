//! `env` subcommand: runtime-shape.
//!
//! Resolves the target, asks the env service for the report or one write,
//! and renders text or JSON. The report is not a gate: every state exits 0.

use crate::cli::env::EnvArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::env::{ItemState, Loader, apply, report};

/// Report the environment items, or write one.
///
/// # Errors
///
/// [`AppError::Usage`] for a target the arguments cannot mean, and whatever
/// the apply refuses.
pub fn run(ctx: &AppContext, args: EnvArgs) -> Result<(), AppError> {
    let target = if args.target.is_absolute() {
        args.target
    } else if args.target == "." {
        ctx.cwd.clone()
    } else {
        return Err(AppError::Usage("target must be absolute or .".to_string()));
    };
    if !target.is_dir() {
        return Err(AppError::Usage(format!("unresolved target: {target}")));
    }
    let report = match args.apply {
        Some(item) => apply(&target, item, args.value.as_deref())?,
        None => report(&target),
    };
    if args.json {
        return output::json(&report);
    }
    let loader = match report.loader {
        Loader::Direnv => "direnv (.envrc)",
        Loader::Dotenv => "dotenv (.env)",
        Loader::None => "none",
    };
    output::line(format!("loader: {loader}"));
    for item in &report.items {
        let state = match item.state {
            ItemState::Wired => "wired",
            ItemState::Unwired => "unwired",
            ItemState::NoLoader => "no loader",
            ItemState::Outside => "outside the repository",
            ItemState::Undeclared => "no docs scratch declared",
        };
        let value = item
            .value
            .as_deref()
            .map_or_else(String::new, |value| format!(": {value}"));
        output::line(format!("{}: {state}{value}", item.item.as_str()));
    }
    for line in &report.wrote {
        output::line(format!("wrote {line}"));
    }
    Ok(())
}
