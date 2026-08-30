//! `hooks` subcommand: runtime-shape.
//!
//! Renders the managed block to stdout. What the render contains is the
//! renderer's business; this handler only projects the flags.

use crate::cli::hooks::HooksArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::hooks_render::{RenderOptions, render_block};

/// Render the delivered gate set.
///
/// # Errors
///
/// None; rendering is pure.
pub fn run(_ctx: &AppContext, args: HooksArgs) -> Result<(), AppError> {
    let rendered = render_block(&RenderOptions {
        docs_root: args.docs_root,
        entry: args.entry,
        indent: args.indent,
    });
    output::line(rendered.trim_end_matches('\n'));
    Ok(())
}
