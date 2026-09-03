//! Integration: `sdd doctor` probes the host and reports by class.

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

use camino::Utf8PathBuf;
use spec_driven_docs::domain::ownership::Sha256;
use spec_driven_docs::domain::skill_record::SkillRecord;
use support::Home;

const RECORD: &str = ".local/state/spec-driven-docs/skills.json";
const SHARED_ROOT: &str = ".local/state/spec-driven-docs/skills/shared";

/// Write an executable stub that exits with `code`.
fn stub(home: &Home, name: &str, code: u8) -> String {
    use std::os::unix::fs::PermissionsExt;
    let path = home.path().join(name);
    std::fs::write(&path, format!("#!/bin/sh\nexit {code}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_str().unwrap().to_string()
}

/// The doctor report, with the two tool probes kept hermetic.
fn report(home: &Home) -> serde_json::Value {
    let out = home
        .cmd()
        .args(["doctor", "--json"])
        .env("SDD_GIT_BIN", "/no/such/git")
        .env("SDD_PRE_COMMIT_BIN", "/no/such/pre-commit")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&out).unwrap()
}

/// One probe's answer, plucked from a report.
fn probe<'a>(report: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    report["probes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|probe| probe["id"] == id)
        .unwrap_or_else(|| panic!("no probe {id}"))
}

/// VERIFIES distribution:the-doctor-answers-for-the-installed-skills
///
/// A doctor run answers on any host: probes fail as results, the exit code
/// stays 0, and the overridable tool binaries keep the test hermetic.
#[test]
fn doctor_reports_every_probe_and_exits_0() {
    let home = Home::new();
    let git = stub(&home, "fake-git", 0);
    let pre_commit = stub(&home, "fake-pre-commit", 1);
    let out = home
        .cmd()
        .args(["doctor", "--json"])
        .env("SDD_GIT_BIN", &git)
        .env("SDD_PRE_COMMIT_BIN", &pre_commit)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(report["schema"], "sdd.doctor/1");
    let ids: Vec<&str> = report["probes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|probe| probe["id"].as_str().expect("an id"))
        .collect();
    assert_eq!(
        ids,
        [
            "state-root",
            "skill-roots",
            "skill-gate",
            "skill-payload",
            "git",
            "pre-commit"
        ]
    );
    assert_eq!(probe(&report, "state-root")["class"], "hard");
    assert_eq!(probe(&report, "state-root")["status"], "ok");
    assert_eq!(probe(&report, "git")["status"], "ok");
    assert_eq!(probe(&report, "pre-commit")["status"], "failed");
    assert_eq!(
        probe(&report, "pre-commit")["remediation"],
        "repair pre-commit; the delivered gates run through it"
    );
}

/// A home with no install is a plan constraint, not a broken host: the
/// roots answer for writability alone, and both install probes name the
/// plain apply.
#[test]
fn the_skill_probes_answer_on_a_home_with_no_install() {
    let home = Home::new();
    let report = report(&home);
    assert_eq!(
        probe(&report, "skill-roots")["status"],
        "ok",
        "an absent root under a writable home is not a refusal"
    );
    for id in ["skill-gate", "skill-payload"] {
        assert_eq!(probe(&report, id)["status"], "failed");
        assert_eq!(
            probe(&report, id)["remediation"],
            "sdd skill install --apply"
        );
    }
}

/// After an apply, every install probe reads the payload back.
#[test]
fn an_installed_home_passes_every_skill_probe() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let report = report(&home);
    for id in ["skill-roots", "skill-gate", "skill-payload"] {
        assert_eq!(probe(&report, id)["status"], "ok", "{id} failed");
    }
}

/// The payload probe names the drift and picks the fix by the record:
/// bytes it vouches for are an older release's, bytes it cannot account
/// for are the operator's own, and an absent file is neither.
#[test]
fn the_payload_probe_tells_a_stale_skill_from_an_edited_one() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let edited = home.path().join(".claude/skills/sdd-setup/SKILL.md");

    // Bytes the record cannot account for are the operator's own.
    std::fs::write(&edited, "mine now\n").unwrap();
    let payload = &report(&home);
    assert_eq!(probe(payload, "skill-payload")["status"], "failed");
    assert_eq!(
        probe(payload, "skill-payload")["remediation"],
        "sdd skill install --apply --force"
    );

    // Bytes the record vouches for are an older release's, and the plain
    // apply corrects them.
    let mut record: SkillRecord = serde_json::from_str(&home.read(RECORD)).unwrap();
    record.written.insert(
        Utf8PathBuf::from(edited.to_str().unwrap()),
        Sha256::of(b"mine now\n"),
    );
    home.write(RECORD, &record.to_json());
    let payload = &report(&home);
    assert_eq!(
        probe(payload, "skill-payload")["remediation"],
        "sdd skill install --apply"
    );

    // A skill destination that is simply absent is neither.
    std::fs::remove_file(&edited).unwrap();
    let payload = &report(&home);
    assert_eq!(
        probe(payload, "skill-payload")["remediation"],
        "sdd skill install --apply"
    );
}

/// The gate probe answers for the shared artifacts alone: a home whose
/// skills are installed but whose gates are gone is exactly the failure it
/// exists to name.
#[test]
fn the_gate_probe_names_a_home_whose_skills_cannot_read_their_gate() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    assert_eq!(probe(&report(&home), "skill-gate")["status"], "ok");

    std::fs::remove_dir_all(home.path().join(SHARED_ROOT)).unwrap();
    let after = report(&home);
    assert_eq!(probe(&after, "skill-gate")["status"], "failed");
    assert!(
        probe(&after, "skill-gate")["message"]
            .as_str()
            .unwrap()
            .contains("gate.md"),
        "the message names the missing artifact"
    );
    assert_eq!(
        probe(&after, "skill-gate")["remediation"],
        "sdd skill install --apply"
    );
    assert_eq!(
        probe(&after, "skill-payload")["status"],
        "ok",
        "the skills are installed; only their gate is missing"
    );
}

/// A symlinked shared-root chain is a layout the installer refuses, so the
/// gate probe must not report it ready.
#[test]
fn the_gate_probe_refuses_a_symlinked_shared_root() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    let state_dir = home.path().join(".local/state/spec-driven-docs");
    let elsewhere = home.path().join("elsewhere");
    std::fs::rename(state_dir.join("skills"), &elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, state_dir.join("skills")).unwrap();
    let after = report(&home);
    assert_eq!(probe(&after, "skill-gate")["status"], "failed");
    assert!(
        probe(&after, "skill-gate")["message"]
            .as_str()
            .unwrap()
            .contains("symlink"),
        "the message names the symlink"
    );
}

/// An absence beside an edit the record cannot vouch for takes the force
/// the edit needs; the plain apply the absence alone would name refuses.
#[test]
fn a_missing_skill_beside_an_unvouched_edit_names_force() {
    let home = Home::new();
    home.cmd()
        .args(["skill", "install", "--apply"])
        .assert()
        .success();
    std::fs::remove_file(home.path().join(".claude/skills/sdd-setup/SKILL.md")).unwrap();
    std::fs::write(
        home.path().join(".claude/skills/sdd-migrate/SKILL.md"),
        "mine now\n",
    )
    .unwrap();
    let after = report(&home);
    assert_eq!(
        probe(&after, "skill-payload")["remediation"],
        "sdd skill install --apply --force"
    );
}
