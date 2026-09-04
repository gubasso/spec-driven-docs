//! `ki` subcommand: runtime-shape.
//!
//! Resolves the target, asks the known-issues service, and renders text or
//! JSON. The listing is a report: a record stating no axis is listed with
//! the value missing, because judging it belongs to `ki-state` and
//! `ki-filing`.

use crate::cli::ki::{KiArgs, KiCommand, ListArgs};
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::known_issues::cases;

/// Run one `sdd ki` verb.
///
/// # Errors
///
/// [`AppError::Usage`] when the target is neither absolute nor `.`, and
/// [`AppError::Io`] when a record cannot be read.
pub fn run(ctx: &AppContext, args: KiArgs) -> Result<(), AppError> {
    match args.command {
        KiCommand::List(args) => list(ctx, args),
    }
}

fn list(ctx: &AppContext, args: ListArgs) -> Result<(), AppError> {
    let target = if args.target.is_absolute() {
        args.target
    } else if args.target == "." {
        ctx.cwd.clone()
    } else {
        return Err(AppError::Usage("target must be absolute or .".to_string()));
    };
    if !target.is_dir() {
        return Err(AppError::Usage(format!("no directory at {target}")));
    }
    let cases = cases(&target)?;
    if args.json {
        return output::json(&cases);
    }
    let width = cases.iter().map(|case| case.id.len()).max().unwrap_or(0);
    for case in &cases {
        output::line(format!(
            "{:width$}  {:<13}  {:<9}  {}",
            case.id,
            case.state.as_deref().unwrap_or("-"),
            case.filing.as_deref().unwrap_or("-"),
            case.upstream.as_deref().unwrap_or("-"),
        ));
    }
    Ok(())
}
