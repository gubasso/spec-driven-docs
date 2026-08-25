//! Integration: `sdd gate` runs one gate and lists the registry.

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
fn list_names_every_gate() {
    let fixture = Fixture::new();
    let assert = fixture.cmd().args(["gate", "--list"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    for gate in spec_driven_docs::gates::GATES {
        assert!(
            stdout.contains(&format!("{}: {}", gate.id, gate.name)),
            "{} unlisted",
            gate.id
        );
    }
}

#[test]
fn a_files_scoped_gate_accepts_and_rejects() {
    let fixture = Fixture::new();
    fixture.write("_docs/decisions/ADR-use-slugs.md", "# Record\n");
    fixture
        .cmd()
        .args([
            "gate",
            "adr-filename-shape",
            "_docs/decisions/ADR-use-slugs.md",
        ])
        .current_dir(fixture.path())
        .assert()
        .success();

    fixture.write("_docs/decisions/ADR-use-v2.md", "# Record\n");
    fixture
        .cmd()
        .args([
            "gate",
            "adr-filename-shape",
            "_docs/decisions/ADR-use-v2.md",
        ])
        .current_dir(fixture.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "decision-records:filename-carries-no-digit",
        ));
}

#[test]
fn an_always_run_gate_reads_the_repository() {
    let fixture = Fixture::new();
    fixture.write("AGENTS.md", &"line\n".repeat(101));
    fixture
        .cmd()
        .args(["gate", "agents-digest-size"])
        .current_dir(fixture.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL docs-format:author-instructions-stay-within-budget ./AGENTS.md",
        ));
}

#[test]
fn an_unknown_gate_is_a_clap_error() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["gate", "nonexistent-gate"])
        .assert()
        .code(2);
}
