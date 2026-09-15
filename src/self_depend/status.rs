//! The offline status report: what a target carries, and nothing judged.
//!
//! A report is not a verdict. Every state it can describe exits 0, and the
//! `next` lines say what an operator does about each.

use camino::{Utf8Path, Utf8PathBuf};
use semver::Version;
use serde::Serialize;

use crate::domain::paths::{CI_VAR, SELF_DEPEND_OFF_VAR, UserEnv, variable};
use crate::self_depend::leftovers::{self, Leftover};
use crate::self_depend::manager::{self, Detected, Manager};
use crate::self_depend::pin;
use crate::self_depend::stamp;
use crate::self_depend::venue::Venue;
use crate::self_depend::{ENVRC, SYNC_LINE};

/// The machine schema `sdd self-depend status --json` declares.
pub const SCHEMA: &str = "sdd.self-depend-status/1";

/// Whether a file is on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Presence {
    /// The file exists.
    Present,
    /// The file does not exist.
    Absent,
}

impl Presence {
    const fn of(present: bool) -> Self {
        if present { Self::Present } else { Self::Absent }
    }
}

/// How a recorded pin compares with the release this binary is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Freshness {
    /// The pin names this binary's release.
    Current,
    /// The pin names an older release.
    Behind,
    /// The pin names a newer release than this binary.
    Ahead,
}

/// One manager's row.
#[derive(Debug, Clone, Serialize)]
pub struct ManagerEntry {
    /// Which manager.
    pub manager: Manager,
    /// Whether the target carries its file.
    pub present: Presence,
    /// The file, where present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<Utf8PathBuf>,
    /// Whether the file names this tool.
    pub pinned: bool,
    /// The version pinned, as the file spells it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The venue the entry's form selects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub venue: Option<Venue>,
    /// How many lines pin this tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin_lines: Option<usize>,
    /// The pin against this binary's release.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness: Option<Freshness>,
    /// Whether the manager's lock is on disk, where the manager keeps one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<Presence>,
    /// The revision the lock holds for this tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_rev: Option<String>,
}

/// What the report concludes about the wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// One manager pins this tool, the sync line is landed, nothing is left over.
    Ready,
    /// No manager file names this tool.
    Unwired,
    /// A manager pins this tool and the shell loader carries no sync line.
    LineAbsent,
    /// More than one manager names this tool.
    Ambiguous,
    /// A predecessor mechanism left a file behind.
    Leftovers,
}

/// What the host offers.
#[derive(Debug, Clone, Serialize)]
pub struct Host {
    /// Whether `nix` is on `PATH`.
    pub nix: bool,
    /// Whether `direnv` is on `PATH`.
    pub direnv: bool,
}

/// The whole report.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The machine schema of this object.
    pub schema: &'static str,
    /// The target read.
    pub target: Utf8PathBuf,
    /// What the wiring amounts to.
    pub state: State,
    /// The one manager that pins this tool, where exactly one does.
    pub wired: Option<Manager>,
    /// Every manager, in the enum's order, absent ones included.
    pub managers: Vec<ManagerEntry>,
    /// Whether the shell loader file exists.
    pub envrc: Presence,
    /// Whether the shell loader carries the sync line.
    pub envrc_sync: bool,
    /// The day of the last attempt for this checkout, where one is stamped.
    pub stamp: Option<String>,
    /// Whether the loop is switched off in this environment.
    pub off: bool,
    /// What the host offers.
    pub host: Host,
    /// What a predecessor mechanism left behind.
    pub leftovers: Vec<Leftover>,
    /// What an operator does next, one line each.
    pub next: Vec<String>,
}

/// Whether the loop is switched off: in continuous integration, or by the
/// operator's variable.
#[must_use]
pub fn switched_off() -> bool {
    variable(CI_VAR).is_some() || variable(SELF_DEPEND_OFF_VAR).is_some()
}

/// Whether one program is on `PATH`.
fn on_path(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

/// The report for one target, offline.
///
/// SATISFIES acquisition:status-reports-and-never-judges
#[must_use]
pub fn report(target: &Utf8Path, env: &UserEnv, this: &Version) -> Report {
    let detected = manager::detect(target);
    let managers = detected
        .iter()
        .map(|held| entry(target, held, this))
        .collect();
    let wired = manager::wired(&detected);
    let envrc_text = std::fs::read_to_string(target.join(ENVRC)).ok();
    let envrc_sync = envrc_text
        .as_deref()
        .is_some_and(|text| text.contains(SYNC_LINE));
    let leftovers = leftovers::find(target, &[]);
    let stamp = env
        .state_root()
        .map(|root| stamp::path(&root.path, target))
        .and_then(|path| stamp::read(&path))
        .map(|day| day.to_string());
    let state = match (wired.len(), envrc_sync, leftovers.is_empty()) {
        (0, _, _) => State::Unwired,
        (1, true, true) => State::Ready,
        (1, true, false) => State::Leftovers,
        (1, false, _) => State::LineAbsent,
        (_, _, _) => State::Ambiguous,
    };
    let next = next_lines(state, &wired, target);
    Report {
        schema: SCHEMA,
        target: target.to_owned(),
        state,
        wired: (wired.len() == 1).then(|| wired[0].manager),
        managers,
        envrc: Presence::of(envrc_text.is_some()),
        envrc_sync,
        stamp,
        off: switched_off(),
        host: Host {
            nix: on_path("nix"),
            direnv: on_path("direnv"),
        },
        leftovers,
        next,
    }
}

fn entry(target: &Utf8Path, held: &Detected, this: &Version) -> ManagerEntry {
    let lock = held.manager.lock_file().map(|lock| {
        let path = target.join(lock);
        (
            Presence::of(path.is_file()),
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| pin::locked_rev(&text)),
        )
    });
    ManagerEntry {
        manager: held.manager,
        present: Presence::of(held.file.is_some()),
        file: held.file.clone(),
        pinned: held.pin.is_some(),
        version: held.pin.as_ref().map(|pin| pin.spelled.clone()),
        venue: held.pin.as_ref().and_then(|pin| pin.venue),
        pin_lines: held.pin.as_ref().map(|pin| pin.lines),
        freshness: held.pin.as_ref().map(|pin| match pin.version.cmp(this) {
            std::cmp::Ordering::Less => Freshness::Behind,
            std::cmp::Ordering::Equal => Freshness::Current,
            std::cmp::Ordering::Greater => Freshness::Ahead,
        }),
        lock: lock.as_ref().map(|(presence, _)| *presence),
        locked_rev: lock.and_then(|(_, rev)| rev),
    }
}

fn next_lines(state: State, wired: &[&Detected], target: &Utf8Path) -> Vec<String> {
    match state {
        State::Ready => vec![format!(
            "sdd self-depend sync --caller operator --target {target} moves the pin by hand"
        )],
        State::Unwired => vec![format!(
            "sdd self-depend add --target {target} serves the fragments for the manager this project runs"
        )],
        State::LineAbsent => vec![format!(
            "add `{SYNC_LINE}` to {ENVRC}; sdd self-depend add --target {target} prints it with its placement"
        )],
        State::Ambiguous => {
            let names: Vec<&str> = wired.iter().map(|held| held.manager.as_str()).collect();
            vec![format!(
                "one target runs one mechanism; keep one of {} and remove the others' pins",
                names.join(", ")
            )]
        }
        State::Leftovers => vec![format!(
            "sdd self-depend clean --target {target} --apply removes what the predecessor left"
        )],
    }
}
