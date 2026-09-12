//! Recoverable multi-file writes.
//!
//! Three primitives, used first by the user-scope skill installer and
//! reused by the repository apply. A lock serializes writers over one
//! record. A stage writes every intended byte beside its destination and
//! replaces it by rename. A journal names every destination before the
//! first replacement, so a run the process did not finish is rolled back by
//! the next invocation rather than left half-applied.
//!
//! What this guarantees, and what it does not. Each replacement is atomic
//! on its own; the set is not. A process that dies at any boundary of the
//! persistence order below leaves a journal, and the next invocation
//! restores every destination before it plans new work. Recovery after
//! power loss rests on that order and on the platform's `fsync` semantics,
//! and is claimed no further.
//!
//! The persistence order is fixed and the same for every domain. Every
//! backup copy is written and synced. The journal is written and synced,
//! and its directory synced. Each scratch file is written and synced, then
//! renamed over its destination, then the destination's directory synced.
//! Each journal state change is a rewrite through the same pair. The record
//! is replaced last. The journal is unlinked and its directory synced.

pub mod journal;
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
