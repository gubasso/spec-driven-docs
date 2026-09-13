//! The embedded-document readers: runtime-shape.
//!
//! One handler serves `method`, `spec`, and `template` — the shelf is the
//! only difference. Shelf semantics live in `services::reader`.
//!
//! A listing renders each name with the catalog's summary, from the same
//! source `sdd docs` reads, so a name described in two places cannot
//! disagree. A shelf document the catalog omits is a canon-test failure
//! rather than a runtime one, and the listing prints the bare name.

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
        let names = list(shelf);
        let width = names.iter().map(String::len).max().unwrap_or(0);
        for name in names {
            match crate::domain::docs_catalog::CATALOG.for_document(shelf.id, &name) {
                Some(topic) => output::line(format!("{name:width$}  {}", topic.summary)),
                None => output::line(name),
            }
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
