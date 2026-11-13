//! What a predecessor bump mechanism left, and what a clean never touches.
//!
//! One target runs one mover. A file a predecessor mechanism left beside
//! the check line is a second mover, dormant or not, so `status` reports it
//! and `clean` removes it. The catalog of known predecessor files is empty:
//! no release of this tool shipped a mover file before this verb, so a
//! leftover file is only ever one the operator names. The one known
//! predecessor line is this tool's own retired sync line, which a clean
//! swaps for the check line in place.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::self_depend::manager::Detected;
use crate::self_depend::{CHECK_LINE, ENVRC, OLD_SYNC_LINE};

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

/// What a clean does with one retired line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    /// The retired line is replaced by the check line.
    Swap,
    /// The retired line is removed, because the loader already carries the
    /// check line.
    Remove,
}

/// One retired line a clean rewrites in place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Swap {
    /// The file, relative to the project root.
    pub file: Utf8PathBuf,
    /// What happens to the line.
    pub action: Action,
    /// The line the file carries now.
    pub from: String,
    /// The line that replaces it, or `None` where it is removed.
    pub to: Option<String>,
}

/// Whether one loader line is `wanted`: the one matching rule the clean,
/// the kept list, and the status report share. Indentation and a carriage
/// return do not count, and a comment is never a match.
#[must_use]
pub fn is_line(line: &str, wanted: &str) -> bool {
    line.trim() == wanted
}

/// Whether a shell loader's text carries `wanted` as a line.
#[must_use]
pub fn carries(text: &str, wanted: &str) -> bool {
    text.lines().any(|line| is_line(line, wanted))
}

/// Whether a shell loader's text carries the retired sync line.
#[must_use]
pub fn carries_old_line(text: &str) -> bool {
    carries(text, OLD_SYNC_LINE)
}

/// Whether a shell loader's text carries the check line.
#[must_use]
pub fn carries_check_line(text: &str) -> bool {
    carries(text, CHECK_LINE)
}

/// The swaps the shell loader needs, one per retired line it carries.
#[must_use]
pub fn envrc_swaps(target: &Utf8Path) -> Vec<Swap> {
    std::fs::read_to_string(target.join(ENVRC))
        .ok()
        .and_then(|text| swapped(&text))
        .map(|(_, swaps)| swaps)
        .unwrap_or_default()
}

/// The shell loader with each retired line swapped for the check line, and
/// what happened to each, or `None` where it carries no retired line.
///
/// Every other line stays byte for byte, its indentation and line ending
/// included. Where the loader already carries the check line, a retired
/// line is removed rather than doubled.
///
/// SATISFIES acquisition:clean-swaps-the-retired-line
#[must_use]
pub fn swapped(text: &str) -> Option<(String, Vec<Swap>)> {
    if !carries_old_line(text) {
        return None;
    }
    let mut has_check = carries_check_line(text);
    let mut swaps = Vec::new();
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        if !is_line(body, OLD_SYNC_LINE) {
            out.push_str(line);
            continue;
        }
        if has_check {
            swaps.push(Swap {
                file: Utf8PathBuf::from(ENVRC),
                action: Action::Remove,
                from: OLD_SYNC_LINE.to_string(),
                to: None,
            });
            continue;
        }
        let indent = &body[..body.len() - body.trim_start().len()];
        out.push_str(indent);
        out.push_str(CHECK_LINE);
        out.push_str(&line[body.len()..]);
        has_check = true;
        swaps.push(Swap {
            file: Utf8PathBuf::from(ENVRC),
            action: Action::Swap,
            from: OLD_SYNC_LINE.to_string(),
            to: Some(CHECK_LINE.to_string()),
        });
    }
    Some((out, swaps))
}

/// What a clean never touches: the wired manager's file, its lock, and the
/// check line in the shell loader.
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
    if std::fs::read_to_string(target.join(ENVRC)).is_ok_and(|text| carries_check_line(&text)) {
        held.push(Kept {
            file: Utf8PathBuf::from(ENVRC),
            line: Some(CHECK_LINE.to_string()),
            reason: "the version check; a scan for a predecessor must not match it".to_string(),
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
    fn the_check_line_is_named_as_kept() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(root.join(ENVRC), format!("use flake\n{CHECK_LINE}\n")).unwrap();
        let held = kept(root, &[]);
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].line.as_deref(), Some(CHECK_LINE));
    }

    /// VERIFIES acquisition:clean-swaps-the-retired-line
    #[test]
    fn the_swap_keeps_indentation_endings_and_every_other_line() {
        let text = format!("use flake\r\n  {OLD_SYNC_LINE}\r\nexport A=1");
        let (next, swaps) = swapped(&text).unwrap();
        assert_eq!(next, format!("use flake\r\n  {CHECK_LINE}\r\nexport A=1"));
        assert_eq!(swaps.len(), 1);
        assert_eq!(swaps[0].action, Action::Swap);
        let both = format!("  {CHECK_LINE}\r\n{OLD_SYNC_LINE}\n");
        let (next, swaps) = swapped(&both).unwrap();
        assert_eq!(next, format!("  {CHECK_LINE}\r\n"));
        assert_eq!(swaps[0].action, Action::Remove);
        assert_eq!(swaps[0].to, None);
        assert!(swapped(&format!("{CHECK_LINE}\n")).is_none());
        assert!(swapped(&format!("# {OLD_SYNC_LINE}\n")).is_none());
    }
}
