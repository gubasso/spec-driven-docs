//! The embedded-document readers: runtime-shape.
//!
//! One handler serves `method`, `spec`, `template`, and `migration` — the
//! shelf is the only difference. Shelf semantics live in
//! `services::reader`.

use crate::cli::read::ReadArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::reader::{Shelf, get, list};

/// Read one document from a shelf, or list it.
///
/// # Errors
///
/// [`AppError::Usage`] when the name resolves to nothing.
pub fn run(_ctx: &AppContext, shelf: &Shelf, args: ReadArgs) -> Result<(), AppError> {
    if args.list {
        for name in list(shelf) {
            output::line(name);
        }
        return Ok(());
    }
    let Some(name) = args.name else {
        return Err(AppError::Usage(
            "a document name or --list is required".to_string(),
        ));
    };
    let Some(text) = get(shelf, &name) else {
        return Err(AppError::Usage(format!("no such document: {name}")));
    };
    output::line(text.trim_end_matches('\n'));
    Ok(())
}
