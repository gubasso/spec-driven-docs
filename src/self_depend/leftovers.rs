//! What a predecessor bump mechanism left, and what a clean never touches.
//!
//! One target runs one mover. A file a predecessor mechanism left beside
//! the sync line is a second mover, dormant or not, so `status` reports it
//! and `clean` removes it. The catalog of known predecessors is empty: no
//! release of this tool shipped a mover before this verb, so a leftover is
//! only ever one the operator names.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::self_depend::manager::Detected;
use crate::self_depend::{ENVRC, SYNC_LINE};

/// Files a predecessor mechanism is known to have left, relative to a
/// project root.
pub const CATALOG: &[&str] = &[];

/// One file a clean removes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Leftover {
    /// The file, relative to the project root.
    pub file: Utf8PathBuf,
    /// Why it is a leftover.
    pub reason: String,
}

/// One file a clean names and leaves byte-identical.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Kept {
    /// The file, relative to the project root.
    pub file: Utf8PathBuf,
    /// The line inside it that a scan must not touch, where one is meant.
    pub line: Option<String>,
    /// Why it stays.
    pub reason: String,
}

/// Every leftover the target carries: the catalog's, plus those named.
#[must_use]
pub fn find(target: &Utf8Path, also: &[Utf8PathBuf]) -> Vec<Leftover> {
    let mut found = Vec::new();
    for file in CATALOG {
        if target.join(file).is_file() {
            found.push(Leftover {
                file: Utf8PathBuf::from(*file),
                reason: "left by a predecessor bump mechanism".to_string(),
            });
        }
    }
    for file in also {
        if target.join(file).is_file() && !found.iter().any(|held| &held.file == file) {
            found.push(Leftover {
                file: file.clone(),
                reason: "named by the operator as a predecessor's file".to_string(),
            });
        }
    }
    found
}

/// What a clean never touches: the wired manager's file, its lock, and the
/// sync line in the shell loader.
#[must_use]
pub fn kept(target: &Utf8Path, wired: &[&Detected]) -> Vec<Kept> {
    let mut held = Vec::new();
    for detected in wired {
        if let Some(file) = detected.file.as_ref() {
            held.push(Kept {
                file: file.clone(),
                line: detected
                    .pin
                    .as_ref()
                    .map(|pin| format!("line {}", pin.line)),
                reason: "the manager file the project owns; the pin moves in place".to_string(),
            });
        }
        if let Some(lock) = detected.manager.lock_file()
            && target.join(lock).is_file()
        {
            held.push(Kept {
                file: Utf8PathBuf::from(lock),
                line: None,
                reason: "the lock the manager owns; it moves with the pin".to_string(),
            });
        }
    }
    if std::fs::read_to_string(target.join(ENVRC)).is_ok_and(|text| text.contains(SYNC_LINE)) {
        held.push(Kept {
            file: Utf8PathBuf::from(ENVRC),
            line: Some(SYNC_LINE.to_string()),
            reason: "the one mover; a scan for a predecessor must not match it".to_string(),
        });
    }
    held
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test panics as its failure signal")]

    use super::*;

    #[test]
    fn a_named_file_is_a_leftover_only_where_it_exists() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(root.join("bump.sh"), "#!/bin/sh\n").unwrap();
        let found = find(
            root,
            &[Utf8PathBuf::from("bump.sh"), Utf8PathBuf::from("absent.sh")],
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file, "bump.sh");
    }

    #[test]
    fn the_sync_line_is_named_as_kept() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(root.join(ENVRC), format!("use flake\n{SYNC_LINE}\n")).unwrap();
        let held = kept(root, &[]);
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].line.as_deref(), Some(SYNC_LINE));
    }
}
