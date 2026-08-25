//! `hooks` subcommand: runtime-shape.
//!
//! Renders the registry in the requested style to stdout. What the render
//! contains is the renderer's business; this handler only projects the
//! flags.

use crate::cli::hooks::HooksArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::hooks_render::{RenderOptions, Style, render_block, render_gates};

/// Render the delivered gate set.
///
/// # Errors
///
/// None; rendering is pure.
pub fn run(_ctx: &AppContext, args: HooksArgs) -> Result<(), AppError> {
    let options = RenderOptions {
        style: args.style,
        docs_root: args.docs_root,
        entry: args.entry,
        language: args.language,
        indent: args.indent,
    };
    let rendered = match options.style {
        Style::Block => render_block(&options),
        Style::Manifest => render_gates(&options),
    };
    output::line(rendered.trim_end_matches('\n'));
    Ok(())
}
