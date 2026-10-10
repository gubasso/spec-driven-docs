//! `lint` subcommand: runtime-shape.
//!
//! Resolves the target, asks the lint service for the report or the proof,
//! and renders text or JSON. The report exits 0 for every finding, because
//! a report is not a gate. A proof that fails is a check that found its
//! violation, so it exits 1.

use camino::Utf8PathBuf;

use crate::cli::lint::{LintArgs, LintCommand};
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::lint::{prove, report};

/// Report the linter's configuration, or prove its link rule runs.
///
/// # Errors
///
/// [`AppError::Violations`] where the proof fails, and whatever the report
/// or the proof refuses.
pub fn run(ctx: &AppContext, args: LintArgs) -> Result<(), AppError> {
    if let Some(LintCommand::Prove(proof)) = args.command {
        return proved(&resolved(ctx, proof.target)?, proof.json);
    }
    let target = resolved(ctx, args.target)?;
    let report = report(&target, args.profile)?;
    if args.json {
        return output::json(&report);
    }
    output::line(format!("profile: {}", report.profile));
    for finding in &report.findings {
        let edit = serde_json::to_value(finding.edit)
            .ok()
            .and_then(|value| value.as_str().map(String::from))
            .unwrap_or_default();
        output::line(format!("{edit} {} ({})", finding.path, finding.rule));
    }
    if report.findings.is_empty() {
        output::line("no finding: the linter reads the delivered configuration as landed");
    }
    Ok(())
}

fn proved(target: &camino::Utf8Path, json: bool) -> Result<(), AppError> {
    let proof = prove(target)?;
    if json {
        output::json(&proof)?;
    } else if proof.ok {
        output::line(format!(
            "OK relative-links failed a broken link and passed a valid one ({})",
            proof.rule
        ));
    } else {
        output::line(format!(
            "FAIL relative-links did not fail a broken link and pass a valid one; the rule does not run here ({})",
            proof.rule
        ));
    }
    if !proof.ok {
        return Err(AppError::Violations { count: 1 });
    }
    Ok(())
}

/// The target, absolute or the working directory.
fn resolved(ctx: &AppContext, target: Utf8PathBuf) -> Result<Utf8PathBuf, AppError> {
    let target = if target.is_absolute() {
        target
    } else if target == "." {
        ctx.cwd.clone()
    } else {
        return Err(AppError::Usage("target must be absolute or .".to_string()));
    };
    if !target.is_dir() {
        return Err(AppError::Usage(format!("unresolved target: {target}")));
    }
    Ok(target)
}
