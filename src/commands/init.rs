//! `init` subcommand: runtime-shape.
//!
//! A front over the engine. It classifies the target first and serves one
//! classification: a first landing. A settled corpus or an installed
//! instance is somebody else's verb, and this one says which rather than
//! landing seeds beside a convention that is already there.
//!
//! Projects the flags into the installer's options and prints its report.
//! Install semantics live in `services::installer`.

use crate::cli::init::InitArgs;
use crate::context::AppContext;
use crate::domain::manifest::parse_docs_scratch;
use crate::error::AppError;
use crate::output;
use crate::plan::classify::Intent;
use crate::services::installer::{InitOptions, init};

/// Install the payload into a target repository.
///
/// # Errors
///
/// Whatever the installer refuses; see [`init`].
pub fn run(_ctx: &AppContext, args: InitArgs) -> Result<(), AppError> {
    let docs_scratch = args
        .docs_scratch
        .as_deref()
        .map(parse_docs_scratch)
        .transpose()
        .map_err(|error| AppError::Usage(format!("--docs-scratch: {error}")))?;
    let writing_style = args
        .writing_style
        .as_deref()
        .map(crate::domain::instance_config::WritingStyle::parse_flag)
        .transpose()
        .map_err(|error| AppError::Usage(format!("--writing-style: {error}")))?;
    let outcome = init(
        &InitOptions {
            target: args.target,
            profile: args.profile,
            apply: args.apply,
            dry_run: args.dry_run,
            docs_scratch,
            reserve: args.reserve,
            writing_style,
        },
        Intent::Init,
    )?;
    for line in &outcome.lines {
        output::line(line);
    }
    Ok(())
}
