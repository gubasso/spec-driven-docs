//! `verify` subcommand: runtime-shape.
//!
//! Resolves the target, runs the verifier, prints its report, and turns any
//! failure into the red exit. Verification semantics live in
//! `services::verifier`.

use crate::cli::verify::VerifyArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::verifier::verify;

/// Verify an installed instance offline.
///
/// # Errors
///
/// [`AppError::Violations`] when checks failed; manifest and I/O errors
/// when the verifier could not run.
pub fn run(ctx: &AppContext, args: VerifyArgs) -> Result<(), AppError> {
    let target = if args.target.is_absolute() {
        args.target
    } else if args.target == "." {
        ctx.cwd.clone()
    } else {
        return Err(AppError::Usage("target must be absolute or .".to_string()));
    };
    // A pending move warns and never fails, because verify runs as a hook.
    // SATISFIES acquisition:every-report-names-a-pending-move
    if let Some(pending) =
        crate::self_depend::status::pending(&target, &crate::domain::paths::UserEnv::from_process())
    {
        eprintln!("{}", pending.text);
    }
    let report = verify(&target)?;
    for line in &report.lines {
        output::line(line);
    }
    if report.failures > 0 {
        return Err(AppError::Violations {
            count: report.failures,
        });
    }
    Ok(())
}
