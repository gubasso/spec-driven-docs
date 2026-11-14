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
    // SATISFIES acquisition:every-report-names-a-pending-move
    if let Some(pending) = report.pending.as_ref() {
        output::line(&pending.text);
    }
    match report.classification {
        Some(classification) => output::line(format!("classification: {classification}")),
        None => output::line("classification: none; the instance is newer than this binary"),
    }
    if let Some(reason) = report.invalid.as_ref() {
        output::line(format!("invalid: {reason}"));
    }
    for legacy in &report.legacy_state {
        let binds = if legacy.active {
            "; it still binds a run only the older release can finish"
        } else {
            ""
        };
        output::line(format!(
            "legacy state ({}): {}; no verb of this release reads it{binds}",
            legacy.kind, legacy.path
        ));
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
            crate::services::status::Alignment::Aligned => "aligned".to_string(),
            // SATISFIES setup:a-gated-command-names-its-fix
            crate::services::status::Alignment::BinaryNewer => {
                format!("binary newer. {}", crate::self_depend::guidance::MOVED)
            }
            crate::services::status::Alignment::InstanceNewer => {
                "instance newer; upgrade sdd".to_string()
            }
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
        if let Some(receipt) = &user.skill_receipt {
            entry("skill receipt", receipt);
        }
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
    entry("lint configuration", &held.lint_config);
}
