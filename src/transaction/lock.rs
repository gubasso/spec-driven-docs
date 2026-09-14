//! An OS advisory lock with holder metadata.
//!
//! One writer at a time over one record. The lock is the kernel's, so it is
//! released by drop and by process death and never by a timestamp this tool
//! wrote: a stale lock file cannot outlive the process that took it, and no
//! command has to decide whether an hour-old lock is abandoned.
//!
//! A busy lock refuses at once, naming the holder. There is no wait and no
//! `--force`: waiting turns a fast refusal into a hang, and forcing past a
//! live writer is the interleaving the lock exists to stop.
//!
//! The lock itself is the standard library's, `flock(2)` on Unix, so no
//! dependency carries it and no second implementation can disagree with the
//! kernel about what a lock means.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Whether a lock excludes other readers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Several readers may hold it at once.
    Shared,
    /// One writer holds it alone.
    Exclusive,
}

/// Who holds a lock, for the refusal message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holder {
    /// The holding process.
    pub pid: u32,
    /// What that process is doing.
    pub purpose: String,
    /// When it took the lock.
    pub since: String,
}

/// A held advisory lock. Dropping it releases the lock.
#[derive(Debug)]
pub struct Lock {
    handle: std::fs::File,
    holder: Utf8PathBuf,
    mode: Mode,
}

fn holder_path(path: &Utf8Path) -> Utf8PathBuf {
    Utf8PathBuf::from(format!("{path}.holder"))
}

impl Lock {
    /// Take the lock at `path`, or refuse naming its holder.
    ///
    /// # Errors
    ///
    /// [`AppError::Busy`] when another process holds it, and I/O errors
    /// when the lock file cannot be created.
    pub fn acquire(path: &Utf8Path, mode: Mode, purpose: &str) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let handle = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        let taken = match mode {
            Mode::Shared => handle.try_lock_shared(),
            Mode::Exclusive => handle.try_lock(),
        };
        if taken.is_err() {
            return Err(AppError::Busy(busy_message(path)));
        }
        let holder = holder_path(path);
        let record = Holder {
            pid: std::process::id(),
            purpose: purpose.to_string(),
            since: jiff::Timestamp::now().to_string(),
        };
        // Written after the lock is held, so two processes never race to
        // describe one holder. A failure to describe it is not a failure to
        // hold it, so the refusal message degrades rather than the run.
        let _ = serde_json::to_string(&record)
            .map_err(|_| ())
            .and_then(|text| std::fs::write(&holder, text).map_err(|_| ()));
        Ok(Self {
            handle,
            holder,
            mode,
        })
    }

    /// Take the lock alone.
    ///
    /// # Errors
    ///
    /// As [`Lock::acquire`].
    pub fn exclusive(path: &Utf8Path, purpose: &str) -> Result<Self, AppError> {
        Self::acquire(path, Mode::Exclusive, purpose)
    }

    /// Take the lock alone, waiting a bounded time for a holder to leave.
    ///
    /// For a lock whose critical section is short and whose contention is
    /// ordinary rather than exceptional: two operators planning at once
    /// should queue, not fail. The bound keeps a dead holder from hanging
    /// a command, so a wait that runs out still refuses and names it.
    ///
    /// # Errors
    ///
    /// [`AppError::Busy`] when the wait runs out, and I/O errors when the
    /// lock file cannot be created.
    pub fn exclusive_waiting(
        path: &Utf8Path,
        purpose: &str,
        budget: std::time::Duration,
    ) -> Result<Self, AppError> {
        let deadline = std::time::Instant::now() + budget;
        loop {
            match Self::acquire(path, Mode::Exclusive, purpose) {
                Ok(held) => return Ok(held),
                Err(AppError::Busy(message)) => {
                    if std::time::Instant::now() >= deadline {
                        return Err(AppError::Busy(message));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                Err(other) => return Err(other),
            }
        }
    }

    /// Take the lock beside other readers.
    ///
    /// # Errors
    ///
    /// As [`Lock::acquire`].
    pub fn shared(path: &Utf8Path, purpose: &str) -> Result<Self, AppError> {
        Self::acquire(path, Mode::Shared, purpose)
    }

    /// Whether this lock excludes other readers.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }
}

/// What the refusal says, read from whatever the holder left.
fn busy_message(path: &Utf8Path) -> String {
    let described = std::fs::read_to_string(holder_path(path))
        .ok()
        .and_then(|text| serde_json::from_str::<Holder>(&text).ok())
        .map_or_else(
            || "another process".to_string(),
            |holder| {
                format!(
                    "process {} ({}) since {}",
                    holder.pid, holder.purpose, holder.since
                )
            },
        );
    format!("{path} is held by {described}; wait for it to finish and run this again")
}

impl Drop for Lock {
    fn drop(&mut self) {
        // The holder note describes a lock that is about to be released, so
        // it goes first. The kernel releases the lock when the handle
        // closes, whether or not this succeeds.
        let _ = std::fs::remove_file(&self.holder);
        let _ = self.handle.unlock();
    }
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
    fn an_exclusive_lock_is_released_by_drop() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("skills.lock");
        {
            let held = Lock::exclusive(&path, "install").unwrap();
            assert_eq!(held.mode(), Mode::Exclusive);
            assert!(holder_path(&path).is_file());
        }
        assert!(!holder_path(&path).is_file());
        Lock::exclusive(&path, "install").unwrap();
    }

    #[test]
    fn a_second_holder_is_refused_with_the_first_named() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("skills.lock");
        let _held = Lock::exclusive(&path, "install").unwrap();
        // The same process holds it, and `flock` is per open file
        // description, so a second open refuses exactly as another process
        // would.
        let error = Lock::exclusive(&path, "uninstall").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("is held by"), "{message}");
        assert!(message.contains("install"), "{message}");
        assert_eq!(error.exit_code(), 73);
        assert_eq!(error.kind(), "Busy");
    }

    #[test]
    fn a_shared_lock_admits_a_second_reader_and_refuses_a_writer() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("plans.lock");
        let _first = Lock::shared(&path, "plan").unwrap();
        let _second = Lock::shared(&path, "plan").unwrap();
        assert!(Lock::exclusive(&path, "apply").is_err());
    }

    #[test]
    fn a_lock_file_in_a_missing_directory_is_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("deep/state/skills.lock");
        Lock::exclusive(&path, "install").unwrap();
        assert!(path.is_file());
    }
}
