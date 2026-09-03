//! Integration: `sdd assess` classifies a target before anything lands.

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

fn assess_json(fixture: &Fixture) -> serde_json::Value {
    let out = fixture
        .cmd()
        .args(["assess", "--target", &fixture.target(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&out).unwrap()
}

#[test]
fn an_empty_target_classifies_greenfield() {
    let fixture = Fixture::new();
    fixture.write("README.md", "# The project\n");
    fixture.write("CONTRIBUTING.md", "How to contribute.\n");
    let report = assess_json(&fixture);
    assert_eq!(report["schema"], "sdd.assess/1");
    assert_eq!(report["classification"], "greenfield");
    assert_eq!(report["instance"]["instance"], false);
    assert_eq!(report["documents"]["count"], 2);
}

/// VERIFIES distribution:a-landing-classifies-its-target-first
#[test]
fn a_corpus_under_a_doc_root_classifies_brownfield() {
    let fixture = Fixture::new();
    fixture.write("docs/architecture.md", "# Architecture\n");
    let report = assess_json(&fixture);
    assert_eq!(report["classification"], "brownfield");
    assert_eq!(report["doc_roots"][0], "docs");
    assert_eq!(report["documents"]["paths"][0], "docs/architecture.md");
}

#[test]
fn a_methodology_marker_alone_classifies_brownfield() {
    let fixture = Fixture::new();
    fixture.write("README.md", "# The project\n");
    fixture.write("mkdocs.yml", "site_name: x\n");
    let report = assess_json(&fixture);
    assert_eq!(report["classification"], "brownfield");
    assert_eq!(report["methodology_markers"][0], "mkdocs.yml");
}

#[test]
fn scattered_markdown_classifies_needs_decision() {
    let fixture = Fixture::new();
    fixture.write("notes/design.md", "Half a spec.\n");
    let report = assess_json(&fixture);
    assert_eq!(report["classification"], "needs-decision");
}

#[test]
fn an_installed_instance_reports_itself_and_its_collisions() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let report = assess_json(&fixture);
    assert_eq!(report["instance"]["instance"], true);
    assert_eq!(report["instance"]["profile"], "codebase");
    assert!(
        report["collisions"]["codebase"]
            .as_array()
            .unwrap()
            .iter()
            .any(|path| path == "docs/specs/SPEC-docs-format.md"),
        "the seeded destinations read back as collisions"
    );
}

#[test]
fn the_human_report_names_the_classification_and_exits_0() {
    let fixture = Fixture::new();
    fixture.write("notes/design.md", "Half a spec.\n");
    fixture
        .cmd()
        .args(["assess", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("classification: needs-decision"));
}

/// A walk that cannot run maps through the typed I/O matrix: a missing
/// target is 66, kind Io.
#[test]
fn an_unreadable_target_maps_through_the_io_matrix() {
    let fixture = Fixture::new();
    let missing = format!("{}/no-such-dir", fixture.target());
    fixture
        .cmd()
        .args(["assess", "--target", &missing])
        .assert()
        .code(66)
        .stderr(predicate::str::contains("\"kind\":\"Io\""));
}

/// A file target is a usage error at the command boundary.
#[test]
fn a_file_target_exits_sixty_four() {
    let fixture = Fixture::new();
    fixture.write("just-a-file.md", "content\n");
    let target = format!("{}/just-a-file.md", fixture.target());
    fixture
        .cmd()
        .args(["assess", "--target", &target])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("\"kind\":\"Usage\""));
}
