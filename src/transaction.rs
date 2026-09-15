//! Multi-file writes, one file at a time.
//!
//! Two primitives, used by the user-scope skill installer and by the
//! repository landing. A lock serializes writers over one target. A stage
//! writes every intended byte beside its destination and replaces it by
//! rename.
//!
//! What this guarantees, and what it does not. Each replacement is atomic
//! on its own; the set is not. A run the process does not finish leaves
//! whole files and the record the previous run wrote, and running it again
//! finishes the rest. Recovery after power loss rests on the persistence
//! order below and on the platform's `fsync` semantics, and is claimed no
//! further.
//!
//! The persistence order is fixed and the same for every domain. Each
//! scratch file is written and synced, then renamed over its destination,
//! then the destination's directory synced. The record is replaced last.

pub mod lock;
pub mod stage;

use camino::Utf8Path;

/// Flush a directory entry, so a rename into it survives a crash.
///
/// A platform that refuses to sync a directory reports `InvalidInput`, and
/// there the ordering rests on the platform's own semantics rather than on
/// a call this tool can make.
///
/// # Errors
///
/// Any I/O error other than a refusal to sync a directory at all.
pub fn sync_dir(dir: &Utf8Path) -> std::io::Result<()> {
    match std::fs::File::open(dir).and_then(|handle| handle.sync_all()) {
        Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => Ok(()),
        other => other,
    }
}

/// Flush the directory holding `path`, where it has one.
///
/// # Errors
///
/// Any I/O error other than a refusal to sync a directory at all.
pub fn sync_parent(path: &Utf8Path) -> std::io::Result<()> {
    match path.parent() {
        Some(parent) if !parent.as_str().is_empty() => sync_dir(parent),
        _ => Ok(()),
    }
}
