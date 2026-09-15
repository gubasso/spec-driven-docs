//! The one lock a target takes while it is being written.

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::ownership::Sha256;
use crate::domain::paths::UserEnv;
use crate::error::AppError;
use crate::transaction::lock::Lock;

/// Where this tool keeps state that outlives a command.
///
/// # Errors
///
/// [`AppError::Usage`] when no state root resolves.
pub fn state_root() -> Result<Utf8PathBuf, AppError> {
    Ok(UserEnv::from_process()
        .state_root()
        .ok_or_else(|| AppError::Usage("no state root resolves".to_string()))?
        .path)
}

/// The lock one target takes, keyed by where it is.
///
/// A digest rather than the path itself: a lock file named after a
/// repository would put a person's directory layout in the state root, and
/// two targets whose paths differ only in case would collide.
///
/// # Errors
///
/// [`AppError::Usage`] when no state root resolves.
pub fn path_for(target: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    let key = Sha256::of(target.as_str().as_bytes());
    Ok(state_root()?.join("locks").join(format!("{key}.lock")))
}

/// Hold one target exclusively for the whole of a landing.
///
/// # Errors
///
/// [`AppError::Busy`] when another process holds it, and I/O errors
/// creating the lock file.
pub fn hold(target: &Utf8Path) -> Result<Lock, AppError> {
    Lock::exclusive(&path_for(target)?, "landing")
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    #[test]
    fn two_targets_take_two_locks_and_neither_names_a_directory() {
        let one = path_for(Utf8Path::new("/work/one")).unwrap();
        let two = path_for(Utf8Path::new("/work/two")).unwrap();
        assert_ne!(one, two);
        assert!(!one.as_str().contains("work"), "{one}");
    }
}
