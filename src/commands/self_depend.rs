//! `self-depend` subcommand: runtime-shape.
//!
//! Four verbs over one detection. `status` reports and never judges. `add`
//! prints fragments and seeds only where the target has nothing. `sync`
//! moves the pin in one transaction, under a stamp for the shell-entry
//! caller. `clean` removes only what it names.

use std::fmt::Write as _;

use camino::Utf8PathBuf;
use semver::Version;
use serde::Serialize;

use crate::cli::self_depend::{
    AddArgs, Caller, CleanArgs, SelfDependArgs, SelfDependCommand, StatusArgs, SyncArgs,
};
use crate::context::AppContext;
use crate::domain::paths::{CI_VAR, SELF_DEPEND_OFF_VAR, UserEnv};
use crate::error::AppError;
use crate::output;
use crate::release::crates_io::CratesIoResolver;
use crate::self_depend::fragments::{self, Fragment};
use crate::self_depend::leftovers::{self, Kept, Leftover};
use crate::self_depend::manager::{self, Detected, Manager};
use crate::self_depend::status::{self, Freshness, Presence, Report, State};
use crate::self_depend::txn::{self, Moved};
use crate::self_depend::venue::{self, Venue, Verdict};
use crate::self_depend::{ENVRC, parse_tag, stamp};

/// Dispatch one verb.
///
/// # Errors
///
/// Each verb's own, stated on its handler.
pub fn run(ctx: &AppContext, args: SelfDependArgs) -> Result<(), AppError> {
    match args.command {
        SelfDependCommand::Status(args) => run_status(ctx, args),
        SelfDependCommand::Add(args) => run_add(ctx, args),
        SelfDependCommand::Sync(args) => run_sync(ctx, args),
        SelfDependCommand::Clean(args) => run_clean(ctx, args),
    }
}

/// The target, absolute or the working directory.
fn resolve_target(ctx: &AppContext, target: Utf8PathBuf) -> Result<Utf8PathBuf, AppError> {
    if target.is_absolute() {
        Ok(target)
    } else if target == "." {
        Ok(ctx.cwd.clone())
    } else {
        Err(AppError::Usage("target must be absolute or .".to_string()))
    }
}

/// The release this binary is.
fn this_version() -> Version {
    env!("CARGO_PKG_VERSION")
        .parse()
        .unwrap_or_else(|_| Version::new(0, 0, 0))
}

const fn presence_word(presence: Presence) -> &'static str {
    match presence {
        Presence::Present => "present",
        Presence::Absent => "absent",
    }
}

// --- status -----------------------------------------------------------------

/// Report what a target carries, offline.
///
/// # Errors
///
/// [`AppError::Usage`] for a relative target. Every state reports and exits 0.
fn run_status(ctx: &AppContext, args: StatusArgs) -> Result<(), AppError> {
    let target = resolve_target(ctx, args.target)?;
    let mut report = status::report(&target, &UserEnv::from_process(), &this_version());
    if let Some(manager) = args.manager {
        report.managers.retain(|entry| entry.manager == manager);
    }
    if args.json {
        return output::json(&report);
    }
    render_status(&report);
    Ok(())
}

fn render_status(report: &Report) {
    let state = match report.state {
        State::Ready => "ready",
        State::Unwired => "unwired",
        State::LineAbsent => "line-absent",
        State::Ambiguous => "ambiguous",
        State::Leftovers => "leftovers",
    };
    output::line(format!("state {state}"));
    if let Some(wired) = report.wired {
        output::line(format!("wired through {wired}"));
    }
    for entry in &report.managers {
        let mut line = format!("manager {} ", entry.manager);
        match entry.file.as_ref() {
            Some(file) => {
                let _ = write!(line, "present ({file})");
            }
            None => line.push_str("absent"),
        }
        if let Some(version) = entry.version.as_ref() {
            let _ = write!(line, ", pinned {version}");
            if let Some(freshness) = entry.freshness {
                let word = match freshness {
                    Freshness::Current => "current with this binary",
                    Freshness::Behind => "behind this binary",
                    Freshness::Ahead => "ahead of this binary",
                };
                let _ = write!(line, " ({word})");
            }
        }
        if let Some(lock) = entry.lock {
            let _ = write!(line, ", lock {}", presence_word(lock));
        }
        if let Some(rev) = entry.locked_rev.as_ref() {
            let _ = write!(line, ", locked at {rev}");
        }
        output::line(line);
    }
    output::line(format!(
        "{ENVRC} {}, sync line {}",
        presence_word(report.envrc),
        if report.envrc_sync { "yes" } else { "no" }
    ));
    if let Some(stamp) = report.stamp.as_ref() {
        output::line(format!("last sync attempt {stamp}"));
    }
    if report.off {
        output::line(format!(
            "the loop is switched off here: {CI_VAR} or {SELF_DEPEND_OFF_VAR} is set"
        ));
    }
    output::line(format!(
        "host nix {}, direnv {}",
        if report.host.nix { "ok" } else { "absent" },
        if report.host.direnv { "ok" } else { "absent" }
    ));
    for leftover in &report.leftovers {
        output::line(format!("leftover {}: {}", leftover.file, leftover.reason));
    }
    output::line("Next:");
    for line in &report.next {
        output::line(format!("  {line}"));
    }
}

// --- add --------------------------------------------------------------------

/// What `add` served, and what it wrote.
#[derive(Debug, Serialize)]
struct AddReport {
    schema: &'static str,
    target: Utf8PathBuf,
    manager: Manager,
    venue: Venue,
    version: String,
    #[serde(flatten)]
    verdict: Verdict,
    fragments: Vec<Fragment>,
    seeds: Vec<Utf8PathBuf>,
    applied: bool,
}

/// The manager `add` serves: the one asked for, the one already naming
/// this tool, the one manager file the target carries, or the flake.
///
/// SATISFIES acquisition:one-target-runs-one-mechanism
fn choose_manager(asked: Option<Manager>, detected: &[Detected]) -> Result<Manager, AppError> {
    let wired = manager::wired(detected);
    let names = |held: &[&Detected]| -> String {
        held.iter()
            .map(|held| held.manager.as_str())
            .collect::<Vec<_>>()
            .join(" and ")
    };
    match (asked, wired.as_slice()) {
        (Some(asked), [held]) if held.manager != asked => Err(AppError::Refused(format!(
            "{} already pins this tool in this target; one target runs one mechanism, so --manager {asked} is refused",
            held.manager
        ))),
        (Some(asked), _) => Ok(asked),
        (None, [held]) => Ok(held.manager),
        (None, []) => {
            let present: Vec<&Detected> =
                detected.iter().filter(|held| held.file.is_some()).collect();
            match present.as_slice() {
                [] => Ok(Manager::Flake),
                [one] => Ok(one.manager),
                several => Err(AppError::Usage(format!(
                    "the target carries {}; name one with --manager",
                    names(several)
                ))),
            }
        }
        (None, several) => Err(AppError::Refused(format!(
            "{} each pin this tool; one target runs one mechanism",
            names(several)
        ))),
    }
}

/// Serve the fragments for one pair, and seed what the target lacks.
///
/// # Errors
///
/// [`AppError::Usage`] for a relative target, a tag that is not a release,
/// or several managers with none named. [`AppError::Refused`] for a manager
/// other than the one already naming this tool, and for a manual pair.
fn run_add(ctx: &AppContext, args: AddArgs) -> Result<(), AppError> {
    let target = resolve_target(ctx, args.target)?;
    let version = match args.tag.as_deref() {
        Some(tag) => parse_tag(tag).map_err(AppError::Usage)?,
        None => this_version(),
    };
    let detected = manager::detect(&target);
    let manager = choose_manager(args.manager, &detected)?;
    let venue = match args.venue {
        Some(venue) => venue,
        None => venue::default_venue(manager).ok_or_else(|| {
            AppError::Refused(format!(
                "{manager} renders no fragment for any venue; every pair is manual"
            ))
        })?,
    };
    let verdict = venue::verdict(manager, venue);
    if let Verdict::Manual { reason } = verdict {
        return Err(AppError::Refused(format!(
            "{manager} with {venue} is a manual pair: {}",
            reason.as_str()
        )));
    }
    let mut fragments = fragments::render(manager, venue, &version);
    fragments.push(fragments::envrc_fragment());

    // SATISFIES acquisition:the-tool-serves-and-does-not-edit
    let mut seeds: Vec<(Utf8PathBuf, String)> = Vec::new();
    let manager_file_absent = detected
        .iter()
        .find(|held| held.manager == manager)
        .is_none_or(|held| held.file.is_none());
    if manager_file_absent && let Some((file, contents)) = fragments::seed(manager, venue, &version)
    {
        seeds.push((Utf8PathBuf::from(file), contents));
    }
    if manager == Manager::Flake && venue == Venue::Flake && !target.join(ENVRC).exists() {
        seeds.push((Utf8PathBuf::from(ENVRC), fragments::envrc_seed()));
    }
    if args.apply {
        for (file, contents) in &seeds {
            crate::adapters::fs::write_atomic(&target.join(file), contents.as_bytes())?;
        }
    }
    let report = AddReport {
        schema: "sdd.self-depend-add/1",
        target,
        manager,
        venue,
        version: version.to_string(),
        verdict,
        fragments,
        seeds: seeds.iter().map(|(file, _)| file.clone()).collect(),
        applied: args.apply,
    };
    if args.json {
        return output::json(&report);
    }
    render_add(&report);
    Ok(())
}

fn render_add(report: &AddReport) {
    output::line(format!(
        "{} with {} at {}: renders",
        report.manager, report.venue, report.version
    ));
    for fragment in &report.fragments {
        output::line(format!(
            "\n{}: {}, near `{}`",
            fragment.file, fragment.placement, fragment.anchor
        ));
        output::line(fragment.text.trim_end());
    }
    for seed in &report.seeds {
        output::line(format!(
            "\n{seed}: {}",
            if report.applied {
                "seeded"
            } else {
                "would be seeded; pass --apply"
            }
        ));
    }
    if report.seeds.is_empty() {
        output::line("\nnothing to seed: place the fragments in the files the project owns");
    }
}

// --- sync -------------------------------------------------------------------

/// What one sync run concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Outcome {
    /// The loop is switched off in this environment.
    Off,
    /// No manager names this tool, so there is no pin to move.
    Unwired,
    /// The stamp says an attempt already ran today.
    Stamped,
    /// The pin already names the release asked for.
    Current,
    /// The pin would move; `--apply` was not passed.
    Pending,
    /// The pin moved.
    Moved,
}

/// The sync report.
#[derive(Debug, Serialize)]
struct SyncReport {
    schema: &'static str,
    target: Utf8PathBuf,
    caller: &'static str,
    outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    manager: Option<Manager>,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    moved: Option<Moved>,
    message: String,
}

impl SyncReport {
    fn new(target: &Utf8PathBuf, caller: Caller, outcome: Outcome, message: String) -> Self {
        Self {
            schema: "sdd.self-depend-sync/1",
            target: target.clone(),
            caller: match caller {
                Caller::Envrc => "envrc",
                Caller::Operator => "operator",
            },
            outcome,
            manager: None,
            from: None,
            to: None,
            moved: None,
            message,
        }
    }
}

/// Move the pin through the wired manager.
///
/// The shell-entry caller stays silent and exits 0 on every outcome. The
/// operator caller reports every outcome and fails with the matrix below.
///
/// # Errors
///
/// For the operator caller: [`AppError::Usage`] for a relative target, a
/// tag that is not a release, no wired manager, or several managers with
/// none named; [`AppError::Other`] where the latest release cannot be read;
/// [`AppError::Refused`] where the transaction failed and both files were
/// put back.
fn run_sync(ctx: &AppContext, args: SyncArgs) -> Result<(), AppError> {
    let caller = args.caller;
    let json = args.json;
    match sync_inner(ctx, args) {
        Ok(report) => {
            if json {
                output::json(&report)?;
            } else if caller == Caller::Operator {
                output::line(report.message);
            }
            Ok(())
        }
        // SATISFIES acquisition:the-shell-entry-caller-is-rate-limited-and-silent
        Err(_) if caller == Caller::Envrc && !json => Ok(()),
        Err(error) if caller == Caller::Envrc => output::json(&serde_json::json!({
            "schema": "sdd.self-depend-sync/1",
            "caller": "envrc",
            "outcome": "failed",
            "message": error.to_string(),
        })),
        Err(error) => Err(error),
    }
}

/// The one wired manager a sync moves, or why none can be chosen.
///
/// `Ok(None)` is the shell-entry caller's silent answer to a target that
/// pins nothing.
fn choose_wired(
    asked: Option<Manager>,
    detected: &[Detected],
    caller: Caller,
) -> Result<Option<&Detected>, AppError> {
    let wired = manager::wired(detected);
    match (asked, wired.as_slice()) {
        (Some(asked), held) => held
            .iter()
            .find(|held| held.manager == asked)
            .copied()
            .map(Some)
            .ok_or_else(|| {
                AppError::Usage(format!("{asked} does not pin this tool in this target"))
            }),
        (None, [one]) => Ok(Some(one)),
        (None, []) if caller == Caller::Envrc => Ok(None),
        (None, []) => Err(AppError::Usage(
            "no manager names this tool in this target; sdd self-depend add serves the fragments"
                .to_string(),
        )),
        (None, several) => {
            let names: Vec<&str> = several.iter().map(|held| held.manager.as_str()).collect();
            Err(AppError::Usage(format!(
                "{} each pin this tool; name one with --manager",
                names.join(" and ")
            )))
        }
    }
}

fn sync_inner(ctx: &AppContext, args: SyncArgs) -> Result<SyncReport, AppError> {
    let target = resolve_target(ctx, args.target)?;
    let report =
        |outcome: Outcome, message: String| SyncReport::new(&target, args.caller, outcome, message);
    if status::switched_off() {
        return Ok(report(
            Outcome::Off,
            format!("the loop is switched off: {CI_VAR} or {SELF_DEPEND_OFF_VAR} is set"),
        ));
    }
    let detected = manager::detect(&target);
    let Some(held) = choose_wired(args.manager, &detected, args.caller)? else {
        return Ok(report(
            Outcome::Unwired,
            "no manager names this tool, so there is no pin to move".to_string(),
        ));
    };
    let pin = held
        .pin
        .as_ref()
        .ok_or_else(|| AppError::Refused(format!("{} names no pin", held.manager)))?;

    // SATISFIES acquisition:the-shell-entry-caller-is-rate-limited-and-silent
    let env = UserEnv::from_process();
    if args.caller == Caller::Envrc
        && args.tag.is_none()
        && let Some(root) = env.state_root()
    {
        let path = stamp::path(&root.path, &target);
        let today = stamp::today();
        if stamp::attempted(&path, today) {
            return Ok(report(
                Outcome::Stamped,
                format!("an attempt already ran today ({today}); at most one runs a day"),
            ));
        }
        stamp::mark(&path, today)?;
    }

    let to = match args.tag.as_deref() {
        Some(tag) => parse_tag(tag).map_err(AppError::Usage)?,
        None => latest(&env)?,
    };
    let spelled_to = if pin.spelled.starts_with('v') {
        format!("v{to}")
    } else {
        to.to_string()
    };
    let mut out = report(Outcome::Current, String::new());
    out.manager = Some(held.manager);
    out.from = Some(pin.spelled.clone());
    out.to = Some(spelled_to.clone());
    if pin.version == to {
        out.message = format!("{} pins {} already", held.manager, pin.spelled);
        return Ok(out);
    }
    if !args.apply {
        out.outcome = Outcome::Pending;
        out.message = format!(
            "{} would move from {} to {spelled_to}; pass --apply",
            held.manager, pin.spelled
        );
        return Ok(out);
    }
    // SATISFIES acquisition:the-operator-caller-reports-every-outcome
    let moved = txn::sync(&target, held, &to)?;
    out.outcome = Outcome::Moved;
    out.message = format!(
        "{} moved from {} to {}; review and commit {}",
        moved.manager,
        moved.from,
        moved.to,
        moved
            .files
            .iter()
            .map(|file| file.as_str())
            .collect::<Vec<_>>()
            .join(" and ")
    );
    out.moved = Some(moved);
    Ok(out)
}

/// The latest stable release the registry serves.
fn latest(env: &UserEnv) -> Result<Version, AppError> {
    let cache = env
        .user_paths()
        .ok_or_else(|| AppError::Usage("no cache root resolves".to_string()))?
        .bundle_cache
        .path;
    CratesIoResolver::new(&cache)
        .latest_version()
        .map_err(|error| anyhow::anyhow!("the latest release could not be read: {error}").into())
}

// --- clean ------------------------------------------------------------------

/// The clean report.
#[derive(Debug, Serialize)]
struct CleanReport {
    schema: &'static str,
    target: Utf8PathBuf,
    leftovers: Vec<Leftover>,
    kept: Vec<Kept>,
    removed: Vec<Utf8PathBuf>,
    applied: bool,
}

/// Remove what a predecessor left, and name what stays.
///
/// # Errors
///
/// [`AppError::Usage`] for a relative target or an `--also` path that
/// leaves it; [`AppError::Io`] where a removal fails.
fn run_clean(ctx: &AppContext, args: CleanArgs) -> Result<(), AppError> {
    let target = resolve_target(ctx, args.target)?;
    for also in &args.also {
        if also.is_absolute()
            || also
                .components()
                .any(|c| c == camino::Utf8Component::ParentDir)
        {
            return Err(AppError::Usage(format!(
                "--also {also} must be relative and stay inside the target"
            )));
        }
    }
    let detected = manager::detect(&target);
    let wired = manager::wired(&detected);
    let found = leftovers::find(&target, &args.also);
    let kept = leftovers::kept(&target, &wired);
    let mut removed = Vec::new();
    if args.apply {
        // SATISFIES acquisition:clean-removes-only-a-named-leftover
        for leftover in &found {
            std::fs::remove_file(target.join(&leftover.file))?;
            removed.push(leftover.file.clone());
        }
    }
    let report = CleanReport {
        schema: "sdd.self-depend-clean/1",
        target,
        leftovers: found,
        kept,
        removed,
        applied: args.apply,
    };
    if args.json {
        return output::json(&report);
    }
    if report.leftovers.is_empty() {
        output::line("no leftover: this target runs one mechanism");
    }
    for leftover in &report.leftovers {
        output::line(format!(
            "{} {}: {}",
            if args.apply { "removed" } else { "leftover" },
            leftover.file,
            leftover.reason
        ));
    }
    for kept in &report.kept {
        match kept.line.as_ref() {
            Some(line) => output::line(format!("kept {} ({line}): {}", kept.file, kept.reason)),
            None => output::line(format!("kept {}: {}", kept.file, kept.reason)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> AppContext {
        AppContext {
            cwd: Utf8PathBuf::from("/work"),
            verbosity: 0,
        }
    }

    #[test]
    fn a_relative_target_is_a_usage_error() {
        assert!(matches!(
            resolve_target(&ctx(), Utf8PathBuf::from("relative")),
            Err(AppError::Usage(_))
        ));
        assert_eq!(
            resolve_target(&ctx(), Utf8PathBuf::from(".")).ok(),
            Some(Utf8PathBuf::from("/work"))
        );
        assert_eq!(
            resolve_target(&ctx(), Utf8PathBuf::from("/elsewhere")).ok(),
            Some(Utf8PathBuf::from("/elsewhere"))
        );
    }

    /// VERIFIES acquisition:one-target-runs-one-mechanism
    #[test]
    fn a_second_manager_is_refused_where_one_already_pins_this_tool() {
        let pinned = Detected {
            manager: Manager::Mise,
            file: Some(Utf8PathBuf::from("mise.toml")),
            pin: crate::self_depend::pin::read(
                Manager::Mise,
                "[tools]\n\"cargo:spec-driven-docs\" = \"0.1.0\"\n",
            ),
        };
        let detected = vec![pinned];
        assert!(matches!(
            choose_manager(Some(Manager::Flake), &detected),
            Err(AppError::Refused(_))
        ));
        assert_eq!(choose_manager(None, &detected).ok(), Some(Manager::Mise));
        assert_eq!(choose_manager(None, &[]).ok(), Some(Manager::Flake));
    }
}
