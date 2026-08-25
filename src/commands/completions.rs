//! `completions` subcommand: runtime-shape.
//!
//! Generates shell completions for the full command surface to stdout.

use clap::CommandFactory;

use crate::cli::completions::CompletionsArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;

/// Generate completions for one shell.
///
/// # Errors
///
/// None.
pub fn run(_ctx: &AppContext, args: CompletionsArgs) -> Result<(), AppError> {
    let mut buffer = Vec::new();
    clap_complete::generate(
        args.shell,
        &mut crate::cli::Cli::command(),
        "sdd",
        &mut buffer,
    );
    output::raw(&buffer);
    Ok(())
}
