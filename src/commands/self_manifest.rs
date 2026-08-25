//! `self-manifest` subcommand: runtime-shape.
//!
//! Regenerates the canon's own manifest from its payload; refuses anywhere
//! else. The generation lives in `services::self_manifest`.

use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::self_manifest::regenerate;

/// Regenerate the canon checkout's instance manifest.
///
/// # Errors
///
/// [`AppError::Refused`] outside the canon checkout; I/O errors otherwise.
pub fn run(ctx: &AppContext) -> Result<(), AppError> {
    output::line(regenerate(&ctx.cwd)?);
    Ok(())
}
