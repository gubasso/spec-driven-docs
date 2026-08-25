//! Shared helpers for the integration suites.
//!
//! A `Fixture` owns one scratch target repository and builds `sdd`
//! invocations against it with a curated environment, so a developer's
//! `RUST_LOG` cannot leak into assertions. Test semantics live in each
//! `cmd_*.rs`; this only holds the plumbing.

// Shared by every integration binary; not every binary uses every helper,
// and helpers unwrap because a broken fixture should abort the test.
#![allow(dead_code, clippy::unwrap_used, clippy::unused_self)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;

/// One scratch target repository.
pub struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    /// A fresh target holding only `.git/`.
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        Self { dir }
    }

    /// The target's absolute path.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// The target's absolute path as a string argument.
    pub fn target(&self) -> String {
        self.dir.path().to_str().unwrap().to_string()
    }

    /// An `sdd` invocation with a curated environment.
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("sdd").unwrap();
        cmd.env_remove("RUST_LOG");
        cmd
    }

    /// Install the given profile into this target and assert success.
    pub fn install(&self, profile: &str) {
        self.cmd()
            .args([
                "init",
                "--target",
                &self.target(),
                "--profile",
                profile,
                "--apply",
            ])
            .assert()
            .success();
    }

    /// Write a file under the target, creating parents.
    pub fn write(&self, relative: &str, content: &str) {
        let path = self.dir.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    /// Read a file under the target.
    pub fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(relative)).unwrap()
    }

    /// A digest of every file outside `.git/`, for byte-stability checks.
    pub fn tree_digest(&self) -> String {
        let mut entries: Vec<(PathBuf, Vec<u8>)> = walkdir::WalkDir::new(self.dir.path())
            .into_iter()
            .filter_entry(|e| e.file_name() != ".git")
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .map(|e| (e.path().to_path_buf(), std::fs::read(e.path()).unwrap()))
            .collect();
        entries.sort();
        format!("{entries:?}")
    }
}
