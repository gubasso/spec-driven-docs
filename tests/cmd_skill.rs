//! Integration: `sdd skill` lists and prints the embedded skills, refuses to
//! install them under the home, and removes the copies an earlier release
//! installed there.

// Integration tests: assertion style is the point, so the production
// restrictions on unwrap/panic and string building do not apply here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect,
    clippy::case_sensitive_file_extension_comparisons,
    reason = "the panic is this suite's failure signal, not control flow"
)]

mod support;

use std::path::Path;

use camino::Utf8PathBuf;
use predicates::prelude::*;
use spec_driven_docs::domain::ownership::Sha256;
use spec_driven_docs::domain::skill_record::SkillRecord;
use support::Home;

/// Every file of one package under one root, relative to that root's
/// parent, with its package bytes.
fn package(root: &str, name: &str) -> Vec<(String, &'static [u8])> {
    spec_driven_docs::embedded::skill_package(name)
        .expect("the payload carries the skill")
        .into_iter()
        .map(|(relative, bytes)| (format!("{root}/{name}/{relative}"), bytes))
        .collect()
}

/// Every file an earlier release's whole install landed, home-relative.
fn destinations() -> Vec<String> {
    let mut found = Vec::new();
    for root in [".agents/skills", ".claude/skills"] {
        for name in ["sdd-setup", "sdd-write-docs"] {
            found.extend(package(root, name).into_iter().map(|(path, _)| path));
        }
    }
    found
}

/// Where the receipt of an earlier install sits under a home.
const RECORD: &str = ".local/state/spec-driven-docs/skills.json";

/// Write each package under each skill root, and the receipt that vouches
/// for every file: the home an earlier release's install left.
fn as_an_earlier_release_installed_it(home: &Home, roots: &[&Path]) {
    let mut record = SkillRecord::new();
    for root in roots {
        for name in ["sdd-setup", "sdd-write-docs"] {
            for (relative, bytes) in spec_driven_docs::embedded::skill_package(name).unwrap() {
                let path = root.join(name).join(&relative);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, bytes).unwrap();
                record
                    .written
                    .insert(Utf8PathBuf::from_path_buf(path).unwrap(), Sha256::of(bytes));
            }
        }
    }
    home.write(RECORD, &record.to_json());
}

/// Both default skill roots of a home.
fn installed(home: &Home) {
    as_an_earlier_release_installed_it(
        home,
        &[
            &home.path().join(".claude/skills"),
            &home.path().join(".agents/skills"),
        ],
    );
}

#[test]
fn list_prints_every_skill_name_one_per_line() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "list"])
        .assert()
        .success()
        .stdout("sdd-setup\nsdd-write-docs\n");
}

#[test]
fn show_prints_the_frontmatter_and_body() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "show", "sdd-setup"])
        .assert()
        .success()
        .stdout(predicate::str::contains("name: sdd-setup"))
        .stdout(predicate::str::contains("## Pre-flight"));
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

/// VERIFIES distribution:a-skill-is-readable-before-an-instance
///
/// Both shared gates and one ordinary reference print their authored bytes,
/// with no instance and no home copy, and an unknown path exits 66.
#[test]
fn skill_show_prints_one_reference_file() {
    let home = Home::new();
    let package = spec_driven_docs::embedded::skill_package("sdd-setup").unwrap();
    for path in [
        "references/plan-gate.md",
        "references/pre-flight-gate.md",
        "references/landing.md",
        "SKILL.md",
    ] {
        let authored = package
            .iter()
            .find(|(relative, _)| relative == path)
            .unwrap_or_else(|| panic!("the package carries no {path}"))
            .1;
        home.cmd()
            .current_dir(home.path())
            .args(["skill", "show", "sdd-setup", "--file", path])
            .assert()
            .success()
            .stdout(authored);
    }
    home.cmd()
        .args(["skill", "show", "sdd-setup", "--file", "references/nope.md"])
        .assert()
        .code(66)
        .stderr(predicate::str::contains("references/nope.md"));
}

/// VERIFIES distribution:a-skill-has-one-owner
#[test]
fn skill_install_refuses_and_names_the_landing() {
    let home = Home::new();
    for (args, code) in [
        (vec!["skill", "install", "--apply"], 73),
        (vec!["skill", "install"], 0),
    ] {
        let output = home.cmd().args(&args).output().unwrap();
        assert_eq!(output.status.code(), Some(code), "{args:?}");
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(said.contains("`sdd init` and `sdd upgrade`"), "{said}");
        assert!(said.contains("sdd skill uninstall --apply"), "{said}");
    }
    assert!(!home.path().join(".claude").exists());
    assert!(!home.path().join(".agents").exists());
    assert!(!home.path().join(RECORD).exists());
}

/// VERIFIES distribution:skill-uninstall-removes-only-what-it-wrote
///
/// A home holding a copy an earlier release wrote, with its receipt, comes
/// out empty, and a file the operator edited stays.
#[test]
fn skill_uninstall_removes_a_copy_an_earlier_release_wrote() {
    let home = Home::new();
    installed(&home);
    home.write(".agents/skills/sdd-write-docs/SKILL.md", "mine\n");
    home.cmd()
        .args(["skill", "uninstall", "--apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kept (edited):"));
    for destination in destinations() {
        let kept = destination == ".agents/skills/sdd-write-docs/SKILL.md";
        assert_eq!(
            home.path().join(&destination).exists(),
            kept,
            "{destination}"
        );
    }
    assert!(!home.path().join(".claude/skills/sdd-setup").exists());
    assert_eq!(
        home.read(".agents/skills/sdd-write-docs/SKILL.md"),
        "mine\n"
    );
}

/// VERIFIES distribution:skill-uninstall-removes-only-what-it-wrote
#[test]
fn uninstall_previews_by_default_and_removes_nothing() {
    let home = Home::new();
    installed(&home);
    let digest = home.tree_digest();
    let assert = home
        .cmd()
        .args(["skill", "uninstall"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN: no files removed"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    for destination in destinations() {
        assert!(
            stdout.contains(&destination),
            "preview misses {destination}"
        );
    }
    assert_eq!(digest, home.tree_digest());
}

/// VERIFIES distribution:skill-uninstall-removes-only-what-it-wrote
#[test]
fn uninstall_apply_removes_payload_files_and_keeps_foreign_ones() {
    let home = Home::new();
    installed(&home);
    home.write(".claude/skills/sdd-setup/notes.md", "mine\n");
    home.cmd()
        .args(["skill", "uninstall", "--apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kept (not empty):"));
    for destination in destinations() {
        assert!(
            !home.path().join(&destination).exists(),
            "left behind {destination}"
        );
    }
    assert!(!home.path().join(".agents/skills/sdd-setup").exists());
    assert!(!home.path().join(".claude/skills/sdd-write-docs").exists());
    assert_eq!(home.read(".claude/skills/sdd-setup/notes.md"), "mine\n");
}

/// Each root is removed by its own uninstall, and the last one takes the
/// receipt with it.
#[test]
fn each_root_is_removed_by_its_own_uninstall() {
    let home = Home::new();
    installed(&home);
    home.cmd()
        .args(["skill", "uninstall", "--agent", "codex", "--apply"])
        .assert()
        .success();
    for (destination, _) in package(".claude/skills", "sdd-setup") {
        assert!(
            home.path().join(&destination).is_file(),
            "missing {destination}"
        );
    }
    for (destination, _) in package(".agents/skills", "sdd-setup") {
        assert!(
            !home.path().join(&destination).exists(),
            "kept {destination}"
        );
    }
    home.cmd()
        .args(["skill", "uninstall", "--agent", "claude", "--apply"])
        .assert()
        .success();
    for destination in destinations() {
        assert!(
            !home.path().join(&destination).exists(),
            "kept {destination}"
        );
    }
    assert!(!home.path().join(RECORD).exists());
}

#[test]
fn a_second_apply_refuses_while_the_lock_is_held() {
    let home = Home::new();
    installed(&home);
    home.write(".local/state/spec-driven-docs/skills.lock", "");
    let lock = home
        .path()
        .join(".local/state/spec-driven-docs/skills.lock");
    let handle = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock)
        .unwrap();
    handle.try_lock().unwrap();
    let digest = home.tree_digest();
    home.cmd()
        .args(["skill", "uninstall", "--apply"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("busy"));
    handle.unlock().unwrap();
    assert_eq!(digest, home.tree_digest());
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

/// The verb honours the variable that relocates the Claude configuration
/// directory, so a copy an install wrote there is removed there.
#[test]
fn claude_config_dir_relocates_the_claude_root_and_nothing_else() {
    let home = Home::new();
    let elsewhere = tempfile::tempdir().unwrap();
    as_an_earlier_release_installed_it(
        &home,
        &[
            &elsewhere.path().join("skills"),
            &home.path().join(".agents/skills"),
        ],
    );
    home.cmd()
        .env("CLAUDE_CONFIG_DIR", elsewhere.path())
        .args(["skill", "uninstall", "--apply"])
        .assert()
        .success();
    assert!(!elsewhere.path().join("skills/sdd-setup/SKILL.md").exists());
    assert!(
        !home
            .path()
            .join(".agents/skills/sdd-setup/SKILL.md")
            .exists()
    );
}
