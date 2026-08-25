//! `gate` subcommand: runtime-shape.
//!
//! Runs one delivered gate against the invoking repository, printing every
//! violation to stdout, or lists the registry. Gate semantics live in
//! `gates`; this handler only dispatches and reports.

use crate::cli::gate::GateArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::gates::{GateCtx, spec};
use crate::output;

/// Run or list gates.
///
/// # Errors
///
/// [`AppError::Violations`] when the gate found any; I/O errors when it
/// could not run.
pub fn run(_ctx: &AppContext, args: GateArgs) -> Result<(), AppError> {
    let GateArgs { id, files, list } = args;
    if list {
        for gate in crate::gates::GATES {
            output::line(format!("{}: {}", gate.id, gate.name));
        }
        return Ok(());
    }
    let Some(id) = id else {
        return Err(AppError::Usage(
            "a gate id or --list is required".to_string(),
        ));
    };
    let gate_ctx = GateCtx::new(".");
    let violations = (spec(id).run)(&gate_ctx, &files)?;
    if violations.is_empty() {
        return Ok(());
    }
    for violation in &violations {
        output::line(violation);
    }
    Err(AppError::Violations {
        count: violations.len(),
    })
}
