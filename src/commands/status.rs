//! `status` subcommand: runtime-shape.
//!
//! Resolves the target, asks the status service, and renders text or JSON.
//! Status is a report, not a gate: drift never turns into a failing exit.

use crate::cli::status::StatusArgs;
use crate::context::AppContext;
use crate::domain::paths::{PathEntry, PathSource, Paths};
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
        render_paths(&report.paths);
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
    render_paths(&report.paths);
    Ok(())
}

/// One entry line: what it is, where it resolved, and what decided it.
fn entry(label: &str, path: &PathEntry) {
    output::line(format!(
        "  {label}: {} ({})",
        path.path,
        source_word(path.source)
    ));
}

const fn source_word(source: PathSource) -> &'static str {
    match source {
        PathSource::Recorded => "recorded",
        PathSource::Default => "default",
        PathSource::Env => "env",
        PathSource::Profile => "profile",
        PathSource::Proposal => "proposal",
    }
}

/// Print every path the report carries, under one heading.
///
/// The text form is for a person reading a terminal. A skill reads
/// `--json`, where every field is named and nothing is abbreviated.
fn render_paths(paths: &Paths) {
    output::line("paths:");
    if let Some(user) = paths.user.as_ref() {
        entry("state root", &user.state_root);
        entry("cache root", &user.cache_root);
        entry("skill receipt", &user.skill_receipt);
        entry("plan store", &user.plan_store);
        entry("bundle cache", &user.bundle_cache);
        for root in &user.agent_roots {
            let moved = root
                .variable
                .map_or_else(|| source_word(root.source).to_string(), str::to_string);
            output::line(format!("  agent root {}: {} ({moved})", root.id, root.path));
        }
    }
    let Some(active) = paths.active.as_ref() else {
        for (name, candidate) in &paths.candidates {
            output::line(format!(
                "  candidate {name}: {}",
                candidate.destinations.docs_root.path
            ));
        }
        return;
    };
    let held = &active.destinations;
    entry("instance directory", &held.instance_dir);
    entry("declaration", &held.declaration);
    entry("debt", &held.debt);
    entry("hook configuration", &held.hooks_config);
    entry("agent digest", &held.agents_digest);
    entry("documentation root", &held.docs_root);
}
