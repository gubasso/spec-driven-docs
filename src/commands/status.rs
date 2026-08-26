//! `status` subcommand: runtime-shape.
//!
//! Resolves the target, asks the status service, and renders text or JSON.
//! Status is a report, not a gate: drift never turns into a failing exit.

use crate::cli::status::StatusArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::status::status;

/// Report an instance's state.
///
/// # Errors
///
/// [`AppError::ManifestInvalid`] when a manifest exists but cannot be
/// trusted; manifest absence reports instead of failing.
pub fn run(ctx: &AppContext, args: StatusArgs) -> Result<(), AppError> {
    let target = if args.target.is_absolute() {
        args.target
    } else if args.target == "." {
        ctx.cwd.clone()
    } else {
        return Err(AppError::Usage("target must be absolute or .".to_string()));
    };
    let report = status(&target)?;
    if args.json {
        return output::json(&report);
    }
    if !report.instance {
        output::line(format!("no instance at {target}"));
        return Ok(());
    }
    let profile = report.profile.map_or("unknown", |profile| profile.as_str());
    if let Some(version) = report.canon_version {
        output::line(format!(
            "spec-driven-docs {version} ({profile}) at {target}"
        ));
    }
    if let Some(alignment) = report.alignment {
        let word = match alignment {
            crate::services::status::Alignment::Aligned => "aligned",
            crate::services::status::Alignment::BinaryNewer => "binary newer; run 'sdd upgrade'",
            crate::services::status::Alignment::InstanceNewer => "instance newer; upgrade sdd",
        };
        output::line(format!("alignment: {word}"));
    }
    output::line(format!(
        "managed drift: {}; adopted drift: {}; failures: {}",
        report.managed_drift, report.adopted_drift, report.failures
    ));
    Ok(())
}
