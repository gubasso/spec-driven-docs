//! `assess` subcommand: runtime-shape.
//!
//! Resolves the target, asks the assess service, and renders text or JSON.
//! Assess is a report, not a gate: every classification exits 0.

use crate::cli::assess::AssessArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::assess::assess;

/// Classify a target repository.
///
/// # Errors
///
/// [`AppError::ManifestInvalid`] when an instance manifest exists but
/// cannot be trusted, and I/O errors from the walk.
pub fn run(ctx: &AppContext, args: AssessArgs) -> Result<(), AppError> {
    let target = if args.target.is_absolute() {
        args.target
    } else if args.target == "." {
        ctx.cwd.clone()
    } else {
        return Err(AppError::Usage("target must be absolute or .".to_string()));
    };
    let report = assess(&target)?;
    if args.json {
        return output::json(&report);
    }
    output::line(format!(
        "classification: {}",
        report.classification.as_str()
    ));
    output::line(format!("instance: {}", report.instance.instance));
    output::line(format!("doc roots: {}", join_or_none(&report.doc_roots)));
    output::line(format!(
        "populated doc roots: {}",
        join_or_none(&report.populated_doc_roots)
    ));
    output::line(format!("document files: {}", report.documents.count));
    output::line(format!(
        "methodology markers: {}",
        join_or_none(&report.methodology_markers)
    ));
    for (profile, existing) in &report.collisions {
        output::line(format!("collisions ({profile}): {}", existing.len()));
    }
    Ok(())
}

fn join_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}
