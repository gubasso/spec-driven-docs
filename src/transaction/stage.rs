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
    /// The scratch file is created exclusively. A regular file already
    /// there is one a run that stopped partway left, and it is truncated
    /// and reused so the rerun needs no hand cleanup. Anything else there,
    /// a symlink included, refuses rather than being followed.
    ///
    /// # Errors
    ///
    /// Any I/O error creating the parent, the scratch file, or syncing it.
    pub fn write(destination: &Utf8Path, bytes: &[u8]) -> Result<Utf8PathBuf, AppError> {
        use std::io::Write as _;

        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let scratch = scratch_for(destination);
        let mut handle = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&scratch)
        {
            Ok(handle) => handle,
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
                let held = std::fs::symlink_metadata(&scratch)?;
                if !held.is_file() {
                    return Err(AppError::Io(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        format!(
                            "{scratch} is not a regular file; move it aside and run this again"
                        ),
                    )));
                }
                std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&scratch)?
            }
            Err(source) => {
                return Err(AppError::Io(std::io::Error::new(
                    source.kind(),
                    format!("{scratch}: {source}"),
                )));
            }
        };
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
#[must_use]
pub fn scratch_for(destination: &Utf8Path) -> Utf8PathBuf {
    Utf8PathBuf::from(format!("{destination}{SCRATCH_SUFFIX}"))
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
        let victim = root(&dir).join("victim");
        std::fs::write(&victim, b"keep\n").unwrap();
        std::os::unix::fs::symlink(&victim, scratch_for(&destination).as_std_path()).unwrap();
        assert!(Stage::write(&destination, b"new\n").is_err());
        assert_eq!(std::fs::read(&victim).unwrap(), b"keep\n");
    }

    #[test]
    fn a_scratch_file_a_stopped_run_left_is_reused() {
        let dir = tempfile::tempdir().unwrap();
        let destination = root(&dir).join("SKILL.md");
        std::fs::write(scratch_for(&destination).as_std_path(), b"half a write").unwrap();

        let scratch = Stage::write(&destination, b"new\n").unwrap();
        assert_eq!(std::fs::read(&scratch).unwrap(), b"new\n");
        Stage::replace(&scratch, &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"new\n");
    }
}
