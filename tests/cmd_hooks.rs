//! Integration: `sdd hooks` renders the registry as the managed block.
//!
//! The block is the registry's one delivery, rendered at install time and
//! committed nowhere, so what these hold is that every registered gate
//! reaches the wiring — not that some checked-in copy still agrees.

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

use predicates::prelude::*;
use support::Fixture;

#[test]
fn the_block_carries_markers_verifier_and_every_gate() {
    let fixture = Fixture::new();
    let assert = fixture.cmd().args(["hooks"]).assert().success();
    let block = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(block.starts_with("# BEGIN spec-driven-docs managed\n"));
    assert!(block.trim_end().ends_with("# END spec-driven-docs managed"));
    assert!(block.contains("entry: sdd verify"));
    assert_eq!(
        block.matches("- id: ").count(),
        spec_driven_docs::gates::GATES.len() + 1,
        "the block must wire the verifier plus every gate"
    );
}

#[test]
fn a_custom_entry_prefix_reaches_every_entry() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["hooks", "--entry", "cargo run -q --"])
        .assert()
        .success()
        .stdout(predicate::str::contains("entry: cargo run -q -- verify"))
        .stdout(predicate::str::contains(
            "entry: cargo run -q -- gate adr-word-cap",
        ));
}

// ---------------------------------------------------------------------
// The project declares what its gates judge, and the block is rendered
// from that declaration.
// ---------------------------------------------------------------------

/// The declaration's whole point: a project states an exclusion and the
/// block it never edits carries it.
#[test]
fn the_rendered_block_carries_the_declared_filters() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - AGENTS.md\ngates:\n  no-personal-path:\n    exclude:\n      - third-party/**\n",
    );
    fixture
        .cmd()
        .args([
            "hooks",
            "--target",
            fixture.path().to_str().unwrap(),
            "--apply",
        ])
        .assert()
        .success();

    let config = fixture.read(".pre-commit-config.yaml");
    assert!(
        config.contains("third-party"),
        "the per-gate exclude did not reach the block:\n{config}"
    );
    assert!(
        config.contains("AGENTS"),
        "the reserved path did not reach the block:\n{config}"
    );
}

/// `sdd upgrade` returns early at the same version, so without this command
/// a project could edit its declaration and nothing would ever change.
#[test]
fn apply_splices_the_managed_region_at_the_same_version() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let before = fixture.read(".pre-commit-config.yaml");
    let markers = before.matches("BEGIN spec-driven-docs managed").count();

    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - vendor/**\ngates: {}\n",
    );
    fixture
        .cmd()
        .args([
            "hooks",
            "--target",
            fixture.path().to_str().unwrap(),
            "--apply",
        ])
        .assert()
        .success();

    let after = fixture.read(".pre-commit-config.yaml");
    assert_ne!(before, after, "the apply changed nothing");
    assert_eq!(
        after.matches("BEGIN spec-driven-docs managed").count(),
        markers,
        "the apply appended a second region rather than replacing the one"
    );
    // The instance is whole again: the record follows the rewrite.
    fixture
        .cmd()
        .args(["verify", "--target", fixture.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn check_exits_non_zero_on_a_stale_block_and_zero_once_applied() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let target = fixture.path().to_str().unwrap().to_string();

    fixture
        .cmd()
        .args(["hooks", "--target", &target, "--check"])
        .assert()
        .success();

    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - vendor/**\ngates: {}\n",
    );
    fixture
        .cmd()
        .args(["hooks", "--target", &target, "--check"])
        .assert()
        .code(1);

    fixture
        .cmd()
        .args(["hooks", "--target", &target, "--apply"])
        .assert()
        .success();
    fixture
        .cmd()
        .args(["hooks", "--target", &target, "--check"])
        .assert()
        .success();
}

#[test]
fn apply_refuses_a_malformed_declaration_and_changes_nothing() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let before = fixture.read(".pre-commit-config.yaml");

    fixture.write(
        ".spec-driven-docs/config.yaml",
        "gates:\n  no-such-gate:\n    exclude: [a]\n",
    );
    fixture
        .cmd()
        .args([
            "hooks",
            "--target",
            fixture.path().to_str().unwrap(),
            "--apply",
        ])
        .assert()
        .code(64);

    assert_eq!(
        fixture.read(".pre-commit-config.yaml"),
        before,
        "a refused apply left the configuration changed"
    );
}

#[test]
fn an_upgrade_over_a_declaring_instance_neither_conflicts_nor_drops_it() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let target = fixture.path().to_str().unwrap().to_string();
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - vendor/**\ngates: {}\n",
    );
    fixture
        .cmd()
        .args(["hooks", "--target", &target, "--apply"])
        .assert()
        .success();

    fixture
        .cmd()
        .args(["upgrade", "--target", &target])
        .assert()
        .success();

    assert!(
        fixture
            .read(".spec-driven-docs/config.yaml")
            .contains("vendor/**"),
        "the upgrade dropped the project's declaration"
    );
}
