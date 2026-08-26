//! Integration: `sdd skill` lists, prints, and installs the embedded skills.

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

use predicates::prelude::*;
use support::{Fixture, Home};

const DESTINATIONS: &[&str] = &[
    ".agents/skills/sdd-authoring/SKILL.md",
    ".agents/skills/sdd-docs/SKILL.md",
    ".claude/skills/sdd-authoring/SKILL.md",
    ".claude/skills/sdd-docs/SKILL.md",
];

#[test]
fn list_prints_every_skill_name_one_per_line() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "list"])
        .assert()
        .success()
        .stdout("sdd-authoring\nsdd-docs\n");
}

#[test]
fn show_prints_the_frontmatter_and_body() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "show", "sdd-docs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("name: sdd-docs"))
        .stdout(predicate::str::contains("## Land an instance"));
}

#[test]
fn show_of_an_unknown_skill_exits_sixty_four() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "show", "nope"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("no such skill: nope"));
}

/// VERIFIES distribution:skill-install-previews-before-writing
#[test]
fn install_previews_by_default_and_writes_nothing() {
    let home = Home::new();
    let digest = home.tree_digest();
    let assert = home
        .cmd()
        .args(["skill", "install"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN: no files written"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    for destination in DESTINATIONS {
        assert!(stdout.contains(destination), "preview misses {destination}");
    }
    assert_eq!(digest, home.tree_digest());
}

/// VERIFIES distribution:skills-are-part-of-the-payload
#[test]
fn install_apply_writes_both_roots_for_all_agents() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    for destination in DESTINATIONS {
        assert!(
            home.path().join(destination).is_file(),
            "missing {destination}"
        );
    }
    assert_eq!(
        home.read(".claude/skills/sdd-docs/SKILL.md"),
        home.read(".agents/skills/sdd-docs/SKILL.md")
    );
}

#[test]
fn install_apply_for_claude_writes_one_root() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--agent", "claude", "--apply"])
        .assert()
        .success();
    assert!(
        home.path()
            .join(".claude/skills/sdd-docs/SKILL.md")
            .is_file()
    );
    assert!(!home.path().join(".agents").exists());
}

#[test]
fn a_reinstall_is_idempotent() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let digest = home.tree_digest();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    assert_eq!(digest, home.tree_digest());
}

/// VERIFIES distribution:skill-install-previews-before-writing
#[test]
fn conflicting_destinations_refuse_atomically_listing_every_conflict() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    home.write(".claude/skills/sdd-docs/SKILL.md", "edited\n");
    home.write(".agents/skills/sdd-authoring/SKILL.md", "edited\n");
    let digest = home.tree_digest();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains(".claude/skills/sdd-docs/SKILL.md"))
        .stderr(predicate::str::contains(
            ".agents/skills/sdd-authoring/SKILL.md",
        ))
        .stderr(predicate::str::contains("--force"));
    assert_eq!(digest, home.tree_digest());
}

#[test]
fn force_overwrites_a_conflicting_destination() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    home.write(".claude/skills/sdd-docs/SKILL.md", "edited\n");
    home.cmd()
        .args(["skill", "install", "--apply", "--force"])
        .assert()
        .success();
    assert!(
        home.read(".claude/skills/sdd-docs/SKILL.md")
            .contains("name: sdd-docs")
    );
}

/// VERIFIES distribution:user-scope-files-stay-unrecorded
#[test]
fn a_user_scope_install_leaves_instance_verification_unchanged() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let before = fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let after = home
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(before, after);
    let manifest = fixture.read(".spec-driven-docs/manifest.json");
    assert!(!manifest.contains(home.path().to_str().unwrap()));
}

#[test]
fn install_without_a_home_exits_sixty_four() {
    let home = Home::new();
    home.cmd()
        .env_remove("HOME")
        .args(["skill", "install"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("HOME is not set"));
}

/// VERIFIES distribution:skill-uninstall-removes-only-payload-files
#[test]
fn uninstall_previews_by_default_and_removes_nothing() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let digest = home.tree_digest();
    let assert = home
        .cmd()
        .args(["skill", "uninstall"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN: no files removed"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    for destination in DESTINATIONS {
        assert!(stdout.contains(destination), "preview misses {destination}");
    }
    assert_eq!(digest, home.tree_digest());
}

/// VERIFIES distribution:skill-uninstall-removes-only-payload-files
#[test]
fn uninstall_apply_removes_payload_files_and_keeps_foreign_ones() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    home.write(".claude/skills/sdd-docs/notes.md", "mine\n");
    home.cmd()
        .args(["skill", "uninstall", "--apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kept (not empty):"));
    for destination in DESTINATIONS {
        assert!(
            !home.path().join(destination).exists(),
            "left behind {destination}"
        );
    }
    assert!(!home.path().join(".agents/skills/sdd-docs").exists());
    assert!(!home.path().join(".claude/skills/sdd-authoring").exists());
    assert_eq!(home.read(".claude/skills/sdd-docs/notes.md"), "mine\n");
}

#[test]
fn uninstall_for_claude_leaves_the_other_root_alone() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    home.cmd()
        .args(["skill", "uninstall", "--agent", "claude", "--apply"])
        .assert()
        .success();
    assert!(!home.path().join(".claude/skills/sdd-docs").exists());
    assert!(
        home.path()
            .join(".agents/skills/sdd-docs/SKILL.md")
            .is_file()
    );
}

#[test]
fn uninstall_on_an_empty_home_is_a_no_op() {
    let home = Home::new();
    let digest = home.tree_digest();
    home.cmd()
        .args(["skill", "uninstall", "--apply"])
        .assert()
        .success();
    assert_eq!(digest, home.tree_digest());
}
