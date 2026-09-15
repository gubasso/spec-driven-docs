//! Integration: `sdd skill` lists, prints, and installs the embedded skills.

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

use camino::Utf8PathBuf;
use predicates::prelude::*;
use spec_driven_docs::domain::ownership::Sha256;
use spec_driven_docs::domain::skill_record::SkillRecord;
use support::{Fixture, Home};

/// Every file a whole install lands, home-relative.
///
/// Derived from the payload rather than listed, so a shared artifact added
/// to a package joins every assertion below without an edit here.
fn destinations() -> Vec<String> {
    let mut found = Vec::new();
    for root in [".agents/skills", ".claude/skills"] {
        found.extend(package(root, "sdd-setup"));
        found.extend(package(root, "sdd-write-docs"));
    }
    found
}

/// Every file of one package under one root, home-relative.
fn package(root: &str, name: &str) -> Vec<String> {
    spec_driven_docs::embedded::skill_package(name)
        .expect("the payload carries the skill")
        .into_iter()
        .map(|(relative, _)| format!("{root}/{name}/{relative}"))
        .collect()
}

/// The two files the retired shared root held, home-relative.
const RETIRED: &[&str] = &[RETIRED_GATE, RETIRED_PREFLIGHT];
const RETIRED_GATE: &str = ".local/state/spec-driven-docs/skills/shared/plan-gate.md";
const RETIRED_PREFLIGHT: &str = ".local/state/spec-driven-docs/skills/shared/pre-flight-gate.md";

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
        .stdout(predicate::str::contains("## 2. Request a plan"));
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
    for destination in destinations() {
        assert!(
            stdout.contains(&destination),
            "preview misses {destination}"
        );
    }
    assert_eq!(digest, home.tree_digest());
}

/// VERIFIES distribution:a-skill-has-one-owner
#[test]
fn install_apply_writes_both_roots_for_all_agents() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    for destination in destinations() {
        assert!(
            home.path().join(&destination).is_file(),
            "missing {destination}"
        );
    }
    assert_eq!(
        home.read(".claude/skills/sdd-setup/SKILL.md"),
        home.read(".agents/skills/sdd-setup/SKILL.md")
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
            .join(".claude/skills/sdd-setup/SKILL.md")
            .is_file()
    );
    assert!(!home.path().join(".agents").exists());
    for destination in package(".claude/skills", "sdd-setup") {
        assert!(
            home.path().join(&destination).is_file(),
            "missing {destination}"
        );
    }
    for retired in RETIRED {
        assert!(!home.path().join(retired).exists(), "wrote {retired}");
    }
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

const RECORD: &str = ".local/state/spec-driven-docs/skills.json";
const OLDER: &str = "older canon bytes\n";

/// Leave every destination holding `OLDER`, recorded as this tool's own
/// work — the state an older release's successful apply left behind.
fn as_a_previous_release_left_it(home: &Home) {
    let mut record = SkillRecord::new();
    for destination in destinations() {
        home.write(&destination, OLDER);
        let path = Utf8PathBuf::from_path_buf(home.path().join(&destination)).unwrap();
        record.written.insert(path, Sha256::of(OLDER.as_bytes()));
    }
    home.write(RECORD, &record.to_json());
}

/// VERIFIES distribution:skill-install-previews-before-writing
///
/// The regression this rule exists for: `just install` after a release that
/// edited a skill refused on every destination, because the payload was the
/// installer's only reference and a previous release's bytes are
/// indistinguishable from an edit.
#[test]
fn a_copy_a_previous_release_wrote_is_replaced_without_force() {
    let home = Home::new();
    as_a_previous_release_left_it(&home);
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    for destination in destinations() {
        assert_ne!(
            home.read(&destination),
            OLDER,
            "{destination} was not replaced"
        );
    }
}

/// VERIFIES distribution:skill-install-previews-before-writing
///
/// The record vouches for bytes, not for paths: an edit on top of a stale
/// copy is still the user's and still refuses.
#[test]
fn an_edit_over_a_stale_copy_still_refuses() {
    let home = Home::new();
    as_a_previous_release_left_it(&home);
    home.write(".claude/skills/sdd-setup/SKILL.md", "mine\n");
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains(
            ".claude/skills/sdd-setup/SKILL.md",
        ))
        .stderr(predicate::str::contains("--force"));
    assert_eq!(home.read(".claude/skills/sdd-setup/SKILL.md"), "mine\n");
    assert_eq!(home.read(".agents/skills/sdd-setup/SKILL.md"), OLDER);
}

/// VERIFIES distribution:user-scope-files-stay-unrecorded
///
/// The record is the installer's own state and no verification reads it, so
/// a home with no record still installs — it only loses the benefit of the
/// doubt, which is the pre-record behaviour.
#[test]
fn a_home_with_no_record_installs_and_writes_one() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let record = SkillRecord::load(&Utf8PathBuf::from_path_buf(home.path().join(RECORD)).unwrap());
    for destination in destinations() {
        let path = Utf8PathBuf::from_path_buf(home.path().join(&destination)).unwrap();
        assert!(
            record.written.contains_key(&path),
            "{destination} unrecorded"
        );
    }
    assert_eq!(record.schema_version, 2);
}

/// VERIFIES distribution:a-skill-install-restores-on-failure
#[test]
fn an_apply_that_cannot_write_the_second_root_names_what_it_finished() {
    let home = Home::new();
    as_a_previous_release_left_it(&home);

    // `.agents` is written after `.claude`, so this destination refuses
    // once the first root already holds the new bytes.
    let blocked = home.path().join(".agents/skills/sdd-write-docs");
    std::fs::remove_file(blocked.join("SKILL.md")).unwrap();
    let mut permissions = std::fs::metadata(&blocked).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o500);
    std::fs::set_permissions(&blocked, permissions.clone()).unwrap();

    home.cmd()
        .args(["skill", "install", "--apply", "--force"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("skill install stopped"))
        .stderr(predicate::str::contains("these destinations are written"))
        .stderr(predicate::str::contains(
            ".agents/skills/sdd-write-docs/SKILL.md",
        ));

    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o700);
    std::fs::set_permissions(&blocked, permissions).unwrap();
    // The first root holds the new bytes the report named, and the run that
    // follows finishes the second.
    assert_ne!(home.read(".claude/skills/sdd-setup/SKILL.md"), OLDER);
    assert!(!blocked.join("SKILL.md").exists());

    home.cmd()
        .args(["skill", "install", "--apply", "--force"])
        .assert()
        .success();
    assert_ne!(home.read(".agents/skills/sdd-setup/SKILL.md"), OLDER);
    assert_ne!(home.read(".agents/skills/sdd-write-docs/SKILL.md"), OLDER);
}

/// VERIFIES distribution:skill-install-previews-before-writing
#[test]
fn conflicting_destinations_refuse_atomically_listing_every_conflict() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    home.write(".claude/skills/sdd-setup/SKILL.md", "edited\n");
    home.write(".agents/skills/sdd-write-docs/SKILL.md", "edited\n");
    let digest = home.tree_digest();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains(
            ".claude/skills/sdd-setup/SKILL.md",
        ))
        .stderr(predicate::str::contains(
            ".agents/skills/sdd-write-docs/SKILL.md",
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
    home.write(".claude/skills/sdd-setup/SKILL.md", "edited\n");
    home.cmd()
        .args(["skill", "install", "--apply", "--force"])
        .assert()
        .success();
    assert!(
        home.read(".claude/skills/sdd-setup/SKILL.md")
            .contains("name: sdd-setup")
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

/// Leave one destination under a name the payload does not carry, recorded
/// as this tool's own work — the state a release that renamed a skill leaves.
fn as_a_rename_left_it(home: &Home) -> String {
    let dropped = ".claude/skills/sdd-old-name/SKILL.md";
    home.write(dropped, OLDER);
    let path = Utf8PathBuf::from_path_buf(home.path().join(dropped)).unwrap();
    let record_path = Utf8PathBuf::from_path_buf(home.path().join(RECORD)).unwrap();
    let mut record = SkillRecord::load(&record_path);
    record.written.insert(path, Sha256::of(OLDER.as_bytes()));
    home.write(RECORD, &record.to_json());
    dropped.to_string()
}

/// VERIFIES distribution:an-install-sweeps-what-the-payload-dropped
#[test]
fn an_install_sweeps_a_skill_the_payload_no_longer_carries() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let dropped = as_a_rename_left_it(&home);

    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sweep (no longer in the payload)"))
        .stdout(predicate::str::contains("sdd-old-name"));

    assert!(!home.path().join(&dropped).exists(), "left {dropped}");
    assert!(
        !home.path().join(".claude/skills/sdd-old-name").exists(),
        "the swept skill left its directory behind"
    );
    let record = SkillRecord::load(&Utf8PathBuf::from_path_buf(home.path().join(RECORD)).unwrap());
    assert!(!record.to_json().contains("sdd-old-name"));
    for destination in destinations() {
        assert!(home.path().join(&destination).is_file());
    }
}

/// VERIFIES distribution:an-install-sweeps-what-the-payload-dropped
///
/// A preview names the sweep and performs none of it, and the sweep is what
/// an uninstall owes too — the leftover is ours whichever verb takes it back.
#[test]
fn a_preview_names_the_sweep_and_an_uninstall_performs_it() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let dropped = as_a_rename_left_it(&home);
    let digest = home.tree_digest();

    home.cmd()
        .args(["skill", "install"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sdd-old-name"));
    assert_eq!(digest, home.tree_digest());

    home.cmd()
        .args(["skill", "uninstall", "--apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sdd-old-name"));
    assert!(!home.path().join(&dropped).exists());
    // The shared root itself stays: other tools install skills beside ours.
    assert!(home.path().join(".claude/skills").is_dir());
    assert!(!home.path().join(".claude/skills/sdd-old-name").exists());
    assert!(!home.path().join(".claude/skills/sdd-setup").exists());
}

/// VERIFIES distribution:skill-uninstall-removes-only-what-it-wrote
///
/// The record vouches for bytes, so a leftover the user has since rewritten
/// is theirs and survives both verbs.
#[test]
fn an_edited_leftover_is_never_swept() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let dropped = as_a_rename_left_it(&home);
    home.write(&dropped, "mine\n");

    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    assert_eq!(home.read(&dropped), "mine\n");

    home.cmd()
        .args(["skill", "uninstall", "--apply"])
        .assert()
        .success();
    assert_eq!(home.read(&dropped), "mine\n");
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

/// VERIFIES distribution:skill-uninstall-removes-only-what-it-wrote
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
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
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
    assert!(!home.path().join(".claude/skills/sdd-setup").exists());
    for destination in package(".agents/skills", "sdd-setup") {
        assert!(
            home.path().join(&destination).is_file(),
            "missing {destination}"
        );
    }
}

/// VERIFIES distribution:a-skill-package-is-self-contained
///
/// Each root's packages carry their own references, so an uninstall of one
/// root leaves the other root's gates where they are, and the last uninstall
/// takes the receipt with it.
#[test]
fn each_root_keeps_its_own_references_until_its_own_uninstall() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    home.cmd()
        .args(["skill", "uninstall", "--agent", "codex", "--apply"])
        .assert()
        .success();
    for destination in package(".claude/skills", "sdd-setup") {
        assert!(
            home.path().join(&destination).is_file(),
            "missing {destination}"
        );
    }
    for destination in package(".agents/skills", "sdd-setup") {
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

/// VERIFIES distribution:a-skill-package-is-self-contained
///
/// A home an earlier release left holds the two gates under the retired
/// shared root, and the receipt vouches for them. The first install sweeps
/// them and lands the packages.
#[test]
fn a_home_from_the_shared_root_release_is_swept_and_relanded() {
    let home = Home::new();
    let mut record = SkillRecord::new();
    for retired in RETIRED {
        home.write(retired, OLDER);
        let path = Utf8PathBuf::from_path_buf(home.path().join(retired)).unwrap();
        record.written.insert(path, Sha256::of(OLDER.as_bytes()));
    }
    home.write(RECORD, &record.to_json());

    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sweep (no longer in the payload)"));
    for retired in RETIRED {
        assert!(!home.path().join(retired).exists(), "kept {retired}");
    }
    assert!(
        !home
            .path()
            .join(".local/state/spec-driven-docs/skills/shared")
            .exists()
    );
    for destination in destinations() {
        assert!(
            home.path().join(&destination).is_file(),
            "missing {destination}"
        );
    }
}

/// VERIFIES distribution:skill-uninstall-removes-only-what-it-wrote
///
/// The defect this phase closes: the uninstall consulted the payload rather
/// than the receipt, so a skill the operator had edited was deleted.
#[test]
fn an_edited_skill_survives_an_uninstall_and_is_named_as_kept() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    home.write(".claude/skills/sdd-setup/SKILL.md", "mine\n");
    home.cmd()
        .args(["skill", "uninstall", "--apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains("kept (edited):"))
        .stdout(predicate::str::contains(
            ".claude/skills/sdd-setup/SKILL.md",
        ));
    assert_eq!(home.read(".claude/skills/sdd-setup/SKILL.md"), "mine\n");
}

/// VERIFIES distribution:a-skill-install-restores-on-failure
#[test]
fn a_second_apply_refuses_while_the_lock_is_held() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let lock = home
        .path()
        .join(".local/state/spec-driven-docs/skills.lock");
    let handle = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock)
        .unwrap();
    handle.try_lock().unwrap();
    home.write(".claude/skills/sdd-setup/SKILL.md", "mine\n");
    home.cmd()
        .args(["skill", "install", "--apply", "--force"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("busy"));
    handle.unlock().unwrap();
    assert_eq!(home.read(".claude/skills/sdd-setup/SKILL.md"), "mine\n");
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

#[test]
fn the_agents_value_installs_the_agents_root_and_codex_is_its_alias() {
    for value in ["agents", "codex"] {
        let home = Home::new();
        home.cmd()
            .args(["skill", "install", "--agent", value, "--apply"])
            .assert()
            .success();
        assert!(
            home.path()
                .join(".agents/skills/sdd-setup/SKILL.md")
                .is_file(),
            "--agent {value} did not land the shared root"
        );
        assert!(!home.path().join(".claude").exists());
    }
}

#[test]
fn claude_config_dir_relocates_the_claude_root_and_nothing_else() {
    let home = Home::new();
    let elsewhere = tempfile::tempdir().unwrap();
    home.cmd()
        .env("CLAUDE_CONFIG_DIR", elsewhere.path())
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    assert!(
        elsewhere.path().join("skills/sdd-setup/SKILL.md").is_file(),
        "the relocated Claude root did not receive the package"
    );
    assert!(!home.path().join(".claude").exists());
    assert!(
        home.path()
            .join(".agents/skills/sdd-setup/SKILL.md")
            .is_file(),
        "the shared root moved with the Claude root"
    );
}

#[test]
fn a_preview_names_the_relocated_claude_root() {
    let home = Home::new();
    let elsewhere = tempfile::tempdir().unwrap();
    home.cmd()
        .env("CLAUDE_CONFIG_DIR", elsewhere.path())
        .args(["skill", "install"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            elsewhere
                .path()
                .join("skills/sdd-setup/SKILL.md")
                .to_str()
                .unwrap(),
        ));
}

#[test]
fn two_selected_roots_that_resolve_to_one_path_are_planned_once() {
    let home = Home::new();
    let output = home
        .cmd()
        .env("CLAUDE_CONFIG_DIR", home.path().join(".agents"))
        .args(["skill", "install"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    let landed = text
        .lines()
        .filter(|line| line.ends_with(".agents/skills/sdd-setup/SKILL.md"))
        .count();
    assert_eq!(landed, 1, "the collided root was planned twice:\n{text}");
}
