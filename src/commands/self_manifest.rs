//! `self-manifest` subcommand: runtime-shape.
//!
//! Writes the payload record of the canon checkout; refuses anywhere else. The generation lives in `services::self_manifest`.

use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::self_manifest::regenerate;

/// Write the canon checkout's payload record.
///
/// # Errors
///
/// [`AppError::Refused`] outside the canon checkout; I/O errors otherwise.
pub fn run(ctx: &AppContext) -> Result<(), AppError> {
    output::line(regenerate(&ctx.cwd)?);
    Ok(())
}
