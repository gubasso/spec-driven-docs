//! The manager axis, and what a target carries of each.
//!
//! A manager owns a file in the repository, and that file records a
//! version. A shell loader such as direnv is not a manager: it loads an
//! environment and records no version, so it is reported beside the list.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::self_depend::pin::{self, Pin};

/// What a project declares its development tools in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Manager {
    /// A Nix flake: an input pinned at a tag and its package in the devshell.
    Flake,
    /// mise: one `[tools]` entry in its configuration file.
    Mise,
    /// asdf: one line in `.tool-versions`.
    Asdf,
    /// devbox: one entry in the `packages` array of `devbox.json`.
    Devbox,
}

impl Manager {
    /// Every manager, in report order.
    pub const ALL: [Self; 4] = [Self::Flake, Self::Mise, Self::Asdf, Self::Devbox];

    /// The kebab-case word the command line and the report use.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Flake => "flake",
            Self::Mise => "mise",
            Self::Asdf => "asdf",
            Self::Devbox => "devbox",
        }
    }

    /// The files the manager reads at a project root, in search order.
    ///
    /// The first is the one a seed writes.
    #[must_use]
    pub const fn files(self) -> &'static [&'static str] {
        match self {
            Self::Flake => &["flake.nix"],
            Self::Mise => &["mise.toml", ".mise.toml"],
            Self::Asdf => &[".tool-versions"],
            Self::Devbox => &["devbox.json"],
        }
    }

    /// The lock beside the manager file, where the manager keeps one.
    #[must_use]
    pub const fn lock_file(self) -> Option<&'static str> {
        match self {
            Self::Flake => Some("flake.lock"),
            Self::Mise | Self::Asdf | Self::Devbox => None,
        }
    }
}

impl std::fmt::Display for Manager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One manager, as a target carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    /// Which manager.
    pub manager: Manager,
    /// The manager file the target carries, relative to its root.
    pub file: Option<Utf8PathBuf>,
    /// The pin the file records for this tool.
    pub pin: Option<Pin>,
}

impl Detected {
    /// Whether the manager file names this tool.
    #[must_use]
    pub const fn names_this_tool(&self) -> bool {
        self.pin.is_some()
    }
}

/// Every manager, detected once, in report order.
///
/// One detection serves the status report and the add verb both.
#[must_use]
pub fn detect(target: &Utf8Path) -> Vec<Detected> {
    Manager::ALL
        .into_iter()
        .map(|manager| detect_one(target, manager))
        .collect()
}

fn detect_one(target: &Utf8Path, manager: Manager) -> Detected {
    for file in manager.files() {
        let path = target.join(file);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        return Detected {
            manager,
            file: Some(Utf8PathBuf::from(*file)),
            pin: pin::read(manager, &text),
        };
    }
    Detected {
        manager,
        file: None,
        pin: None,
    }
}

/// The managers whose file names this tool.
#[must_use]
pub fn wired(detected: &[Detected]) -> Vec<&Detected> {
    detected
        .iter()
        .filter(|held| held.names_this_tool())
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test panics as its failure signal")]

    use super::*;

    #[test]
    fn every_manager_is_detected_once_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(root.join(".tool-versions"), "nodejs 20.0.0\n").unwrap();
        let held = detect(root);
        let order: Vec<Manager> = held.iter().map(|d| d.manager).collect();
        assert_eq!(order, Manager::ALL);
        assert_eq!(
            held[2].file.as_deref(),
            Some(Utf8Path::new(".tool-versions"))
        );
        assert!(held[2].pin.is_none());
        assert!(held[0].file.is_none());
        assert!(wired(&held).is_empty());
    }

    #[test]
    fn a_mise_file_is_found_under_either_name() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        std::fs::write(root.join(".mise.toml"), "[tools]\n").unwrap();
        let held = detect_one(root, Manager::Mise);
        assert_eq!(held.file.as_deref(), Some(Utf8Path::new(".mise.toml")));
    }
}
