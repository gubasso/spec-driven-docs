//! Integration: `sdd hooks` renders the registry, and the committed
//! published manifest equals the render — the delivered set is declared
//! once and reaches both deliveries.

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

#[test]
fn the_published_manifest_is_the_rendered_registry() {
    let fixture = Fixture::new();
    let assert = fixture
        .cmd()
        .args([
            "hooks",
            "--style",
            "manifest",
            "--docs-root",
            "_?docs",
            "--language",
            "rust",
        ])
        .assert()
        .success();
    let rendered = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    let published = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".pre-commit-hooks.yaml"),
    )
    .unwrap();
    let body: String = published
        .lines()
        .skip_while(|line| !line.starts_with("- id: "))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_eq!(
        body, rendered,
        ".pre-commit-hooks.yaml does not match the rendered registry"
    );
}

#[test]
fn the_block_style_carries_markers_verifier_and_every_gate() {
    let fixture = Fixture::new();
    let assert = fixture
        .cmd()
        .args(["hooks", "--style", "block"])
        .assert()
        .success();
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
        .args(["hooks", "--style", "block", "--entry", "cargo run -q --"])
        .assert()
        .success()
        .stdout(predicate::str::contains("entry: cargo run -q -- verify"))
        .stdout(predicate::str::contains(
            "entry: cargo run -q -- gate adr-word-cap",
        ));
}
