//! Integration: `sdd status` reports an instance's state without gating.

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
use support::Fixture;

fn status_json(fixture: &Fixture) -> serde_json::Value {
    let output = fixture
        .cmd()
        .args(["status", "--target", &fixture.target(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

#[test]
fn a_target_with_no_instance_reports_instance_false_and_exits_zero() {
    let fixture = Fixture::new();
    let report = status_json(&fixture);
    assert_eq!(report["instance"], false);
    assert_eq!(report["profile"], serde_json::Value::Null);
    assert_eq!(report["ok"], serde_json::Value::Null);
    assert_eq!(report["binary_version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn a_fresh_install_is_aligned_and_ok() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let report = status_json(&fixture);
    assert_eq!(report["instance"], true);
    assert_eq!(report["profile"], "codebase");
    assert_eq!(report["docs_root"], "docs");
    assert_eq!(report["alignment"], "aligned");
    assert_eq!(report["canon_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(report["managed_drift"], 0);
    assert_eq!(report["adopted_drift"], 0);
    assert_eq!(report["failures"], 0);
    assert_eq!(report["ok"], true);
}

#[test]
fn an_adopted_edit_counts_as_adopted_drift_and_stays_ok() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("_docs/specs/SPEC-instance.md", "# Edited\n");
    let report = status_json(&fixture);
    assert_eq!(report["adopted_drift"], 1);
    assert_eq!(report["managed_drift"], 0);
    assert_eq!(report["failures"], 0);
    assert_eq!(report["ok"], true);
}

#[test]
fn a_managed_edit_counts_as_managed_drift_and_is_not_ok() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(".claude/skills/sdd-setup/SKILL.md", "edited\n");
    let report = status_json(&fixture);
    assert_eq!(report["managed_drift"], 1);
    assert_eq!(report["ok"], false);
    assert!(report["failures"].as_u64().unwrap() >= 1);
}

#[test]
fn a_corrupt_manifest_exits_sixty_five() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(".spec-driven-docs/manifest.json", "not json");
    fixture
        .cmd()
        .args(["status", "--target", &fixture.target(), "--json"])
        .assert()
        .code(65)
        .stderr(predicate::str::contains("ManifestInvalid"));
}

#[test]
fn text_mode_prints_a_summary() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    fixture
        .cmd()
        .args(["status", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("alignment: aligned"))
        .stdout(predicate::str::contains("managed drift: 0"));
    let empty = Fixture::new();
    empty
        .cmd()
        .args(["status", "--target", &empty.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("no instance at"));
}
