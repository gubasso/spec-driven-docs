//! The record a run leaves before its first replacement.
//!
//! Without it, a process that dies partway through a multi-file write
//! leaves no statement of what it was doing, and the in-memory backup map
//! that would restore it dies with the process. The journal names every
//! destination, the digest of what was there, and the digest of what the
//! run intends to put there, and it is written and synced before the first
//! rename.
//!
//! Recovery goes one way: back to what was there. Both writers use it, the
//! skill install and the repository apply alike. A run that did not finish
//! is undone whole, including its record, so the target returns to one
//! state it was in rather than to a mixture of two. Completing forward
//! would need a durable commit marker and a second recovery path, and the
//! cost of rolling back instead is one re-run of a plan that is still
//! stored under its own id.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::domain::ownership::Sha256;
use crate::error::AppError;
use crate::transaction::stage::Stage;
use crate::transaction::sync_parent;

/// The journal schema this binary writes and recovers.
pub const SCHEMA_VERSION: u32 = 1;

/// How far one destination has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryState {
    /// The run intends to touch it and has not yet.
    Planned,
    /// The run has replaced or removed it.
    Done,
}

/// One destination a run touches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The absolute destination.
    pub destination: Utf8PathBuf,
    /// The digest of what was there, or `None` where nothing was.
    pub before: Option<Sha256>,
    /// The digest the run intends to leave, or `None` for a removal.
    pub after: Option<Sha256>,
    /// How far this destination has got.
    pub state: EntryState,
}

impl Entry {
    /// A destination the run will write.
    #[must_use]
    pub const fn write(destination: Utf8PathBuf, before: Option<Sha256>, after: Sha256) -> Self {
        Self {
            destination,
            before,
            after: Some(after),
            state: EntryState::Planned,
        }
    }

    /// A destination the run will remove.
    #[must_use]
    pub const fn remove(destination: Utf8PathBuf, before: Sha256) -> Self {
        Self {
            destination,
            before: Some(before),
            after: None,
            state: EntryState::Planned,
        }
    }
}

/// What one journal file holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// Always [`SCHEMA_VERSION`] once parsed.
    pub schema_version: u32,
    /// Where the copies of the previous bytes are.
    pub backups: Utf8PathBuf,
    /// Every destination the run touches.
    pub entries: Vec<Entry>,
}

/// An open journal. Its file exists until [`Journal::finish`] removes it.
#[derive(Debug)]
pub struct Journal {
    path: Utf8PathBuf,
    record: Record,
}

fn write_record(path: &Utf8Path, record: &Record) -> Result<(), AppError> {
    let text = serde_json::to_string_pretty(record)
        .map_err(|source| anyhow::anyhow!("the journal did not serialize: {source}"))?;
    crate::adapters::fs::write_atomic(path, text.as_bytes())?;
    sync_parent(path)?;
    Ok(())
}

impl Journal {
    /// Write the journal before the first replacement.
    ///
    /// # Errors
    ///
    /// Any I/O error writing or syncing the journal.
    pub fn begin(
        path: &Utf8Path,
        backups: &Utf8Path,
        entries: Vec<Entry>,
    ) -> Result<Self, AppError> {
        let record = Record {
            schema_version: SCHEMA_VERSION,
            backups: backups.to_owned(),
            entries,
        };
        write_record(path, &record)?;
        Ok(Self {
            path: path.to_owned(),
            record,
        })
    }

    /// Mark one destination as reached, and rewrite the journal.
    ///
    /// # Errors
    ///
    /// Any I/O error rewriting the journal.
    pub fn mark_done(&mut self, destination: &Utf8Path) -> Result<(), AppError> {
        for entry in &mut self.record.entries {
            if entry.destination == destination {
                entry.state = EntryState::Done;
            }
        }
        write_record(&self.path, &self.record)
    }

    /// Put every reached destination back, then remove the journal.
    ///
    /// # Errors
    ///
    /// [`AppError::Unrecovered`] when a destination cannot be put back.
    pub fn roll_back(&self) -> Result<(), AppError> {
        roll_back_record(&self.path, &self.record)
    }

    /// Remove the journal, ending the run.
    ///
    /// # Errors
    ///
    /// Any I/O error removing the journal or syncing its directory.
    pub fn finish(&mut self) -> Result<(), AppError> {
        std::fs::remove_file(&self.path)?;
        // The record is gone from disk, so the run is complete whatever
        // the sync reports. A failure here means the removal may not
        // survive a crash, which is worth saying and is not worth undoing
        // a landing that already holds what the plan described.
        sync_parent(&self.path).map_err(AppError::Io)
    }

    /// What this run recorded, for a test that inspects it.
    #[must_use]
    pub const fn record(&self) -> &Record {
        &self.record
    }
}

/// Put every destination the record names back where it was.
fn roll_back_record(path: &Utf8Path, record: &Record) -> Result<(), AppError> {
    let stage = Stage::new(&record.backups)?;
    let mut failed: Vec<String> = Vec::new();
    for entry in &record.entries {
        // A destination already holding what it held needs no copy, which
        // is what makes a second recovery cost nothing.
        let restored = entry.before.as_ref().map_or_else(
            || remove_if_present(&entry.destination),
            |digest| {
                if already_holds(&entry.destination, digest) {
                    Ok(())
                } else {
                    stage.restore(digest, &entry.destination)
                }
            },
        );
        if let Err(source) = restored {
            failed.push(format!("{}: {source}", entry.destination));
        }
    }
    if !failed.is_empty() {
        return Err(AppError::Unrecovered(format!(
            "{path} names destinations that could not be put back: {}",
            failed.join("; ")
        )));
    }
    // Every destination is back, so the record has nothing left to say.
    // Removing it last is what makes a second interruption harmless: the
    // recovery runs again from the same journal and reaches the same place.
    std::fs::remove_file(path)?;
    sync_parent(path)?;
    Ok(())
}

fn already_holds(destination: &Utf8Path, digest: &Sha256) -> bool {
    std::fs::read(destination).is_ok_and(|found| &Sha256::of(&found) == digest)
}

fn remove_if_present(destination: &Utf8Path) -> Result<(), AppError> {
    match std::fs::remove_file(destination) {
        Ok(()) => sync_parent(destination).map_err(AppError::Io),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(AppError::Io(source)),
    }
}

/// Roll back an outstanding run, if one is there.
///
/// Called before a command plans new work. `Ok(false)` means there was
/// nothing to recover. A journal this binary cannot parse is a refusal
/// rather than a fresh start: the destinations it names are in an unknown
/// state, and writing over them would bury that.
///
/// # Errors
///
/// [`AppError::Unrecovered`] when a journal exists and cannot be read or
/// cannot be completed.
pub fn recover(path: &Utf8Path) -> Result<bool, AppError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(source) => return Err(AppError::Io(source)),
    };
    let record: Record = serde_json::from_str(&text).map_err(|source| {
        AppError::Unrecovered(format!(
            "{path} is a journal this binary cannot read: {source}; move it aside once you have checked the destinations it names"
        ))
    })?;
    if record.schema_version != SCHEMA_VERSION {
        return Err(AppError::Unrecovered(format!(
            "{path} is journal schema {}, and this binary recovers schema {SCHEMA_VERSION}",
            record.schema_version
        )));
    }
    roll_back_record(path, &record)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn root(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap())
    }

    #[test]
    fn a_recovery_restores_what_was_there_and_removes_what_was_not() {
        let dir = tempfile::tempdir().unwrap();
        let backups = root(&dir).join("backups");
        let stage = Stage::new(&backups).unwrap();
        let held = root(&dir).join("a/held.md");
        let fresh = root(&dir).join("a/fresh.md");
        crate::adapters::fs::write_file(&held, b"before\n").unwrap();
        let before = stage.back_up(&held).unwrap().unwrap();

        let path = root(&dir).join("run.journal");
        let mut journal = Journal::begin(
            &path,
            &backups,
            vec![
                Entry::write(held.clone(), Some(before), Sha256::of(b"after\n")),
                Entry::write(fresh.clone(), None, Sha256::of(b"after\n")),
            ],
        )
        .unwrap();
        std::fs::write(&held, b"after\n").unwrap();
        journal.mark_done(&held).unwrap();
        crate::adapters::fs::write_file(&fresh, b"after\n").unwrap();

        // The process dies here; the next invocation recovers.
        drop(journal);
        assert!(recover(&path).unwrap());
        assert_eq!(std::fs::read(&held).unwrap(), b"before\n");
        assert!(!fresh.exists());
        assert!(!path.exists());
        assert!(!recover(&path).unwrap());
    }

    #[test]
    fn recovery_is_idempotent_across_a_second_interruption() {
        let dir = tempfile::tempdir().unwrap();
        let backups = root(&dir).join("backups");
        let stage = Stage::new(&backups).unwrap();
        let first = root(&dir).join("first.md");
        let second = root(&dir).join("second.md");
        crate::adapters::fs::write_file(&first, b"one\n").unwrap();
        crate::adapters::fs::write_file(&second, b"two\n").unwrap();
        let one = stage.back_up(&first).unwrap().unwrap();
        let two = stage.back_up(&second).unwrap().unwrap();

        let path = root(&dir).join("run.journal");
        Journal::begin(
            &path,
            &backups,
            vec![
                Entry::write(first.clone(), Some(one), Sha256::of(b"new\n")),
                Entry::write(second.clone(), Some(two), Sha256::of(b"new\n")),
            ],
        )
        .unwrap();
        std::fs::write(&first, b"new\n").unwrap();
        std::fs::write(&second, b"new\n").unwrap();

        // A recovery that itself dies leaves the journal, so the next one
        // does the same work and reaches the same place.
        recover(&path).unwrap();
        assert_eq!(std::fs::read(&first).unwrap(), b"one\n");
        assert_eq!(std::fs::read(&second).unwrap(), b"two\n");
        assert!(!recover(&path).unwrap());
    }

    #[test]
    fn a_finished_run_leaves_no_journal() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("run.journal");
        let mut journal = Journal::begin(&path, &root(&dir).join("backups"), Vec::new()).unwrap();
        assert!(path.is_file());
        journal.finish().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn an_unreadable_journal_refuses_rather_than_starting_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("run.journal");
        std::fs::write(&path, "{not json").unwrap();
        let error = recover(&path).unwrap_err();
        assert_eq!(error.kind(), "Unrecovered");
        assert!(path.is_file(), "the unreadable journal was deleted");
    }
}
