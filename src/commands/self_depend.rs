//! `self-depend` subcommand: runtime-shape.
//!
//! Five verbs over one detection. `status` reports and never judges. `add`
//! prints fragments and seeds only where the target has nothing. `check`
//! warns from a cache on every directory entry and writes nothing else.
//! `sync` is the operator's act that moves the pin in one transaction.
//! `clean` removes or swaps only what it names.

use std::fmt::Write as _;

use camino::Utf8PathBuf;
use semver::Version;
use serde::Serialize;

use crate::cli::self_depend::{
    AddArgs, Caller, CheckArgs, CleanArgs, SelfDependArgs, SelfDependCommand, StatusArgs, SyncArgs,
};
use crate::context::AppContext;
use crate::domain::paths::{CI_VAR, DIRENV_IN_ENVRC_VAR, SELF_DEPEND_OFF_VAR, UserEnv, variable};
use crate::error::AppError;
use crate::output;
use crate::self_depend::cache::{self, Entry};
use crate::self_depend::fragments::{self, Fragment};
use crate::self_depend::leftovers::{self, Kept, Leftover, Swap};
use crate::self_depend::manager::{self, Detected, Manager};
use crate::self_depend::registry::Index;
use crate::self_depend::status::{self, Freshness, Presence, Report, State};
use crate::self_depend::txn::{self, Moved};
use crate::self_depend::venue::{self, Venue, Verdict};
use crate::self_depend::{CHECK_LINE, ENVRC, OLD_SYNC_LINE, SYNC_COMMAND, parse_tag};

/// Dispatch one verb.
///
/// # Errors
///
/// Each verb's own, stated on its handler.
pub fn run(ctx: &AppContext, args: SelfDependArgs) -> Result<(), AppError> {
    match args.command {
        SelfDependCommand::Status(args) => run_status(ctx, args),
        SelfDependCommand::Add(args) => run_add(ctx, args),
        SelfDependCommand::Check(args) => {
            run_check(ctx, args);
            Ok(())
        }
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
        State::OldLine => "old-line",
        State::Ambiguous => "ambiguous",
        State::Leftovers => "leftovers",
        State::Host => "host",
    };
    output::line(format!("state {state}"));
    if let Some(wired) = report.wired {
        output::line(format!("wired through {wired}"));
    }
    if let Some(decision) = report.acquisition.as_ref() {
        output::line(format!(
            "host install recorded, at sdd {} or newer",
            decision.minimum
        ));
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
        "{ENVRC} {}, check line {}",
        presence_word(report.envrc),
        if report.envrc_check { "yes" } else { "no" }
    ));
    if report.envrc_old_line {
        output::line(format!(
            "{ENVRC} still runs the retired line `{OLD_SYNC_LINE}`"
        ));
    }
    if let Some(check) = report.check.as_ref() {
        if let Some(success) = check.last_success.as_ref() {
            output::line(format!(
                "last version check {success}, latest {}",
                check.latest.as_deref().unwrap_or("unknown")
            ));
        }
        if let Some(error) = check.error.as_ref() {
            output::line(format!("last version check failed: {error}"));
        }
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

/// Refuse a host decision where a manager already pins this tool, because
/// one target runs one mechanism. Every route that records the decision
/// asks: this verb, the stage that carries it, and the landing that
/// writes it.
///
/// SATISFIES acquisition:a-host-install-is-a-recorded-decision
///
/// # Errors
///
/// [`AppError::Refused`] naming the manager that pins this tool.
pub(crate) fn refuse_host_where_pinned(target: &camino::Utf8Path) -> Result<(), AppError> {
    let detected = manager::detect(target);
    if let Some(pinned) = manager::wired(&detected).first() {
        return Err(AppError::Refused(format!(
            "{} already pins this tool; one target runs one mechanism",
            pinned.manager
        )));
    }
    Ok(())
}

/// Record that every host installs this tool itself.
///
/// The decision lives in the instance's declaration, comments kept, at
/// this binary's release as the minimum. Before a first landing there is
/// no declaration to write, so the decision rides the landing instead:
/// `sdd stage --acquisition host` records it in the candidate, and
/// `sdd init --stage <root>` lands it.
///
/// SATISFIES acquisition:a-host-install-is-a-recorded-decision
///
/// # Errors
///
/// [`AppError::Refused`] where a manager already pins this tool, where an
/// apply meets no instance, or where the declaration does not parse.
fn run_add_host(target: &camino::Utf8Path, apply: bool, json: bool) -> Result<(), AppError> {
    refuse_host_where_pinned(target)?;
    let decision = crate::domain::instance_config::Acquisition::host_at_this_release();
    let path = target.join(crate::domain::instance_config::CONFIG_PATH);
    let landed = target
        .join(crate::domain::manifest::MANIFEST_PATH)
        .is_file()
        && path.is_file();
    if !landed {
        if apply {
            return Err(AppError::Refused(format!(
                "{target} carries no instance declaration yet, so nothing was written; the decision rides the landing: sdd stage --acquisition host, then sdd init --stage <root>"
            )));
        }
        return report_host(json, &decision, None, false);
    }
    let text = std::fs::read_to_string(&path)?;
    crate::domain::instance_config::InstanceConfig::parse(&text).map_err(|error| {
        AppError::Refused(format!("{error}; repair it before recording a decision"))
    })?;
    let next = crate::domain::instance_config::with_acquisition(&text, &decision);
    let changed = next != text;
    if apply && changed {
        crate::adapters::fs::write_atomic(&path, next.as_bytes())?;
    }
    report_host(json, &decision, Some(&path), apply && changed)
}

/// What a host decision wrote, or would write.
fn report_host(
    json: bool,
    decision: &crate::domain::instance_config::Acquisition,
    declaration: Option<&camino::Utf8Path>,
    wrote: bool,
) -> Result<(), AppError> {
    if json {
        return output::json(&serde_json::json!({
            "schema": "sdd.self-depend-host/1",
            "acquisition": decision,
            "declaration": declaration,
            "written": wrote,
        }));
    }
    match declaration {
        None => output::line(
            "no instance declaration yet; the decision rides the landing: sdd stage --acquisition host, then sdd init --stage <root>",
        ),
        Some(path) if wrote => output::line(format!("recorded in {path}:")),
        Some(path) => output::line(format!("{path} would carry:")),
    }
    for line in decision.render().lines() {
        output::line(format!("  {line}"));
    }
    Ok(())
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
    if args.host {
        return run_add_host(&target, args.apply, args.json);
    }
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

// --- check ------------------------------------------------------------------

/// Write one warning to stderr.
fn warn(message: &str) {
    eprintln!("{message}");
}

/// Warn when a newer release is out, from the cache alone unless a refresh
/// is due.
///
/// Every outcome exits 0, so the shell loads. The one file written is the
/// cache, under its lock.
///
/// SATISFIES acquisition:the-check-warns-and-writes-only-its-cache
fn run_check(ctx: &AppContext, args: CheckArgs) {
    // SATISFIES acquisition:the-off-switch-stops-the-check-alone
    if status::switched_off() {
        return;
    }
    let Ok(target) = resolve_target(ctx, args.target) else {
        warn("sdd self-depend check: the target must be absolute or .");
        return;
    };
    let detected = manager::detect(&target);
    let [held] = manager::wired(&detected)[..] else {
        return;
    };
    let Some(pin) = held.pin.as_ref() else {
        return;
    };
    let Some(root) = UserEnv::from_process().cache_root() else {
        return;
    };
    let path = cache::path(&root.path, &target);
    let mut entry = cache::read(&path);
    let now = jiff::Timestamp::now();
    if cache::due(entry.as_ref(), now) {
        match refresh(&path, now) {
            Ok(Some(fresh)) => entry = Some(fresh),
            Ok(None) => {}
            Err(reason) => {
                warn(&format!(
                    "sdd cannot record its version check at {path}: {reason}."
                ));
                return;
            }
        }
    }
    let Some(entry) = entry else {
        return;
    };
    if let (Some(error), Some(next)) = (entry.error.as_ref(), entry.next_try()) {
        warn(&format!(
            "sdd could not check for a newer release: {error}. Next try after {next}."
        ));
    }
    if let Some(latest) = entry.latest_version()
        && latest > pin.version
    {
        warn(&format!(
            "sdd {latest} is out, this project pins {}. To move: {SYNC_COMMAND}",
            pin.version
        ));
    }
}

/// Refresh the cache under its lock: record the attempt, read the latest
/// release, record the result.
///
/// `Ok(None)` means another check holds the lock, so this one reads the
/// cache alone. `Err` carries why the cache cannot be written, and then
/// nothing reads the network.
fn refresh(path: &camino::Utf8Path, now: jiff::Timestamp) -> Result<Option<Entry>, String> {
    let Some(_lock) =
        cache::try_lock(&cache::lock_path(path)).map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    // Another check may have refreshed between the read and the lock.
    let mut entry = cache::read(path).unwrap_or_default();
    if !cache::due(Some(&entry), now) {
        return Ok(Some(entry));
    }
    entry.last_attempt = Some(now.to_string());
    cache::write(path, &entry).map_err(|error| error.to_string())?;
    match Index::new().latest_version() {
        Ok(version) => {
            entry.last_success = Some(now.to_string());
            entry.latest = Some(version.to_string());
            entry.error = None;
        }
        Err(error) => entry.error = Some(error.to_string()),
    }
    cache::write(path, &entry).map_err(|error| error.to_string())?;
    Ok(Some(entry))
}

// --- sync -------------------------------------------------------------------

/// What one sync run concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Outcome {
    /// The sync ran inside an `.envrc` evaluation, so nothing moved.
    InsideEnvrc,
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
    fn new(target: &Utf8PathBuf, outcome: Outcome, message: String) -> Self {
        Self {
            schema: "sdd.self-depend-sync/2",
            target: target.clone(),
            caller: "operator",
            outcome,
            manager: None,
            from: None,
            to: None,
            moved: None,
            message,
        }
    }
}

/// Move the pin through the wired manager, as the operator's act.
///
/// Where `DIRENV_IN_ENVRC` is set, inside an `.envrc` evaluation or under
/// `direnv exec`, it moves nothing, warns, and exits 1, so a retired
/// shell-entry line cannot resume automatic moves. That line still loads
/// the shell, because it ends in `|| true`.
///
/// # Errors
///
/// [`AppError::Violations`] where `DIRENV_IN_ENVRC` is set;
/// [`AppError::Usage`] for `--caller envrc`, a relative target, a tag that
/// is not a release, no wired manager, or several managers with none named;
/// [`AppError::Other`] where the latest release cannot be read;
/// [`AppError::Refused`] where the transaction failed and both files were
/// put back.
fn run_sync(ctx: &AppContext, args: SyncArgs) -> Result<(), AppError> {
    // SATISFIES acquisition:sync-is-the-operators-act
    if args.caller == Caller::Envrc {
        return Err(AppError::Usage(format!(
            "--caller envrc is retired: the shell-entry line is `{CHECK_LINE}`, and sync moves the pin only when a person runs it"
        )));
    }
    let json = args.json;
    let report = sync_inner(ctx, args)?;
    if report.outcome == Outcome::InsideEnvrc {
        warn(&report.message);
        if json {
            output::json(&report)?;
        }
        // The exit-1 path carries no envelope: the warning is on stderr
        // already, and a red exit is what a person running it must see.
        return Err(AppError::Violations { count: 1 });
    }
    if json {
        output::json(&report)
    } else {
        output::line(report.message);
        Ok(())
    }
}

/// The one wired manager a sync moves, or why none can be chosen.
fn choose_wired(asked: Option<Manager>, detected: &[Detected]) -> Result<&Detected, AppError> {
    let wired = manager::wired(detected);
    match (asked, wired.as_slice()) {
        (Some(asked), held) => held
            .iter()
            .find(|held| held.manager == asked)
            .copied()
            .ok_or_else(|| {
                AppError::Usage(format!("{asked} does not pin this tool in this target"))
            }),
        (None, [one]) => Ok(one),
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
    let report = |outcome: Outcome, message: String| SyncReport::new(&target, outcome, message);
    // SATISFIES acquisition:sync-is-the-operators-act
    if variable(DIRENV_IN_ENVRC_VAR).is_some() {
        return Ok(report(
            Outcome::InsideEnvrc,
            format!(
                "sdd self-depend sync moved nothing: {DIRENV_IN_ENVRC_VAR} is set, because this is an {ENVRC} evaluation or `direnv exec`. Run the sync from the loaded shell, or with `nix develop -c {SYNC_COMMAND}`. The shell-entry line is `{CHECK_LINE}`, which sdd self-depend clean --apply swaps in."
            ),
        ));
    }
    let detected = manager::detect(&target);
    let held = choose_wired(args.manager, &detected)?;
    let pin = held
        .pin
        .as_ref()
        .ok_or_else(|| AppError::Refused(format!("{} names no pin", held.manager)))?;

    let to = match args.tag.as_deref() {
        Some(tag) => parse_tag(tag).map_err(AppError::Usage)?,
        None => latest()?,
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
fn latest() -> Result<Version, AppError> {
    Index::new()
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
    /// The line swaps the clean makes, or would make without `--apply`.
    swaps: Vec<Swap>,
    /// The retired daily stamps under the state root it removes, or would.
    stamps: Vec<Utf8PathBuf>,
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
    let swaps: Vec<Swap> = leftovers::envrc_swaps(&target);
    let stamps = UserEnv::from_process()
        .state_root()
        .map(|root| cache::retired_stamps(&root.path))
        .unwrap_or_default();
    let mut removed = Vec::new();
    if args.apply {
        // SATISFIES acquisition:clean-removes-only-a-named-leftover
        for leftover in &found {
            std::fs::remove_file(target.join(&leftover.file))?;
            removed.push(leftover.file.clone());
        }
        // SATISFIES acquisition:clean-swaps-the-retired-line
        let envrc = target.join(ENVRC);
        if !swaps.is_empty()
            && let Some((next, _)) = leftovers::swapped(&std::fs::read_to_string(&envrc)?)
        {
            crate::adapters::fs::write_atomic(&envrc, next.as_bytes())?;
        }
        for stamp in &stamps {
            std::fs::remove_file(stamp)?;
        }
    }
    let kept = leftovers::kept(&target, &wired);
    let report = CleanReport {
        schema: "sdd.self-depend-clean/2",
        target,
        leftovers: found,
        kept,
        removed,
        swaps,
        stamps,
        applied: args.apply,
    };
    if args.json {
        return output::json(&report);
    }
    if report.leftovers.is_empty() && report.swaps.is_empty() && report.stamps.is_empty() {
        output::line("no leftover: this target runs one mechanism");
    }
    for swap in &report.swaps {
        let line = match (swap.action, swap.to.as_ref()) {
            (leftovers::Action::Swap, Some(to)) => format!(
                "{} {}: `{}` -> `{to}`",
                if args.apply { "swapped" } else { "would swap" },
                swap.file,
                swap.from
            ),
            _ => format!(
                "{} {}: `{}`, because the check line is already there",
                if args.apply {
                    "removed from"
                } else {
                    "would remove from"
                },
                swap.file,
                swap.from
            ),
        };
        output::line(line);
    }
    for stamp in &report.stamps {
        output::line(format!(
            "{} {stamp}: a retired daily stamp",
            if args.apply {
                "removed"
            } else {
                "would remove"
            }
        ));
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
