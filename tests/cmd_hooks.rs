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
    clippy::case_sensitive_file_extension_comparisons
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
