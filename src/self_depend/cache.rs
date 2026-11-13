//! The per-checkout cache behind the version check.
//!
//! The check runs from the project's shell entry, which fires on every
//! directory entry. It reads this cache and nothing else, unless a refresh
//! is due. A refresh is due seven days after the last success, or one hour
//! after a failed attempt, and it runs under a lock beside the cache so two
//! shells read the network once.

use camino::{Utf8Path, Utf8PathBuf};
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use crate::domain::ownership::Sha256;
use crate::domain::paths::{SELF_DEPEND_CACHE_DIR, SELF_DEPEND_STAMP_DIR};

/// How long a success holds before the next refresh is due.
pub const REFRESH_AFTER: SignedDuration = SignedDuration::from_hours(7 * 24);
/// How long a failed attempt holds before the next refresh is due.
pub const RETRY_AFTER: SignedDuration = SignedDuration::from_hours(1);

/// What the cache records for one checkout.
///
/// Times are RFC 3339 text on disk and compared as times.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// When the last refresh started, success or failure.
    #[serde(default)]
    pub last_attempt: Option<String>,
    /// When the last refresh read the latest release.
    #[serde(default)]
    pub last_success: Option<String>,
    /// The latest release the last success found.
    #[serde(default)]
    pub latest: Option<String>,
    /// Why the last attempt failed, where it failed.
    #[serde(default)]
    pub error: Option<String>,
}

fn time(text: Option<&String>) -> Option<Timestamp> {
    text.and_then(|text| text.parse().ok())
}

impl Entry {
    /// When the last attempt started.
    #[must_use]
    pub fn attempted_at(&self) -> Option<Timestamp> {
        time(self.last_attempt.as_ref())
    }

    /// When the last success happened.
    #[must_use]
    pub fn succeeded_at(&self) -> Option<Timestamp> {
        time(self.last_success.as_ref())
    }

    /// When a failed attempt allows the next try, where the last attempt
    /// failed.
    #[must_use]
    pub fn next_try(&self) -> Option<Timestamp> {
        self.error.as_ref()?;
        self.attempted_at()?
            .checked_add(RETRY_AFTER)
            .ok()?
            .round(jiff::Unit::Second)
            .ok()
    }

    /// The latest release found, as a version.
    #[must_use]
    pub fn latest_version(&self) -> Option<semver::Version> {
        self.latest.as_deref()?.parse().ok()
    }
}

/// The digest that keys one checkout: its absolute path, so two checkouts
/// of one project keep two caches.
fn key(target: &Utf8Path) -> String {
    let digest = Sha256::of(target.as_str().as_bytes());
    digest.as_str()[..16].to_string()
}

/// Where the cache for one target lives under the cache root.
#[must_use]
pub fn path(cache_root: &Utf8Path, target: &Utf8Path) -> Utf8PathBuf {
    cache_root
        .join(SELF_DEPEND_CACHE_DIR)
        .join(format!("{}.json", key(target)))
}

/// The lock beside one cache file.
#[must_use]
pub fn lock_path(cache: &Utf8Path) -> Utf8PathBuf {
    cache.with_extension("lock")
}

/// The entry the cache records, where one is readable.
#[must_use]
pub fn read(path: &Utf8Path) -> Option<Entry> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Record one entry.
///
/// # Errors
///
/// The I/O error where the cache cannot be written.
///
/// The caller holds the lock, so a scratch file left beside the cache is
/// one a killed check left, and it is removed first: otherwise the
/// exclusive scratch create would refuse every later write.
pub fn write(path: &Utf8Path, entry: &Entry) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match std::fs::remove_file(scratch_path(path)) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut text = serde_json::to_string_pretty(entry).map_err(std::io::Error::other)?;
    text.push('\n');
    crate::adapters::fs::write_atomic(path, text.as_bytes())
}

/// The scratch file [`crate::adapters::fs::write_atomic`] writes beside
/// one cache file.
fn scratch_path(path: &Utf8Path) -> Utf8PathBuf {
    path.with_extension(format!("{}.sdd-tmp", path.extension().unwrap_or_default()))
}

/// Whether a refresh is due at `now`.
///
/// After a failed attempt, a refresh is due only one hour after that
/// attempt. Otherwise it is due where the cache is missing, where no
/// success is recorded yet, or where the last success is seven days old.
///
/// SATISFIES acquisition:the-check-refreshes-weekly-and-retries-hourly
#[must_use]
pub fn due(entry: Option<&Entry>, now: Timestamp) -> bool {
    let Some(entry) = entry else {
        return true;
    };
    if entry.error.is_some() {
        return entry
            .attempted_at()
            .is_none_or(|attempt| now.duration_since(attempt) >= RETRY_AFTER);
    }
    entry
        .succeeded_at()
        .is_none_or(|success| now.duration_since(success) >= REFRESH_AFTER)
}

/// A held refresh lock: an advisory lock on the lock file, which closing
/// the file releases, and which the kernel releases when a process dies.
/// The lock file itself stays in place.
#[derive(Debug)]
pub struct Lock {
    _file: std::fs::File,
}

/// Take the refresh lock, or learn that another check holds it.
///
/// `Ok(None)` means another process holds the lock and refreshes the
/// cache, so this check reads the cache alone.
///
/// # Errors
///
/// The I/O error where the lock file cannot be opened or locked for a
/// reason other than another holder.
pub fn try_lock(path: &Utf8Path) -> std::io::Result<Option<Lock>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(Lock { _file: file })),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(error)) => Err(error),
    }
}

/// The retired daily stamps under the state root: every `*.stamp` file in
/// the stamp folder, and nothing else.
#[must_use]
pub fn retired_stamps(state_root: &Utf8Path) -> Vec<Utf8PathBuf> {
    let dir = state_root.join(SELF_DEPEND_STAMP_DIR);
    let Ok(entries) = dir.read_dir_utf8() else {
        return Vec::new();
    };
    let mut found: Vec<Utf8PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path().to_owned())
        .filter(|path| path.extension() == Some("stamp"))
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test panics as its failure signal")]

    use super::*;

    fn at(now: Timestamp, ago: SignedDuration) -> Option<String> {
        now.checked_sub(ago).ok().map(|time| time.to_string())
    }

    /// VERIFIES acquisition:the-check-refreshes-weekly-and-retries-hourly
    #[test]
    fn a_refresh_is_due_weekly_after_success_and_hourly_after_failure() {
        let now = Timestamp::now();
        assert!(due(None, now));
        assert!(due(Some(&Entry::default()), now));
        let fresh = Entry {
            last_attempt: at(now, SignedDuration::from_hours(6 * 24)),
            last_success: at(now, SignedDuration::from_hours(6 * 24)),
            latest: Some("1.0.0".to_string()),
            error: None,
        };
        assert!(!due(Some(&fresh), now));
        let week = Entry {
            last_success: at(now, REFRESH_AFTER),
            ..fresh
        };
        assert!(due(Some(&week), now));
        let failed = Entry {
            last_attempt: at(now, SignedDuration::from_mins(59)),
            last_success: at(now, SignedDuration::from_hours(8 * 24)),
            latest: None,
            error: Some("down".to_string()),
        };
        assert!(!due(Some(&failed), now));
        let hour = Entry {
            last_attempt: at(now, RETRY_AFTER),
            ..failed
        };
        assert!(due(Some(&hour), now));
    }

    #[test]
    fn two_checkouts_keep_two_caches() {
        let root = Utf8Path::new("/cache");
        assert_ne!(
            path(root, Utf8Path::new("/work/a")),
            path(root, Utf8Path::new("/work/b"))
        );
        let one = path(root, Utf8Path::new("/work/a"));
        assert_eq!(lock_path(&one).extension(), Some("lock"));
    }

    #[test]
    fn a_leftover_scratch_file_does_not_block_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let cache = root.join("x.json");
        std::fs::write(scratch_path(&cache), "half").unwrap();
        let entry = Entry {
            latest: Some("1.0.0".to_string()),
            ..Entry::default()
        };
        write(&cache, &entry).unwrap();
        assert_eq!(read(&cache), Some(entry));
        assert!(!scratch_path(&cache).exists());
    }

    #[test]
    fn a_second_lock_on_the_file_fails_while_the_first_is_held() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let lock = root.join("x.lock");
        let held = try_lock(&lock).unwrap();
        assert!(held.is_some());
        assert!(
            try_lock(&lock).unwrap().is_none(),
            "a held lock was taken twice"
        );
        drop(held);
        assert!(lock.exists(), "releasing the lock removed its file");
        assert!(try_lock(&lock).unwrap().is_some());
    }
}
