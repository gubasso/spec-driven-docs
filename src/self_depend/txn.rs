//! The pin transaction: both facts move or neither does.
//!
//! Where the manager records two facts, a tag and its locked node, the tag
//! is rewritten and the lock refreshed by the manager's own command, and a
//! failure at either step puts both files back byte-identical. Where the
//! manager records one fact, that one fact moves in place. Nothing here
//! commits or pushes: what the transaction leaves behind is a diff.

use std::process::{Command, Stdio};

use camino::{Utf8Path, Utf8PathBuf};
use semver::Version;
use serde::Serialize;

use crate::adapters::fs::write_atomic;
use crate::error::AppError;
use crate::self_depend::manager::{Detected, Manager};
use crate::self_depend::pin;

/// What one sync moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Moved {
    /// The manager whose pin moved.
    pub manager: Manager,
    /// The version the pin named before, as the file spelled it.
    pub from: String,
    /// The version the pin names now, in the same spelling.
    pub to: String,
    /// Every file the transaction rewrote, relative to the project root.
    pub files: Vec<Utf8PathBuf>,
}

/// One file's bytes before the transaction, or its absence.
struct Snapshot {
    path: Utf8PathBuf,
    bytes: Option<Vec<u8>>,
}

impl Snapshot {
    fn take(path: Utf8PathBuf) -> std::io::Result<Self> {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        Ok(Self { path, bytes })
    }

    fn restore(&self) -> std::io::Result<()> {
        self.bytes.as_ref().map_or_else(
            || match std::fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            },
            |bytes| write_atomic(&self.path, bytes),
        )
    }
}

/// Move one manager's pin to `to`.
///
/// # Errors
///
/// [`AppError::Refused`] where the manager file no longer records the pin,
/// or where the lock refresh fails; in the second case both files are put
/// back before the error returns. [`AppError::Io`] where a file cannot be
/// read or written.
pub fn sync(target: &Utf8Path, detected: &Detected, to: &Version) -> Result<Moved, AppError> {
    let file = detected.file.as_ref().ok_or_else(|| {
        AppError::Refused(format!("{} names no file in this target", detected.manager))
    })?;
    let manager_path = target.join(file);
    let text = std::fs::read_to_string(&manager_path)?;
    let (moved_text, held) = pin::rewrite(detected.manager, &text, to)
        .map_err(|reason| AppError::Refused(format!("{file}: {reason}")))?;
    let spelled_to = if held.spelled.starts_with('v') {
        format!("v{to}")
    } else {
        to.to_string()
    };

    let Some(lock) = detected.manager.lock_file() else {
        write_atomic(&manager_path, moved_text.as_bytes())?;
        return Ok(Moved {
            manager: detected.manager,
            from: held.spelled,
            to: spelled_to,
            files: vec![file.clone()],
        });
    };

    // SATISFIES acquisition:the-pin-moves-in-one-transaction
    let before_manager = Snapshot::take(manager_path.clone())?;
    let before_lock = Snapshot::take(target.join(lock))?;
    write_atomic(&manager_path, moved_text.as_bytes())?;
    let outcome = refresh_lock(target, &text);
    if let Err(error) = outcome {
        before_manager.restore()?;
        before_lock.restore()?;
        return Err(error);
    }
    Ok(Moved {
        manager: detected.manager,
        from: held.spelled,
        to: spelled_to,
        files: vec![file.clone(), Utf8PathBuf::from(lock)],
    })
}

/// Refresh this tool's node in the lock through the manager's own command.
fn refresh_lock(target: &Utf8Path, flake: &str) -> Result<(), AppError> {
    let input = pin::input_name(flake).ok_or_else(|| {
        AppError::Refused("flake.nix names this tool at a URL no input attribute holds".to_string())
    })?;
    let output = Command::new("nix")
        .args(["flake", "update", &input])
        .current_dir(target)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AppError::Refused(
                    "nix is not on PATH, so the lock cannot follow the tag".to_string(),
                )
            } else {
                AppError::Io(error)
            }
        })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut tail: Vec<&str> = stderr.lines().rev().take(5).collect();
    tail.reverse();
    Err(AppError::Refused(format!(
        "nix flake update {input} failed ({}); both files were put back: {}",
        output.status,
        tail.join(" | ")
    )))
}
