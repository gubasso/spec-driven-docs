//! Integration: `sdd track` reports freshness offline and checks upstreams
//! over a faked network, never touching the target tree.

// Integration tests: assertion style is the point, so the production
// restrictions on unwrap/panic and string building do not apply here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect,
    clippy::case_sensitive_file_extension_comparisons
)]

mod support;

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;
use support::Fixture;

const PINNED: &str = "1111111111111111111111111111111111111111";
const MOVED: &str = "2222222222222222222222222222222222222222";

fn git_entry(revision: &str) -> String {
    format!(
        "schema_version: 1\ntracked:\n  - id: dep\n    path: _docs/reference/tracking.yaml\n    last_checked: 2026-09-01\n    cadence_days: 30\n    why: it moves\n    revalidate:\n      - re-fetch it\n    dependents: []\n    source:\n      kind: git\n      repository: https://github.com/o/r\n      reference: refs/tags/v1\n      revision: {revision}\n      license: MIT\n"
    )
}

/// A fake `git` whose behavior a mode file selects, placed on a PATH dir.
fn fake_git(dir: &std::path::Path, mode: &str) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let script = format!(
        "#!/bin/sh\ncase \"{mode}\" in\n  current) printf '%s\\trefs/tags/v1\\n' {PINNED} ;;\n  moved) printf '%s\\trefs/tags/v1\\n' {MOVED} ;;\n  missing) : ;;\n  fail) echo 'fatal: could not read' >&2; exit 128 ;;\nesac\nexit 0\n"
    );
    let git = bin.join("git");
    std::fs::write(&git, script).unwrap();
    let mut perms = std::fs::metadata(&git).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&git, perms).unwrap();
    bin
}

fn install_with_registry(fixture: &Fixture, revision: &str) {
    fixture.install("knowledge-base");
    fixture.write("_docs/reference/tracking.yaml", &git_entry(revision));
}

fn track_cmd(fixture: &Fixture, path_dir: &std::path::Path, args: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("sdd").unwrap();
    cmd.env_remove("RUST_LOG");
    let existing = std::env::var("PATH").unwrap_or_default();
    cmd.env("PATH", format!("{}:{existing}", path_dir.display()));
    cmd.args(args);
    cmd.arg("--target").arg(fixture.target());
    cmd
}

#[test]
fn status_reports_freshness_offline() {
    let fixture = Fixture::new();
    install_with_registry(&fixture, PINNED);
    fixture
        .cmd()
        .args(["track", "status", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("dep: current"))
        .stdout(predicate::str::contains(PINNED));
}

#[test]
fn status_as_of_a_later_date_reports_overdue() {
    let fixture = Fixture::new();
    install_with_registry(&fixture, PINNED);
    fixture
        .cmd()
        .args([
            "track",
            "status",
            "--target",
            &fixture.target(),
            "--as-of",
            "2027-01-01",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("dep: overdue"));
}

#[test]
fn check_current_leaves_the_tree_unchanged() {
    let fixture = Fixture::new();
    install_with_registry(&fixture, PINNED);
    let path = fake_git(fixture.path(), "current");
    let before = fixture.tree_digest();
    track_cmd(&fixture, &path, &["track", "check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dep: current"));
    assert_eq!(before, fixture.tree_digest(), "check changed the tree");
}

#[test]
fn check_detects_a_moved_reference() {
    let fixture = Fixture::new();
    install_with_registry(&fixture, PINNED);
    let path = fake_git(fixture.path(), "moved");
    track_cmd(&fixture, &path, &["track", "check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("moved"))
        .stdout(predicate::str::contains(MOVED));
}

#[test]
fn fail_on_update_exits_one_on_a_move() {
    let fixture = Fixture::new();
    install_with_registry(&fixture, PINNED);
    let path = fake_git(fixture.path(), "moved");
    track_cmd(&fixture, &path, &["track", "check", "--fail-on-update"])
        .assert()
        .code(1);
}

#[test]
fn a_missing_reference_is_reported_not_an_error() {
    let fixture = Fixture::new();
    install_with_registry(&fixture, PINNED);
    let path = fake_git(fixture.path(), "missing");
    track_cmd(&fixture, &path, &["track", "check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("reference missing upstream"));
}

#[test]
fn a_transport_failure_is_an_operational_error() {
    let fixture = Fixture::new();
    install_with_registry(&fixture, PINNED);
    let path = fake_git(fixture.path(), "fail");
    track_cmd(&fixture, &path, &["track", "check"])
        .assert()
        .code(69)
        .stderr(predicate::str::contains("git"));
}

#[test]
fn check_emits_one_json_object() {
    let fixture = Fixture::new();
    install_with_registry(&fixture, PINNED);
    let path = fake_git(fixture.path(), "moved");
    let assert = track_cmd(&fixture, &path, &["track", "check", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["checked"][0]["state"], "moved");
    assert_eq!(value["checked"][0]["observed"], MOVED);
}

#[test]
fn an_unknown_id_is_a_usage_error() {
    let fixture = Fixture::new();
    install_with_registry(&fixture, PINNED);
    let path = fake_git(fixture.path(), "current");
    track_cmd(&fixture, &path, &["track", "check", "--id", "nope"])
        .assert()
        .code(64);
}
