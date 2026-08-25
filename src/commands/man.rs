//! `man` subcommand: runtime-shape.
//!
//! Renders the manual page for the full command surface to stdout.

use clap::CommandFactory;

use crate::context::AppContext;
use crate::error::AppError;
use crate::output;

/// Render the manual page.
///
/// # Errors
///
/// [`AppError::Io`] when rendering fails.
pub fn run(_ctx: &AppContext) -> Result<(), AppError> {
    let mut buffer = Vec::new();
    clap_mangen::Man::new(crate::cli::Cli::command()).render(&mut buffer)?;
    output::raw(&buffer);
    Ok(())
}
