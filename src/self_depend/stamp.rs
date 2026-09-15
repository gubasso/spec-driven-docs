//! The per-checkout stamp that holds the shell-entry caller to one attempt
//! a day.
//!
//! The sync runs from the project's shell entry, which fires on every
//! directory entry. The stamp records the day of the last attempt, success
//! or failure, so a stale pin costs one network read a day and a broken one
//! costs nothing more.

use camino::{Utf8Path, Utf8PathBuf};
use jiff::civil::Date;

use crate::domain::ownership::Sha256;
use crate::domain::paths::SELF_DEPEND_STAMP_DIR;

/// Where the stamp for one target lives under the state root.
///
/// The target's absolute path is digested so two checkouts of one project
/// keep two stamps.
#[must_use]
pub fn path(state_root: &Utf8Path, target: &Utf8Path) -> Utf8PathBuf {
    let digest = Sha256::of(target.as_str().as_bytes());
    state_root
        .join(SELF_DEPEND_STAMP_DIR)
        .join(format!("{}.stamp", &digest.as_str()[..16]))
}

/// The day the stamp records, where one is recorded.
#[must_use]
pub fn read(path: &Utf8Path) -> Option<Date> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Whether an attempt was already made today.
#[must_use]
pub fn attempted(path: &Utf8Path, today: Date) -> bool {
    read(path) == Some(today)
}

/// Record today's attempt.
///
/// # Errors
///
/// The I/O error where the state root cannot be written.
pub fn mark(path: &Utf8Path, today: Date) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::adapters::fs::write_atomic(path, format!("{today}\n").as_bytes())
}

/// Today, in the host's zone.
#[must_use]
pub fn today() -> Date {
    jiff::Zoned::now().date()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test panics as its failure signal")]

    use super::*;

    /// VERIFIES acquisition:the-shell-entry-caller-is-rate-limited-and-silent
    #[test]
    fn a_stamp_holds_one_attempt_per_day_per_checkout() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let one = path(root, Utf8Path::new("/work/a"));
        let two = path(root, Utf8Path::new("/work/b"));
        assert_ne!(one, two);
        let day = Date::constant(2026, 9, 15);
        assert!(!attempted(&one, day));
        mark(&one, day).unwrap();
        assert!(attempted(&one, day));
        assert!(!attempted(&one, Date::constant(2026, 9, 16)));
        assert!(!attempted(&two, day));
    }
}
