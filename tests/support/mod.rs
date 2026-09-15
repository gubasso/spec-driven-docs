//! Shared helpers for the integration suites.
//!
//! A `Fixture` owns one scratch target repository and builds `sdd`
//! invocations against it with a curated environment, so a developer's
//! `RUST_LOG` cannot leak into assertions. Test semantics live in each
//! `cmd_*.rs`; this only holds the plumbing.

// Shared by every integration binary; not every binary uses every helper,
// and helpers unwrap because a broken fixture should abort the test.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::unused_self,
    reason = "a helper no binary calls is dead code by design here"
)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;

/// Every variable that relocates a user-scope root.
///
/// A developer's shell sets these, and a fixture that inherited one would
/// send an install into the developer's own agent directory rather than
/// into the scratch home. A test that means one of them sets it back.
pub const RELOCATING_VARS: [&str; 3] = ["CLAUDE_CONFIG_DIR", "XDG_STATE_HOME", "XDG_CACHE_HOME"];

/// The suite reaches no registry.
///
/// One probe reads the registry's configuration, and a test that reached
/// the network would be slow where the network is slow and red where it is
/// absent. A test that means the read sets the variable back.
pub const OFFLINE: (&str, &str) = ("SDD_OFFLINE", "1");

/// One scratch target repository, with its own user-scope roots.
pub struct Fixture {
    dir: tempfile::TempDir,
    /// The state and cache roots this fixture's invocations use.
    ///
    /// Removing the variables is not enough: the tool then falls back to
    /// the invoking user's own home, so the plan store, its lock, and the
    /// bundle cache would be shared by every test in the suite and by the
    /// developer running it. Each fixture gets its own.
    home: tempfile::TempDir,
}

impl Fixture {
    /// A fresh target holding only `.git/`.
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        Self {
            dir,
            home: tempfile::tempdir().unwrap(),
        }
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
        // The declared location is named by an environment variable, and a
        // variable set in the developer's shell overrides what a fixture
        // records. Removed here, a test says what it means.
        cmd.env_remove("SDD_DOCS_SCRATCH");
        cmd.env(OFFLINE.0, OFFLINE.1);
        for name in RELOCATING_VARS {
            cmd.env_remove(name);
        }
        cmd.env("XDG_STATE_HOME", self.home.path().join("state"));
        cmd.env("XDG_CACHE_HOME", self.home.path().join("cache"));
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

    /// Every answer an upgrade across this canon's own history needs.
    ///
    /// Releases between 0.6.6 and here carry breaking guidance steps, and
    /// the plan raises each as a decision. A test that means "upgrade
    /// mechanically" is saying it accepts them, so it says so once here
    /// rather than repeating the list.
    pub fn guidance_answers() -> Vec<String> {
        [
            "guidance-coverage=accepted",
            "guidance:0.7.0:re-point-the-retired-rule=accepted",
            "guidance:0.7.0:declare-the-fixture-paths=accepted",
        ]
        .iter()
        .flat_map(|answer| ["--set".to_string(), (*answer).to_string()])
        .collect()
    }

    /// An `sdd upgrade` invocation that accepts every guidance step.
    ///
    /// For an instance old enough that the whole interval applies. An
    /// answer the plan does not offer is a usage error, so a fixture
    /// already past a step uses [`Fixture::upgrade_bare`].
    pub fn upgrade(&self) -> Command {
        let mut cmd = self.upgrade_bare();
        cmd.args(Self::guidance_answers());
        cmd
    }

    /// An `sdd upgrade` invocation that answers nothing.
    pub fn upgrade_bare(&self) -> Command {
        let mut cmd = self.cmd();
        cmd.args(["upgrade", "--target", &self.target()]);
        cmd
    }

    /// Where this fixture's invocations keep state that outlives a command.
    pub fn state_root(&self) -> PathBuf {
        self.home.path().join("state")
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
        tree_digest(self.dir.path())
    }
}

/// A digest of every file under a root outside `.git/`.
pub fn tree_digest(root: &Path) -> String {
    let mut entries: Vec<(PathBuf, Vec<u8>)> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| e.file_name() != ".git")
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| (e.path().to_path_buf(), std::fs::read(e.path()).unwrap()))
        .collect();
    entries.sort();
    format!("{entries:?}")
}

/// One scratch home directory for user-scope installs.
pub struct Home {
    dir: tempfile::TempDir,
}

impl Home {
    /// A fresh empty home.
    pub fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    /// The home's absolute path.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// An `sdd` invocation with `HOME` pointing at this directory.
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("sdd").unwrap();
        cmd.env_remove("RUST_LOG");
        cmd.env("HOME", self.dir.path());
        cmd.env(OFFLINE.0, OFFLINE.1);
        for name in RELOCATING_VARS {
            cmd.env_remove(name);
        }
        cmd
    }

    /// Write a file under the home, creating parents.
    pub fn write(&self, relative: &str, content: &str) {
        let path = self.dir.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    /// Read a file under the home.
    pub fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(relative)).unwrap()
    }

    /// A digest of every file, for byte-stability checks.
    pub fn tree_digest(&self) -> String {
        tree_digest(self.dir.path())
    }
}
