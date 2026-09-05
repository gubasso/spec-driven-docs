//! `track` subcommand: runtime-shape.
//!
//! `status` reads the registry and reports freshness offline. `check` is the
//! one network-enabled operation: it compares each pinned Git revision to its
//! upstream through the `git` adapter and writes nothing. Registry parsing
//! and freshness live in `services::tracking`; the network boundary lives in
//! `adapters::git`.

use camino::{Utf8Path, Utf8PathBuf};
use jiff::civil::Date;

use crate::adapters::git::{self, GitError};
use crate::cli::track::{CheckArgs, StatusArgs, TrackArgs, TrackCommand};
use crate::context::AppContext;
use crate::error::AppError;
use crate::gates::GateCtx;
use crate::gates::paths::docs_root;
use crate::output;
use crate::services::tracking::{Freshness, evaluate, today_utc};

/// Dispatch `sdd track`.
///
/// # Errors
///
/// [`AppError`] per the operation: usage errors, I/O, a `git` failure, or
/// [`AppError::Violations`] when `--fail-on-update` sees a moved revision.
pub fn run(ctx: &AppContext, args: TrackArgs) -> Result<(), AppError> {
    match args.command {
        TrackCommand::Status(a) => status(ctx, a),
        TrackCommand::Check(a) => check(ctx, a),
    }
}

fn resolve(ctx: &AppContext, target: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    if target.is_absolute() {
        Ok(target.to_path_buf())
    } else if target == "." {
        Ok(ctx.cwd.clone())
    } else {
        Err(AppError::Usage("target must be absolute or .".to_string()))
    }
}

#[allow(clippy::option_if_let_else)]
fn as_of(value: Option<&str>) -> Result<Date, AppError> {
    match value {
        None => Ok(today_utc()),
        Some(text) => text
            .parse::<Date>()
            .map_err(|_| AppError::Usage(format!("--as-of is not an ISO date: {text}"))),
    }
}

fn git_error(error: &GitError) -> AppError {
    let code = match error {
        GitError::Unsupported(_) => 64,
        GitError::MissingGit | GitError::Transport(_) => 69,
        GitError::Malformed => 65,
        GitError::Timeout => 75,
    };
    AppError::Git {
        message: error.to_string(),
        code,
    }
}

fn status(ctx: &AppContext, args: StatusArgs) -> Result<(), AppError> {
    let StatusArgs {
        target,
        as_of: as_of_arg,
        json,
    } = args;
    let target = resolve(ctx, &target)?;
    let root = docs_root(&GateCtx::new(target.clone()));
    let report = evaluate(&target, &root, as_of(as_of_arg.as_deref())?)?;

    if let Some(fatal) = &report.fatal {
        return Err(AppError::ManifestInvalid(fatal.detail.clone()));
    }

    if json {
        let entries: Vec<serde_json::Value> = report
            .entries
            .iter()
            .map(|a| {
                let state = match a.freshness {
                    Some(Freshness::Current) => "current",
                    Some(Freshness::Overdue(_)) => "overdue",
                    None => "invalid",
                };
                serde_json::json!({
                    "id": a.entry.id,
                    "state": state,
                    "next_check": a.next_check.map(|d| d.to_string()),
                    "revision": a.entry.source.as_ref().map(|s| s.revision.clone()),
                    "revalidate": a.entry.revalidate,
                })
            })
            .collect();
        return output::json(&serde_json::json!({ "ok": true, "tracked": entries }));
    }

    for a in &report.entries {
        let state = match a.freshness {
            Some(Freshness::Current) => "current".to_string(),
            Some(Freshness::Overdue(days)) => format!("overdue by {days} day(s)"),
            None => "invalid date".to_string(),
        };
        let next = a
            .next_check
            .map_or_else(|| "-".to_string(), |d| d.to_string());
        output::line(format!("{}: {state}; next check {next}", a.entry.id));
        if let Some(source) = &a.entry.source {
            output::line(format!(
                "  pinned {} at {}",
                source.reference, source.revision
            ));
        }
        if !matches!(a.freshness, Some(Freshness::Current)) {
            for step in &a.entry.revalidate {
                output::line(format!("  revalidate: {step}"));
            }
        }
    }
    Ok(())
}

fn check(ctx: &AppContext, args: CheckArgs) -> Result<(), AppError> {
    let target = resolve(ctx, &args.target)?;
    let root = docs_root(&GateCtx::new(target.clone()));
    let report = evaluate(&target, &root, today_utc())?;
    if let Some(fatal) = &report.fatal {
        return Err(AppError::ManifestInvalid(fatal.detail.clone()));
    }

    let mut results = Vec::new();
    let mut moved = false;
    let mut matched = false;
    for a in &report.entries {
        if args.id.as_deref().is_some_and(|id| id != a.entry.id) {
            continue;
        }
        let Some(source) = &a.entry.source else {
            continue;
        };
        matched = true;
        let observed =
            git::ls_remote(&source.repository, &source.reference).map_err(|e| git_error(&e))?;
        let (state, observed_str) = match observed {
            None => ("missing-reference", String::new()),
            Some(sha) if sha == source.revision => ("current", sha),
            Some(sha) => {
                moved = true;
                ("moved", sha)
            }
        };
        results.push((
            a.entry.id.clone(),
            source.revision.clone(),
            state,
            observed_str,
        ));
    }

    if args.id.is_some() && !matched {
        return Err(AppError::Usage(format!(
            "no tracked Git entry with id {}",
            args.id.unwrap_or_default()
        )));
    }

    if args.json {
        let entries: Vec<serde_json::Value> = results
            .iter()
            .map(|(id, pinned, state, observed)| {
                serde_json::json!({
                    "id": id,
                    "pinned": pinned,
                    "state": state,
                    "observed": if observed.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(observed.clone()) },
                })
            })
            .collect();
        output::json(&serde_json::json!({ "ok": true, "checked": entries }))?;
    } else {
        for (id, pinned, state, observed) in &results {
            match *state {
                "current" => output::line(format!("{id}: current at {pinned}")),
                "moved" => output::line(format!("{id}: moved from {pinned} to {observed}")),
                _ => output::line(format!(
                    "{id}: reference missing upstream (pinned {pinned})"
                )),
            }
        }
    }

    if args.fail_on_update && moved {
        return Err(AppError::Violations {
            count: results.iter().filter(|(_, _, s, _)| *s == "moved").count(),
        });
    }
    Ok(())
}
