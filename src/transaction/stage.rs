//! Scratch files beside their destinations.
//!
//! Staging is always a sibling of the destination, so the rename that
//! follows never crosses a filesystem, whichever mount `HOME`,
//! `XDG_STATE_HOME`, or `CLAUDE_CONFIG_DIR` puts a root on.

use camino::{Utf8Path, Utf8PathBuf};

use crate::error::AppError;
use crate::transaction::sync_parent;

/// The suffix a staged file carries until it is renamed into place.
const SCRATCH_SUFFIX: &str = ".sdd-stage";

/// The staging operations, which hold no state of their own.
#[derive(Debug, Clone, Copy)]
pub struct Stage;

impl Stage {
    /// Write `bytes` to a scratch file beside `destination`.
    ///
    /// The name carries this run, and the file is created exclusively, so
    /// the write never lands in one somebody else left. A scratch path
    /// already taken refuses rather than being reused: nothing proves an
    /// existing file came from a run of this tool, and renaming it over
    /// the destination would replace the destination with its contents.
    ///
    /// # Errors
    ///
    /// Any I/O error creating the parent, the scratch file, or syncing it.
    pub fn write(destination: &Utf8Path, bytes: &[u8]) -> Result<Utf8PathBuf, AppError> {
        Self::write_at(&scratch_for(destination), destination, bytes)
    }

    /// Write `bytes` to one named scratch path beside `destination`.
    ///
    /// The path is the caller's, which is what lets a test plant something
    /// at it and prove the exclusive create refuses rather than follows.
    ///
    /// # Errors
    ///
    /// As [`Stage::write`].
    pub fn write_at(
        scratch: &Utf8Path,
        destination: &Utf8Path,
        bytes: &[u8],
    ) -> Result<Utf8PathBuf, AppError> {
        use std::io::Write as _;

        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let scratch = scratch.to_owned();
        let mut handle = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&scratch)
            .map_err(|source| {
                std::io::Error::new(
                    source.kind(),
                    format!("{scratch}: {source}; move it aside and run this again"),
                )
            })?;
        let written = handle.write_all(bytes).and_then(|()| handle.sync_all());
        drop(handle);
        if let Err(source) = written {
            let _ = std::fs::remove_file(&scratch);
            return Err(AppError::Io(source));
        }
        Ok(scratch)
    }

    /// Rename a staged file over its destination and sync the directory.
    ///
    /// # Errors
    ///
    /// Any I/O error of the rename or the sync. The scratch file is removed
    /// on failure, so a retry is not blocked by its own leftover.
    pub fn replace(scratch: &Utf8Path, destination: &Utf8Path) -> Result<(), AppError> {
        if let Err(source) = std::fs::rename(scratch, destination) {
            let _ = std::fs::remove_file(scratch);
            return Err(AppError::Io(source));
        }
        sync_parent(destination)?;
        Ok(())
    }

    /// Drop a staged file that will not be used.
    pub fn discard(scratch: &Utf8Path) {
        let _ = std::fs::remove_file(scratch);
    }
}

/// The scratch path one destination stages through.
///
/// The name carries the process and a counter, so two runs never choose
/// one path and a leftover never looks like this run's own.
#[must_use]
pub fn scratch_for(destination: &Utf8Path) -> Utf8PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    Utf8PathBuf::from(format!(
        "{destination}{SCRATCH_SUFFIX}.{}-{serial}",
        std::process::id()
    ))
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
    fn a_staged_file_sits_beside_its_destination_and_lands_by_rename() {
        let dir = tempfile::tempdir().unwrap();
        let destination = root(&dir).join("a/b/SKILL.md");
        let scratch = Stage::write(&destination, b"new\n").unwrap();
        assert_eq!(scratch.parent(), destination.parent());
        assert!(!destination.exists());
        Stage::replace(&scratch, &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"new\n");
        assert!(!scratch.exists());
    }

    #[test]
    fn a_pre_existing_scratch_path_refuses_rather_than_being_followed() {
        let dir = tempfile::tempdir().unwrap();
        let destination = root(&dir).join("SKILL.md");
        let scratch = root(&dir).join("SKILL.md.sdd-stage.taken");
        let victim = root(&dir).join("victim");
        std::fs::write(&victim, b"keep\n").unwrap();
        std::os::unix::fs::symlink(&victim, scratch.as_std_path()).unwrap();
        assert!(Stage::write_at(&scratch, &destination, b"new\n").is_err());
        assert_eq!(std::fs::read(&victim).unwrap(), b"keep\n");
    }

    #[test]
    fn a_leftover_scratch_file_is_never_reused_as_this_runs_own() {
        let dir = tempfile::tempdir().unwrap();
        let destination = root(&dir).join("SKILL.md");
        // What a stopped run leaves, under the suffix but not this run's
        // name. It is neither read nor renamed over the destination.
        let leftover = root(&dir).join("SKILL.md.sdd-stage.1-0");
        std::fs::write(leftover.as_std_path(), b"half a write").unwrap();

        let scratch = Stage::write(&destination, b"new\n").unwrap();
        assert_ne!(scratch, leftover);
        Stage::replace(&scratch, &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"new\n");
        assert_eq!(std::fs::read(&leftover).unwrap(), b"half a write");
    }

    #[test]
    fn two_scratch_paths_for_one_destination_never_collide() {
        let destination = Utf8Path::new("/work/AGENTS.md");
        assert_ne!(scratch_for(destination), scratch_for(destination));
    }
}
