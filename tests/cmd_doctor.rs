//! Integration: `sdd doctor` probes the host and reports by class.

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
use spec_driven_docs::domain::ownership::Sha256;
use spec_driven_docs::domain::skill_record::SkillRecord;
use support::Home;

const RECORD: &str = ".local/state/spec-driven-docs/skills.json";

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
        .env("SDD_PREK_BIN", "/no/such/prek")
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

/// A doctor run answers on any host: probes fail as results, the exit code
/// stays 0, and the overridable tool binaries keep the test hermetic.
#[test]
fn doctor_reports_every_probe_and_exits_0() {
    let home = Home::new();
    let git = stub(&home, "fake-git", 0);
    let prek = stub(&home, "fake-prek", 1);
    let out = home
        .cmd()
        .args(["doctor", "--json"])
        .env("SDD_GIT_BIN", &git)
        .env("SDD_PREK_BIN", &prek)
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
            "skill-shadow",
            "release-registry",
            "git",
            "prek"
        ]
    );
    assert_eq!(probe(&report, "state-root")["class"], "hard");
    assert_eq!(probe(&report, "state-root")["status"], "ok");
    assert_eq!(probe(&report, "git")["status"], "ok");
    assert_eq!(probe(&report, "prek")["status"], "failed");
    assert_eq!(
        probe(&report, "prek")["remediation"],
        "repair prek; the delivered gates run through it"
    );
}

/// A prek that answers its version call passes the probe.
#[test]
fn the_prek_probe_passes_where_prek_answers() {
    let home = Home::new();
    let prek = stub(&home, "fake-prek", 0);
    let out = home
        .cmd()
        .args(["doctor", "--json"])
        .env("SDD_GIT_BIN", "/no/such/git")
        .env("SDD_PREK_BIN", &prek)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let prek = probe(&report, "prek");
    assert_eq!(prek["class"], "soft");
    assert_eq!(prek["status"], "ok");
    assert_eq!(prek["message"], "prek runs");
}

/// A host without prek gets a soft failure that names the install. The
/// probe only reports: pre-commit is no substitute it falls back to.
#[test]
fn the_prek_probe_reports_a_missing_prek() {
    let home = Home::new();
    let report = report(&home);
    let prek = probe(&report, "prek");
    assert_eq!(prek["class"], "soft");
    assert_eq!(prek["status"], "failed");
    assert_eq!(prek["message"], "prek is not on PATH");
    assert_eq!(
        prek["remediation"],
        "install prek; the delivered gates run through it"
    );
}

/// A home with no install is a plan constraint, not a broken host: the
/// roots answer for writability alone, and both install probes name the
/// plain apply.
/// VERIFIES distribution:the-doctor-names-a-shadowing-user-copy
///
/// A home `SKILL.md` of an sdd skill fails the probe and names the cleanup,
/// a folder of notes passes, and a manual the receipt no longer vouches for
/// is named for a hand removal. The probe is soft, and the doctor exits 0.
#[test]
fn the_shadow_probe_names_a_user_scope_manual() {
    let empty = Home::new();
    assert_eq!(probe(&report(&empty), "skill-shadow")["status"], "ok");

    let notes = Home::new();
    notes.write(".claude/skills/sdd-setup/notes.md", "mine\n");
    assert_eq!(probe(&report(&notes), "skill-shadow")["status"], "ok");

    let installed = Home::new();
    let manual = ".agents/skills/sdd-write-docs/SKILL.md";
    let bytes = "an earlier release's manual\n";
    installed.write(manual, bytes);
    let mut record = SkillRecord::new();
    record.written.insert(
        Utf8PathBuf::from_path_buf(installed.path().join(manual)).unwrap(),
        Sha256::of(bytes.as_bytes()),
    );
    installed.write(RECORD, &record.to_json());
    let shadow = report(&installed);
    let shadow = probe(&shadow, "skill-shadow");
    assert_eq!(shadow["status"], "failed");
    assert_eq!(shadow["class"], "soft");
    assert_eq!(shadow["remediation"], "sdd skill uninstall --apply");
    assert!(
        shadow["message"].as_str().unwrap().contains(manual),
        "{shadow}"
    );

    let edited = Home::new();
    edited.write(".claude/skills/sdd-setup/SKILL.md", "my own manual\n");
    let kept = report(&edited);
    let kept = probe(&kept, "skill-shadow");
    assert_eq!(kept["status"], "failed");
    assert!(
        kept["message"].as_str().unwrap().contains("(edited)"),
        "{kept}"
    );
    assert!(
        kept["remediation"]
            .as_str()
            .unwrap()
            .contains("remove it by hand"),
        "{kept}"
    );
}

#[test]
fn the_registry_probe_is_quiet_where_the_host_is_declared_offline() {
    let home = Home::new();
    let report = report(&home);
    let probe = probe(&report, "release-registry");
    assert_eq!(probe["class"], "soft");
    assert_eq!(probe["status"], "ok");
    assert!(
        probe["message"]
            .as_str()
            .unwrap()
            .contains("SDD_OFFLINE is set"),
        "{probe}"
    );
}
