//! Install the embedded skills into agent skill directories, as packages.
//!
//! The destinations live outside any instance — under the invoking user's
//! home — so nothing here touches an instance manifest. The unit is the
//! package: one directory per skill, holding `SKILL.md` and every shared
//! artifact under `references/`, which is what the Agent Skills format and
//! every documented host resolve against.
//!
//! Three references decide every destination: the payload, the receipt, and
//! the disk. Bytes matching the payload are current. Bytes the receipt
//! vouches for are this tool's and may be replaced or taken back. Every
//! other byte is the user's: an install refuses it without `--force`, and
//! an uninstall leaves it and names it.
//!
//! Every apply runs as one transaction. It holds the user-scope lock for
//! its whole run, recovers an unfinished run before it plans new work,
//! stages each write beside its destination, journals before the first
//! replacement, and writes the receipt last. A run the process does not
//! finish is rolled back by the next invocation.

use std::collections::BTreeSet;

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::ownership::Sha256;
use crate::domain::paths::HOME_VAR;
use crate::domain::skill_record::SkillRecord;
use crate::error::AppError;
use crate::transaction::journal::{self, Entry, Journal};
use crate::transaction::lock::Lock;
use crate::transaction::stage::Stage;

// The roots are declared in `domain::paths` and reach their callers from
// here, so that what a destination is stays one statement and what an
// install does with it stays another.
pub use crate::domain::paths::{AGENTS_ROOT, CLAUDE_ROOT, LEGACY_SHARED_ROOT};

/// The invoking user's home directory.
///
/// # Errors
///
/// [`AppError::Usage`] when `HOME` is unset or empty.
pub fn home() -> Result<Utf8PathBuf, AppError> {
    crate::domain::paths::UserEnv::from_process()
        .home
        .ok_or_else(|| AppError::Usage(format!("{HOME_VAR} is not set")))
}

/// Where one run writes: the agent roots, the state root, and the receipt.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The agent skill roots this run was asked to touch, deduplicated.
    pub roots: Vec<Utf8PathBuf>,
    /// Where this tool keeps state that outlives a command.
    pub state_root: Utf8PathBuf,
    /// The receipt vouching for every user-scope file this tool wrote.
    pub receipt: Utf8PathBuf,
    /// The receipt at the home-relative path, read once as a fallback.
    pub legacy_receipt: Utf8PathBuf,
    /// The retired shared root, swept rather than written.
    pub legacy_shared: Utf8PathBuf,
}

impl Layout {
    /// The lock every apply holds for its whole run.
    #[must_use]
    pub fn lock_path(&self) -> Utf8PathBuf {
        self.state_root.join(crate::domain::paths::SKILL_LOCK_FILE)
    }

    /// The journal an unfinished run leaves.
    #[must_use]
    pub fn journal_path(&self) -> Utf8PathBuf {
        self.state_root
            .join(crate::domain::paths::SKILL_JOURNAL_FILE)
    }

    /// Where a run copies what it is about to replace.
    #[must_use]
    pub fn backups(&self) -> Utf8PathBuf {
        self.state_root.join(crate::domain::paths::BACKUPS_DIR)
    }

    /// Every location a sweep reads: the selected roots and the retired one.
    fn scanned(&self) -> Vec<Utf8PathBuf> {
        let mut scanned = self.roots.clone();
        scanned.push(self.legacy_shared.clone());
        scanned
    }
}

/// One planned file: where, which bytes, and their digest.
#[derive(Debug, Clone)]
struct Planned {
    destination: Utf8PathBuf,
    bytes: &'static [u8],
    digest: Sha256,
}

/// A destination this tool refuses to write through, whatever `--force` says.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Blocked {
    path: Utf8PathBuf,
    kind: &'static str,
}

impl std::fmt::Display for Blocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is {}", self.path, self.kind)
    }
}

/// How a run ends when a test interrupts it.
///
/// `Abandoned` stands in for the process dying: the caller does not roll
/// back, so the journal survives and the next invocation is what recovers.
enum Failure {
    Error(AppError),
    Abandoned,
}

impl From<AppError> for Failure {
    fn from(error: AppError) -> Self {
        Self::Error(error)
    }
}

impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        Self::Error(AppError::Io(error))
    }
}

/// Where a run stops, counted in boundaries of the persistence order.
///
/// Production never sets it. The recovery tests walk it from one upward and
/// assert that the next invocation puts every destination back before it
/// writes anything new.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Interrupt {
    /// Stop after this many boundaries, without rolling back.
    after: Option<usize>,
}

fn reached(passed: &mut usize, interrupt: Interrupt) -> Result<(), Failure> {
    *passed += 1;
    if interrupt.after == Some(*passed) {
        return Err(Failure::Abandoned);
    }
    Ok(())
}

/// Every file of every package under `roots`, root by root and skill by skill.
fn plan(roots: &[Utf8PathBuf]) -> Result<Vec<Planned>, AppError> {
    let mut planned = Vec::new();
    for root in roots {
        for name in crate::embedded::skill_names() {
            let package = crate::embedded::skill_package(name)
                .ok_or_else(|| anyhow::anyhow!("payload skill missing: {name}"))?;
            for (relative, bytes) in package {
                planned.push(Planned {
                    destination: root.join(name).join(relative),
                    bytes,
                    digest: Sha256::of(bytes),
                });
            }
        }
    }
    Ok(planned)
}

/// The first component of `destination` under `root` this tool will not
/// write through.
///
/// Every component is inspected without following links, from the agent
/// root down. Directories above the root are the user's layout and stay
/// unjudged: this tool did not create them and does not police them.
fn blocked_by(root: &Utf8Path, destination: &Utf8Path) -> Option<Blocked> {
    let relative = destination.strip_prefix(root).ok()?;
    let mut current = root.to_owned();
    if let Ok(meta) = std::fs::symlink_metadata(&current) {
        if meta.file_type().is_symlink() {
            return Some(Blocked {
                path: current,
                kind: "a symlink",
            });
        }
        if !meta.is_dir() {
            return Some(Blocked {
                path: current,
                kind: "a file where a directory is needed",
            });
        }
    }
    let components: Vec<&str> = relative.as_str().split('/').collect();
    let last = components.len().saturating_sub(1);
    for (index, part) in components.iter().enumerate() {
        current = current.join(part);
        let Ok(meta) = std::fs::symlink_metadata(&current) else {
            // Absent, so nothing below it exists either.
            return None;
        };
        if meta.file_type().is_symlink() {
            return Some(Blocked {
                path: current,
                kind: "a symlink",
            });
        }
        if index == last {
            if !meta.is_file() {
                return Some(Blocked {
                    path: current,
                    kind: if meta.is_dir() {
                        "a directory"
                    } else {
                        "not a regular file"
                    },
                });
            }
        } else if !meta.is_dir() {
            return Some(Blocked {
                path: current,
                kind: "a file where a directory is needed",
            });
        }
    }
    None
}

/// Which root a destination sits under, for the chain check.
fn owning_root<'a>(layout: &'a Layout, destination: &Utf8Path) -> Option<&'a Utf8Path> {
    layout
        .roots
        .iter()
        .chain(std::iter::once(&layout.state_root))
        .map(Utf8PathBuf::as_path)
        .find(|root| destination.starts_with(root))
}

/// Every destination this run will not write through.
fn blocked(layout: &Layout, destinations: &[Utf8PathBuf]) -> Vec<Blocked> {
    let mut found = Vec::new();
    for destination in destinations {
        let Some(root) = owning_root(layout, destination) else {
            continue;
        };
        if let Some(one) = blocked_by(root, destination)
            && !found.contains(&one)
        {
            found.push(one);
        }
    }
    found
}

/// What a destination currently holds, judged against the two references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// Nothing is there.
    Absent,
    /// The payload's own bytes.
    Current,
    /// Bytes the receipt vouches for, from an earlier release.
    Recorded,
    /// Bytes neither reference accounts for.
    Foreign,
}

fn standing(
    destination: &Utf8Path,
    intended: &Sha256,
    record: &SkillRecord,
) -> Result<Standing, AppError> {
    if !destination.is_file() {
        return Ok(Standing::Absent);
    }
    // An unreadable destination raises instead of passing as clean: a
    // comparison that cannot run must never license an overwrite.
    let found = Sha256::of(&std::fs::read(destination)?);
    if &found == intended {
        return Ok(Standing::Current);
    }
    if record.wrote(destination, &found) {
        return Ok(Standing::Recorded);
    }
    Ok(Standing::Foreign)
}

/// Recorded destinations under a scanned root that this run no longer plans.
///
/// Only bytes the receipt still vouches for are swept: a leftover the user
/// edited is theirs. This is also what sweeps the retired shared root, whose
/// two files an earlier release wrote and recorded.
fn leftovers(
    scanned: &[Utf8PathBuf],
    record: &SkillRecord,
    keep: &BTreeSet<Utf8PathBuf>,
) -> Vec<(Utf8PathBuf, Sha256, bool)> {
    record
        .written
        .iter()
        .filter(|(destination, _)| {
            !keep.contains(*destination) && scanned.iter().any(|root| destination.starts_with(root))
        })
        .map(|(destination, digest)| {
            let ours = !destination.is_symlink()
                && destination.is_file()
                && std::fs::read(destination).is_ok_and(|found| Sha256::of(&found) == *digest);
            (destination.clone(), digest.clone(), ours)
        })
        .collect()
}

/// Remove the directories a removal emptied, deepest first, up to `stop`.
fn prune_empty(directories: &BTreeSet<Utf8PathBuf>, stop: &[Utf8PathBuf], lines: &mut Vec<String>) {
    let mut deepest: Vec<&Utf8PathBuf> = directories.iter().collect();
    deepest.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for directory in deepest {
        if stop.iter().any(|root| root == directory) {
            continue;
        }
        let Ok(mut entries) = std::fs::read_dir(directory) else {
            continue;
        };
        if entries.next().is_none() {
            let _ = std::fs::remove_dir(directory);
        } else {
            lines.push(format!("kept (not empty): {directory}"));
        }
    }
}

/// The receipt this run intends to leave.
fn next_receipt(record: &SkillRecord, written: &[Planned], removed: &[Utf8PathBuf]) -> SkillRecord {
    let mut next = record.clone();
    next.schema_version = crate::domain::skill_record::SCHEMA_VERSION;
    next.engine_version = env!("CARGO_PKG_VERSION").to_string();
    for destination in removed {
        next.written.remove(destination);
    }
    for entry in written {
        next.written
            .insert(entry.destination.clone(), entry.digest.clone());
    }
    // A run that changed nothing leaves the receipt alone, timestamp
    // included. A record rewritten on every invocation would make a second
    // install write a file, and idempotence is what tells an operator that
    // the home is already current.
    if next.written == record.written
        && next.engine_version == record.engine_version
        && record.schema_version == crate::domain::skill_record::SCHEMA_VERSION
    {
        return record.clone();
    }
    next.installed_at = jiff::Timestamp::now().to_string();
    next
}

/// Read the receipt, falling back to the home-relative path once.
fn load_receipt(layout: &Layout) -> SkillRecord {
    SkillRecord::load_with_fallback(&layout.receipt, &layout.legacy_receipt)
}

/// Install every embedded skill package under each root, previewing by default.
///
/// # Errors
///
/// [`AppError::Refused`] when a destination is reached through a link or
/// holds bytes neither reference accounts for without `--force`,
/// [`AppError::Busy`] when another process holds the lock,
/// [`AppError::Unrecovered`] when an unfinished run cannot be put back, and
/// [`AppError::Receipt`] when the apply cannot record what it wrote.
pub fn install(layout: &Layout, apply: bool, force: bool) -> Result<Vec<String>, AppError> {
    settle(install_with(layout, apply, force, Interrupt::default()))
}

/// Remove every installed skill package under each root, previewing by default.
///
/// # Errors
///
/// As [`install`], except that nothing here refuses on foreign bytes: a
/// file the receipt cannot vouch for stays and is named.
pub fn uninstall(layout: &Layout, apply: bool) -> Result<Vec<String>, AppError> {
    settle(uninstall_with(layout, apply, Interrupt::default()))
}

fn settle(result: Result<Vec<String>, Failure>) -> Result<Vec<String>, AppError> {
    result.map_err(|failure| match failure {
        Failure::Error(error) => error,
        Failure::Abandoned => AppError::Other(anyhow::anyhow!(
            "the run was interrupted; the next invocation recovers it"
        )),
    })
}

/// The lock an applied run holds for the whole of its own reasoning.
///
/// A preview takes none: it writes nothing, and a preview that refused
/// because somebody else was installing would fail for a reason the
/// operator cannot act on. An applied run takes it before it reads a
/// receipt or scans a destination, so every decision it makes is one the
/// world still agrees with when it executes. A run that derived its
/// removals before the lock could remove files a newer install had just
/// written.
///
/// # Errors
///
/// [`AppError::Busy`] when another process holds it, and whatever
/// recovery refuses.
fn held(layout: &Layout, apply: bool, purpose: &str) -> Result<Option<Lock>, Failure> {
    if !apply {
        return Ok(None);
    }
    let lock = Lock::exclusive(&layout.lock_path(), purpose)?;
    journal::recover(&layout.journal_path())?;
    Ok(Some(lock))
}

/// The install, with the recovery tests' interruption point.
fn install_with(
    layout: &Layout,
    apply: bool,
    force: bool,
    interrupt: Interrupt,
) -> Result<Vec<String>, Failure> {
    let _lock = held(layout, apply, "skill install")?;
    let planned = plan(&layout.roots)?;
    let mut lines: Vec<String> = planned
        .iter()
        .map(|entry| entry.destination.to_string())
        .collect();

    let record = load_receipt(layout);
    let kept: BTreeSet<Utf8PathBuf> = planned
        .iter()
        .map(|entry| entry.destination.clone())
        .collect();
    let stale = leftovers(&layout.scanned(), &record, &kept);
    for (destination, _, ours) in &stale {
        if *ours {
            lines.push(format!("sweep (no longer in the payload): {destination}"));
        } else {
            lines.push(format!("kept (edited): {destination}"));
        }
    }

    let mut destinations: Vec<Utf8PathBuf> = planned
        .iter()
        .map(|entry| entry.destination.clone())
        .collect();
    destinations.push(layout.receipt.clone());
    let refused = blocked(layout, &destinations);
    for one in &refused {
        lines.push(format!("conflict: {one}"));
    }

    let mut foreign: Vec<Utf8PathBuf> = Vec::new();
    let mut writes: Vec<Planned> = Vec::new();
    for entry in &planned {
        if refused
            .iter()
            .any(|one| entry.destination.starts_with(&one.path))
        {
            continue;
        }
        match standing(&entry.destination, &entry.digest, &record)? {
            Standing::Current => {}
            Standing::Foreign => {
                foreign.push(entry.destination.clone());
                writes.push(entry.clone());
            }
            Standing::Absent | Standing::Recorded => writes.push(entry.clone()),
        }
    }
    for destination in &foreign {
        lines.push(format!(
            "conflict: {destination} holds bytes this tool did not write"
        ));
    }

    if !apply {
        lines.push("DRY RUN: no files written".to_string());
        return Ok(lines);
    }
    if !refused.is_empty() {
        return Err(refuse_blocked(&refused).into());
    }
    if !force && !foreign.is_empty() {
        let paths: Vec<&str> = foreign.iter().map(|path| path.as_str()).collect();
        return Err(AppError::Refused(format!(
            "destinations hold bytes this tool did not write: {}; re-run with --force to overwrite",
            paths.join(", ")
        ))
        .into());
    }

    let swept: Vec<Utf8PathBuf> = stale
        .iter()
        .filter(|(_, _, ours)| *ours)
        .map(|(destination, _, _)| destination.clone())
        .collect();
    let receipt = next_receipt(&record, &writes, &swept);
    run_transaction(
        layout,
        &writes,
        &stale
            .into_iter()
            .filter(|(_, _, ours)| *ours)
            .map(|(destination, digest, _)| (destination, digest))
            .collect::<Vec<_>>(),
        &receipt,
        &mut lines,
        interrupt,
    )?;
    Ok(lines)
}

/// The uninstall, with the recovery tests' interruption point.
fn uninstall_with(
    layout: &Layout,
    apply: bool,
    interrupt: Interrupt,
) -> Result<Vec<String>, Failure> {
    // A home with no receipt has nothing this tool wrote, so an uninstall
    // there removes nothing and takes no lock: creating the lock file
    // would itself be the write that home was promised it would not get.
    let _lock = held(layout, apply && layout.receipt.exists(), "skill uninstall")?;
    let planned = plan(&layout.roots)?;
    let record = load_receipt(layout);
    let mut lines: Vec<String> = Vec::new();
    let mut removals: Vec<(Utf8PathBuf, Sha256)> = Vec::new();

    let refused = blocked(
        layout,
        &planned
            .iter()
            .map(|entry| entry.destination.clone())
            .collect::<Vec<_>>(),
    );
    for one in &refused {
        lines.push(format!("conflict: {one}"));
    }

    for entry in &planned {
        if refused
            .iter()
            .any(|one| entry.destination.starts_with(&one.path))
        {
            continue;
        }
        if !entry.destination.is_file() {
            continue;
        }
        let found = Sha256::of(&std::fs::read(&entry.destination)?);
        if record.wrote(&entry.destination, &found) {
            lines.push(entry.destination.to_string());
            removals.push((entry.destination.clone(), found));
        } else if found == entry.digest {
            // The payload's own bytes with no receipt behind them: this
            // tool cannot tell its copy from one the user placed there.
            lines.push(format!("kept (not this tool's): {}", entry.destination));
        } else {
            lines.push(format!("kept (edited): {}", entry.destination));
        }
    }

    let keep: BTreeSet<Utf8PathBuf> = removals
        .iter()
        .map(|(destination, _)| destination.clone())
        .collect();
    for (destination, digest, ours) in leftovers(&layout.scanned(), &record, &keep) {
        if ours {
            lines.push(format!("sweep (no longer in the payload): {destination}"));
            removals.push((destination, digest));
        } else if destination.exists() {
            lines.push(format!("kept (edited): {destination}"));
        }
    }

    if !apply {
        lines.push("DRY RUN: no files removed".to_string());
        return Ok(lines);
    }
    if !refused.is_empty() {
        return Err(refuse_blocked(&refused).into());
    }
    // A home this tool never wrote into has nothing to take back, and an
    // uninstall there must leave it exactly as it found it — no state root,
    // no lock file, nothing.
    if removals.is_empty() && !layout.receipt.exists() {
        return Ok(lines);
    }

    let gone: Vec<Utf8PathBuf> = removals
        .iter()
        .map(|(destination, _)| destination.clone())
        .collect();
    let receipt = next_receipt(&record, &[], &gone);
    run_transaction(layout, &[], &removals, &receipt, &mut lines, interrupt)?;
    Ok(lines)
}

fn refuse_blocked(refused: &[Blocked]) -> AppError {
    let named: Vec<String> = refused.iter().map(Blocked::to_string).collect();
    AppError::Refused(format!(
        "destinations cannot be written through: {}; move them aside and run this again",
        named.join("; ")
    ))
}

/// Stage, journal, replace, remove, and record — or put everything back.
fn run_transaction(
    layout: &Layout,
    writes: &[Planned],
    removals: &[(Utf8PathBuf, Sha256)],
    receipt: &SkillRecord,
    lines: &mut Vec<String>,
    interrupt: Interrupt,
) -> Result<(), Failure> {
    // A receipt that vouches for nothing is removed rather than written:
    // an empty record is a file that says the tool wrote something here,
    // and it did not.
    let empty = receipt.written.is_empty();
    let receipt_bytes = receipt.to_json().into_bytes();
    let receipt_current = if empty {
        !layout.receipt.exists()
    } else {
        std::fs::read(&layout.receipt).is_ok_and(|held| held == receipt_bytes)
    };
    if writes.is_empty() && removals.is_empty() && receipt_current {
        return Ok(());
    }

    let stage = Stage::new(&layout.backups())?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut passed = 0usize;
    for entry in writes {
        let before = stage.back_up(&entry.destination)?;
        entries.push(Entry::write(
            entry.destination.clone(),
            before,
            entry.digest.clone(),
        ));
        reached(&mut passed, interrupt)?;
    }
    for (destination, _) in removals {
        let Some(before) = stage.back_up(destination)? else {
            continue;
        };
        entries.push(Entry::remove(destination.clone(), before));
        reached(&mut passed, interrupt)?;
    }
    let receipt_before = stage.back_up(&layout.receipt)?;
    if empty {
        if let Some(before) = receipt_before {
            entries.push(Entry::remove(layout.receipt.clone(), before));
        }
    } else {
        entries.push(Entry::write(
            layout.receipt.clone(),
            receipt_before,
            Sha256::of(&receipt_bytes),
        ));
    }
    reached(&mut passed, interrupt)?;

    let mut journal = Journal::begin(&layout.journal_path(), &layout.backups(), entries)?;
    reached(&mut passed, interrupt)?;

    let result = execute(
        layout,
        writes,
        removals,
        if empty {
            None
        } else {
            Some(receipt_bytes.as_slice())
        },
        &mut journal,
        &mut passed,
        interrupt,
    );
    match result {
        Ok(()) => {}
        Err(Failure::Abandoned) => return Err(Failure::Abandoned),
        Err(Failure::Error(cause)) => {
            let restored = journal.roll_back();
            return Err(Failure::Error(abort(&cause, restored.err().as_ref())));
        }
    }

    let mut directories: BTreeSet<Utf8PathBuf> = BTreeSet::new();
    for (destination, _) in removals {
        let mut parent = destination.parent();
        while let Some(directory) = parent {
            if !layout.roots.iter().any(|root| directory.starts_with(root))
                && !directory.starts_with(&layout.state_root)
            {
                break;
            }
            directories.insert(directory.to_owned());
            parent = directory.parent();
        }
    }
    let mut stop = layout.roots.clone();
    stop.push(layout.state_root.clone());
    prune_empty(&directories, &stop, lines);

    journal.finish()?;
    // The journal is gone, so nothing can ask for a copy of what this run
    // replaced. Keeping them would grow the state root by one file per
    // release for the life of the home.
    let _ = std::fs::remove_dir_all(layout.backups());
    Ok(())
}

fn execute(
    layout: &Layout,
    writes: &[Planned],
    removals: &[(Utf8PathBuf, Sha256)],
    receipt_bytes: Option<&[u8]>,
    journal: &mut Journal,
    passed: &mut usize,
    interrupt: Interrupt,
) -> Result<(), Failure> {
    for entry in writes {
        replace_one(layout, &entry.destination, entry.bytes)?;
        reached(passed, interrupt)?;
        journal.mark_done(&entry.destination)?;
        reached(passed, interrupt)?;
    }
    for (destination, _) in removals {
        match std::fs::remove_file(destination) {
            Ok(()) => crate::transaction::sync_parent(destination)?,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(Failure::Error(AppError::Io(source))),
        }
        reached(passed, interrupt)?;
        journal.mark_done(destination)?;
        reached(passed, interrupt)?;
    }
    // The receipt is last, and a receipt this run cannot write is a run
    // that rolls back: a landing this tool cannot vouch for is a landing it
    // would refuse to take back.
    match receipt_bytes {
        Some(bytes) => {
            replace_one(layout, &layout.receipt, bytes).map_err(|failure| match failure {
                Failure::Error(cause) => {
                    Failure::Error(AppError::Receipt(format!("{}: {cause}", layout.receipt)))
                }
                abandoned @ Failure::Abandoned => abandoned,
            })?;
        }
        None => match std::fs::remove_file(&layout.receipt) {
            Ok(()) => crate::transaction::sync_parent(&layout.receipt)?,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(Failure::Error(AppError::Receipt(format!(
                    "{}: {source}",
                    layout.receipt
                ))));
            }
        },
    }
    reached(passed, interrupt)?;
    journal.mark_done(&layout.receipt)?;
    reached(passed, interrupt)?;
    Ok(())
}

/// Stage one destination and rename it into place.
///
/// The chain is inspected again immediately before the rename, so a link
/// substituted between the plan and the write is refused rather than
/// followed.
fn replace_one(layout: &Layout, destination: &Utf8Path, bytes: &[u8]) -> Result<(), Failure> {
    let scratch = Stage::write(destination, bytes)?;
    if let Some(root) = owning_root(layout, destination)
        && let Some(one) = blocked_by(root, destination)
    {
        Stage::discard(&scratch);
        return Err(Failure::Error(refuse_blocked(std::slice::from_ref(&one))));
    }
    Stage::replace(&scratch, destination)?;
    Ok(())
}

/// The refusal a failed apply carries, naming the cause and what it restored.
fn abort(cause: &AppError, unrestored: Option<&AppError>) -> AppError {
    unrestored.map_or_else(
        || {
            AppError::Refused(format!(
                "skill install aborted; the destinations were restored: {cause}"
            ))
        },
        |failure| {
            AppError::Unrecovered(format!(
                "skill install aborted and restoration is incomplete; verify by hand: {failure}: {cause}"
            ))
        },
    )
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use std::collections::BTreeMap;

    use super::*;

    fn root(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap())
    }

    /// A digest of every file a run is answerable for, under one scratch
    /// home.
    ///
    /// The lock, the journal, the holder note, and the backup store are the
    /// transaction's own workings rather than destinations, so a comparison
    /// that counted them would report a run as having changed the home when
    /// it changed only its own scaffolding.
    fn tree(dir: &tempfile::TempDir) -> BTreeMap<String, Sha256> {
        walkdir::WalkDir::new(dir.path())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| {
                let path = entry.path().to_str()?.to_string();
                let bytes = std::fs::read(entry.path()).ok()?;
                Some((path, Sha256::of(&bytes)))
            })
            .filter(|(path, _)| {
                !path.contains("/backups/")
                    && !["journal", "lock", "holder"].iter().any(|suffix| {
                        std::path::Path::new(path)
                            .extension()
                            .is_some_and(|found| found == *suffix)
                    })
            })
            .collect()
    }

    /// The layout a home directory implies, selecting both agent roots.
    fn home(dir: &tempfile::TempDir) -> Layout {
        let home = root(dir);
        let state = home.join(crate::domain::paths::STATE_ROOT);
        Layout {
            roots: vec![home.join(AGENTS_ROOT), home.join(CLAUDE_ROOT)],
            receipt: state.join(crate::domain::paths::SKILL_RECEIPT_FILE),
            legacy_receipt: home.join(crate::domain::paths::LEGACY_SKILL_RECEIPT_PATH),
            legacy_shared: home.join(LEGACY_SHARED_ROOT),
            state_root: state,
        }
    }

    /// The same layout narrowed to one selected root.
    fn select(layout: &Layout, index: usize) -> Layout {
        Layout {
            roots: vec![layout.roots[index].clone()],
            ..layout.clone()
        }
    }

    fn package_files(root: &Utf8Path, name: &str) -> Vec<Utf8PathBuf> {
        crate::embedded::skill_package(name)
            .unwrap()
            .into_iter()
            .map(|(relative, _)| root.join(name).join(relative))
            .collect()
    }

    #[test]
    fn a_package_is_skill_md_plus_every_shared_artifact_under_references() {
        let package = crate::embedded::skill_package("sdd-setup").unwrap();
        let names: Vec<&str> = package.iter().map(|(path, _)| path.as_str()).collect();
        assert!(names.contains(&"SKILL.md"));
        assert!(names.contains(&"references/plan-gate.md"));
        assert!(names.contains(&"references/pre-flight-gate.md"));
        assert_eq!(package.len(), 1 + crate::embedded::shared_artifacts().len());
        assert!(crate::embedded::skill_package("no-such-skill").is_none());
    }

    #[test]
    fn an_install_lands_every_package_file_and_records_each_digest() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        let record = SkillRecord::load(&layout.receipt);
        assert_eq!(record.schema_version, 2);
        assert_eq!(record.engine_version, env!("CARGO_PKG_VERSION"));
        for root in &layout.roots {
            for name in crate::embedded::skill_names() {
                for path in package_files(root, name) {
                    assert!(path.is_file(), "{path} did not land");
                    let digest = Sha256::of(&std::fs::read(&path).unwrap());
                    assert!(record.wrote(&path, &digest), "{path} was not recorded");
                }
            }
        }
    }

    #[test]
    fn a_preview_lists_every_destination_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let lines = install(&layout, false, false).unwrap();
        assert_eq!(lines.last().unwrap(), "DRY RUN: no files written");
        let package = crate::embedded::skill_package("sdd-setup").unwrap().len();
        assert_eq!(
            lines.len(),
            crate::embedded::skill_names().len() * package * 2 + 1
        );
        assert!(!layout.roots[0].exists());
        assert!(!layout.receipt.exists());
    }

    #[test]
    fn a_second_install_is_idempotent_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        let before = std::fs::metadata(layout.roots[0].join("sdd-setup/SKILL.md"))
            .unwrap()
            .modified()
            .unwrap();
        let receipt_before = std::fs::read(&layout.receipt).unwrap();
        install(&layout, true, false).unwrap();
        assert_eq!(
            std::fs::metadata(layout.roots[0].join("sdd-setup/SKILL.md"))
                .unwrap()
                .modified()
                .unwrap(),
            before
        );
        assert_eq!(std::fs::read(&layout.receipt).unwrap(), receipt_before);
        assert!(!layout.journal_path().exists());
    }

    #[test]
    fn a_stale_package_file_the_receipt_vouches_for_is_replaced_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();

        let mut stale = SkillRecord::load(&layout.receipt);
        let destination = layout.roots[0].join("sdd-setup/references/plan-gate.md");
        std::fs::write(&destination, "older canon bytes\n").unwrap();
        stale
            .written
            .insert(destination.clone(), Sha256::of(b"older canon bytes\n"));
        crate::adapters::fs::write_file(&layout.receipt, stale.to_json().as_bytes()).unwrap();

        install(&layout, true, false).unwrap();
        assert!(
            std::fs::read_to_string(&destination)
                .unwrap()
                .contains("# The plan gate")
        );
    }

    #[test]
    fn an_edited_reference_refuses_the_install_naming_every_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        let edited = layout.roots[1].join("sdd-setup/references/plan-gate.md");
        std::fs::write(&edited, "mine\n").unwrap();
        let message = install(&layout, true, false).unwrap_err().to_string();
        assert!(message.contains(edited.as_str()), "{message}");
        assert_eq!(std::fs::read_to_string(&edited).unwrap(), "mine\n");
    }

    #[test]
    fn force_replaces_an_edited_file_and_records_the_new_digest() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        let edited = layout.roots[1].join("sdd-setup/SKILL.md");
        std::fs::write(&edited, "mine\n").unwrap();
        install(&layout, true, true).unwrap();
        let held = std::fs::read(&edited).unwrap();
        assert!(String::from_utf8_lossy(&held).contains("name: sdd-setup"));
        assert!(SkillRecord::load(&layout.receipt).wrote(&edited, &Sha256::of(&held)));
    }

    /// The defect this phase closes: an uninstall that consulted the payload
    /// and not the receipt deleted a skill the operator had edited.
    #[test]
    fn an_edited_skill_md_survives_uninstall_and_is_named_as_kept() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        let edited = layout.roots[0].join("sdd-setup/SKILL.md");
        std::fs::write(&edited, "mine\n").unwrap();

        let lines = uninstall(&layout, true).unwrap();
        assert_eq!(std::fs::read_to_string(&edited).unwrap(), "mine\n");
        assert!(
            lines
                .iter()
                .any(|line| line == &format!("kept (edited): {edited}")),
            "{lines:?}"
        );
    }

    #[test]
    fn an_edited_reference_survives_uninstall_and_its_directory_stays() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        let edited = layout.roots[0].join("sdd-setup/references/plan-gate.md");
        std::fs::write(&edited, "mine\n").unwrap();
        uninstall(&layout, true).unwrap();
        assert_eq!(std::fs::read_to_string(&edited).unwrap(), "mine\n");
        assert!(layout.roots[0].join("sdd-setup/references").is_dir());
    }

    #[test]
    fn an_uninstall_removes_a_directory_only_when_nothing_recorded_or_foreign_remains() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        let mine = layout.roots[1].join("sdd-setup/notes.md");
        std::fs::write(&mine, "mine").unwrap();

        let lines = uninstall(&layout, true).unwrap();
        assert!(!layout.roots[1].join("sdd-write-docs").exists());
        assert!(!layout.roots[1].join("sdd-setup/SKILL.md").exists());
        assert!(!layout.roots[1].join("sdd-setup/references").exists());
        assert_eq!(std::fs::read_to_string(&mine).unwrap(), "mine");
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("kept (not empty):"))
        );
        // A re-run on the emptied roots is a no-op, not an error.
        uninstall(&layout, true).unwrap();
    }

    #[test]
    fn the_retired_shared_root_is_swept_when_the_receipt_vouches_for_it() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        // Stand in for a 0.8.0 home: two gate files under the retired root,
        // each vouched for by the receipt that release wrote.
        let mut older = SkillRecord::new();
        for (name, bytes) in crate::embedded::shared_artifacts() {
            let destination = layout.legacy_shared.join(&name);
            crate::adapters::fs::write_file(&destination, bytes).unwrap();
            older.written.insert(destination, Sha256::of(bytes));
        }
        crate::adapters::fs::write_file(&layout.receipt, older.to_json().as_bytes()).unwrap();

        install(&layout, true, false).unwrap();
        assert!(
            !layout.legacy_shared.exists(),
            "the retired shared root survived"
        );
        assert!(
            layout.roots[0]
                .join("sdd-setup/references/plan-gate.md")
                .is_file()
        );
    }

    #[test]
    fn an_unrecorded_file_at_the_retired_shared_root_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let leftover = layout.legacy_shared.join("plan-gate.md");
        crate::adapters::fs::write_file(&leftover, b"mine\n").unwrap();
        install(&layout, true, false).unwrap();
        assert_eq!(std::fs::read(&leftover).unwrap(), b"mine\n");
    }

    #[test]
    fn a_symlinked_package_directory_skill_md_reference_or_parent_is_a_typed_conflict() {
        for linked in ["sdd-setup", "sdd-setup/SKILL.md", "sdd-setup/references"] {
            let dir = tempfile::tempdir().unwrap();
            let layout = home(&dir);
            let elsewhere = root(&dir).join("elsewhere");
            std::fs::create_dir_all(&elsewhere).unwrap();
            let target = layout.roots[0].join(linked);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(&elsewhere, target.as_std_path()).unwrap();

            for force in [false, true] {
                let message = install(&layout, true, force).unwrap_err().to_string();
                assert!(
                    message.contains("symlink"),
                    "{linked} with force {force}: {message}"
                );
                assert!(message.contains(target.as_str()), "{message}");
            }
            assert!(!elsewhere.join("SKILL.md").exists());
            assert!(target.is_symlink(), "the link itself was replaced");
        }
    }

    #[test]
    fn a_symlink_whose_target_matches_the_recorded_digest_is_still_a_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        let destination = layout.roots[0].join("sdd-setup/SKILL.md");
        let elsewhere = root(&dir).join("copy.md");
        std::fs::copy(&destination, &elsewhere).unwrap();
        std::fs::remove_file(&destination).unwrap();
        std::os::unix::fs::symlink(&elsewhere, destination.as_std_path()).unwrap();

        let message = install(&layout, true, true).unwrap_err().to_string();
        assert!(message.contains("symlink"), "{message}");
        let message = uninstall(&layout, true).unwrap_err().to_string();
        assert!(message.contains("symlink"), "{message}");
        assert!(destination.is_symlink());
    }

    #[test]
    fn two_roots_resolving_to_one_path_are_written_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut layout = home(&dir);
        layout.roots = vec![layout.roots[0].clone()];
        let lines = install(&layout, false, false).unwrap();
        let landed = lines
            .iter()
            .filter(|line| line.ends_with("sdd-setup/SKILL.md"))
            .count();
        assert_eq!(landed, 1);
    }

    #[test]
    fn a_second_installer_refuses_while_the_lock_is_held_naming_the_holder() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        // The home holds packages first, so the uninstall below has work to
        // do and reaches the lock rather than returning early.
        install(&layout, true, false).unwrap();
        age(&layout);
        let _held = Lock::exclusive(&layout.lock_path(), "skill install").unwrap();
        let error = install(&layout, true, false).unwrap_err();
        assert_eq!(error.kind(), "Busy");
        assert_eq!(error.exit_code(), 73);
        assert!(error.to_string().contains("skill install"));
        let error = uninstall(&layout, true).unwrap_err();
        assert_eq!(error.kind(), "Busy");
    }

    /// A run the process did not finish is put back before the next one
    /// writes anything, at every boundary of the persistence order.
    /// Stand in for a home an older release left: every package file holds
    /// its bytes, and the receipt vouches for each, so the next install has
    /// a replacement at every boundary.
    fn age(layout: &Layout) {
        let mut older = SkillRecord::new();
        for root in &layout.roots {
            for name in crate::embedded::skill_names() {
                for path in package_files(root, name) {
                    crate::adapters::fs::write_file(&path, b"older\n").unwrap();
                    older.written.insert(path, Sha256::of(b"older\n"));
                }
            }
        }
        crate::adapters::fs::write_file(&layout.receipt, older.to_json().as_bytes()).unwrap();
    }

    /// A run the process did not finish is put back before the next one
    /// writes anything, at every boundary of the persistence order.
    #[test]
    fn an_interrupted_install_is_rolled_back_by_the_next_invocation() {
        let mut boundaries = 0usize;
        for after in 1..200 {
            let dir = tempfile::tempdir().unwrap();
            // One root, because the matrix walks every boundary and the
            // second root doubles the walk without adding a kind of
            // boundary to it.
            let layout = select(&home(&dir), 0);
            age(&layout);
            let before = tree(&dir);

            let outcome = install_with(&layout, true, false, Interrupt { after: Some(after) });
            let Err(Failure::Abandoned) = outcome else {
                // The run finished before this boundary, so every earlier
                // boundary has already been walked.
                break;
            };
            boundaries = after;

            // The next invocation recovers before it plans new work, so the
            // home is exactly what it was before the interrupted run.
            // Before the journal exists there is nothing to recover, and
            // nothing has been replaced either, so both boundaries hold the
            // same promise: the home reads back exactly as it was.
            journal::recover(&layout.journal_path()).unwrap();
            assert!(!layout.journal_path().exists(), "after {after}");
            assert_eq!(tree(&dir), before, "after {after}");

            // Every few boundaries, take the recovered home all the way, so
            // the matrix proves a recovery leaves a home an install can
            // still land into and not only one that reads back the same.
            if after % 7 == 0 {
                install(&layout, true, false).unwrap();
                let record = SkillRecord::load(&layout.receipt);
                for path in package_files(&layout.roots[0], "sdd-setup") {
                    let digest = Sha256::of(&std::fs::read(&path).unwrap());
                    assert!(record.wrote(&path, &digest), "after {after}: {path}");
                }
            }
        }
        assert!(boundaries > 10, "only {boundaries} boundaries were walked");
    }

    #[test]
    fn recovery_is_idempotent_across_a_second_interruption() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        age(&layout);
        let before = tree(&dir);
        let Err(Failure::Abandoned) =
            install_with(&layout, true, false, Interrupt { after: Some(20) })
        else {
            panic!("the run was not interrupted");
        };
        journal::recover(&layout.journal_path()).unwrap();
        assert!(!journal::recover(&layout.journal_path()).unwrap());
        assert_eq!(tree(&dir), before);
    }

    #[test]
    fn a_receipt_write_failure_fails_the_apply_and_rolls_back() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        age(&layout);
        let before = tree(&dir);

        // A directory sits where the receipt stages, so the receipt is the
        // one write that cannot land, and it is the last one the run makes.
        std::fs::create_dir_all(
            crate::transaction::stage::scratch_for(&layout.receipt).as_std_path(),
        )
        .unwrap();
        let error = install(&layout, true, false).unwrap_err();
        std::fs::remove_dir(crate::transaction::stage::scratch_for(&layout.receipt).as_std_path())
            .unwrap();

        let message = error.to_string();
        assert!(message.contains(layout.receipt.as_str()), "{message}");
        assert_eq!(tree(&dir), before);
    }

    #[test]
    fn an_unreadable_journal_refuses_the_next_run() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        crate::adapters::fs::write_file(&layout.journal_path(), b"{not json").unwrap();
        let error = install(&layout, true, false).unwrap_err();
        assert_eq!(error.kind(), "Unrecovered");
        assert!(!layout.roots[0].exists(), "the run wrote past the journal");
    }

    #[test]
    fn a_schema_one_receipt_reads_through_the_adapter() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let destination = layout.roots[0].join("sdd-setup/SKILL.md");
        crate::adapters::fs::write_file(&destination, b"older\n").unwrap();
        crate::adapters::fs::write_file(
            &layout.receipt,
            format!(
                "{{\"schema_version\":1,\"written\":{{\"{destination}\":\"{}\"}}}}",
                Sha256::of(b"older\n")
            )
            .as_bytes(),
        )
        .unwrap();
        // The stale copy is the tool's, so the install replaces it without
        // asking, which only happens if the adapter read the old shape.
        install(&layout, true, false).unwrap();
        assert!(
            String::from_utf8_lossy(&std::fs::read(&destination).unwrap())
                .contains("name: sdd-setup")
        );
        assert_eq!(SkillRecord::load(&layout.receipt).schema_version, 2);
    }

    #[test]
    fn a_receipt_at_the_legacy_path_is_read_once_and_rewritten_at_the_resolved_one() {
        let dir = tempfile::tempdir().unwrap();
        let mut layout = home(&dir);
        layout.state_root = root(&dir).join("xdg/spec-driven-docs");
        layout.receipt = layout
            .state_root
            .join(crate::domain::paths::SKILL_RECEIPT_FILE);

        let destination = layout.roots[0].join("sdd-setup/SKILL.md");
        crate::adapters::fs::write_file(&destination, b"older\n").unwrap();
        let mut legacy = SkillRecord::new();
        legacy
            .written
            .insert(destination.clone(), Sha256::of(b"older\n"));
        crate::adapters::fs::write_file(&layout.legacy_receipt, legacy.to_json().as_bytes())
            .unwrap();

        install(&layout, true, false).unwrap();
        assert!(layout.receipt.is_file());
        assert!(
            String::from_utf8_lossy(&std::fs::read(&destination).unwrap())
                .contains("name: sdd-setup")
        );
    }

    #[test]
    fn a_write_that_fails_partway_restores_every_destination() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();

        // Every destination carries recognisable bytes, then one package
        // directory in the second root denies writes, so the failure lands
        // after the first root is already rewritten.
        for root in &layout.roots {
            for name in crate::embedded::skill_names() {
                for path in package_files(root, name) {
                    std::fs::write(&path, format!("previous {name}\n")).unwrap();
                }
            }
        }
        let before = tree(&dir);
        let blocked = layout.roots[1].join("sdd-write-docs");
        let mut permissions = std::fs::metadata(&blocked).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o500);
        std::fs::set_permissions(&blocked, permissions.clone()).unwrap();

        let message = install(&layout, true, true).unwrap_err().to_string();

        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o700);
        std::fs::set_permissions(&blocked, permissions).unwrap();

        assert!(message.contains("skill install aborted"), "{message}");
        assert!(
            message.contains("the destinations were restored"),
            "{message}"
        );
        assert_eq!(tree(&dir), before);
        assert!(!layout.journal_path().exists());
    }

    #[test]
    fn an_uninstall_of_one_root_keeps_the_others_entries() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        uninstall(&select(&layout, 1), true).unwrap();
        let kept = SkillRecord::load(&layout.receipt);
        assert!(
            kept.written
                .keys()
                .all(|path| path.starts_with(&layout.roots[0]))
        );
        assert!(!kept.written.is_empty());
        assert!(
            layout.roots[0]
                .join("sdd-setup/references/plan-gate.md")
                .is_file()
        );
    }

    #[test]
    fn either_agent_alone_lands_a_whole_package() {
        for index in 0..2 {
            let dir = tempfile::tempdir().unwrap();
            let layout = home(&dir);
            let narrowed = select(&layout, index);
            install(&narrowed, true, false).unwrap();
            for path in package_files(&layout.roots[index], "sdd-setup") {
                assert!(path.is_file(), "{path} did not land");
            }
            assert!(!layout.roots[1 - index].exists());
        }
    }

    #[test]
    fn the_last_uninstall_takes_the_receipt_with_it() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        uninstall(&layout, true).unwrap();
        assert!(
            !layout.receipt.exists(),
            "the receipt outlived every file it vouched for"
        );
        for root in &layout.roots {
            assert!(!root.join("sdd-setup").exists());
        }
    }

    #[test]
    fn an_uninstall_on_a_home_this_tool_never_wrote_into_touches_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let before = tree(&dir);
        uninstall(&layout, true).unwrap();
        assert_eq!(tree(&dir), before);
        assert!(!layout.state_root.exists());
    }
}
