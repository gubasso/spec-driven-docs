//! Integration: what a landing writes passes the gates that landing delivered.
//!
//! The symptom this proves against: a landing reported success, `sdd verify`
//! printed OK, and the project's first hook run then failed on the payload's
//! own files. An operator who applies a landing has to be able to commit it.

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

use support::Fixture;

/// Every delivered gate, run against the tree the landing just wrote.
///
/// The gates run through `sdd gate` rather than through pre-commit: the
/// subject is the landed tree, and pre-commit would add its own network
/// fetch and its own environment to a question about bytes on disk.
fn every_gate_over(fixture: &Fixture) -> Vec<String> {
    let mut failures = Vec::new();
    for gate in spec_driven_docs::gates::GATES {
        let id = gate.id.as_str();
        // A gate judges the working directory, so the run happens in the
        // landed tree rather than wherever the suite started.
        let assert = fixture
            .cmd()
            .current_dir(fixture.path())
            .args(["gate", id])
            .assert();
        let output = assert.get_output();
        if !output.status.success() {
            failures.push(format!(
                "{id}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    failures
}

/// VERIFIES distribution:initialization-preserves-project-content
#[test]
fn a_landing_into_an_empty_repository_passes_every_gate_it_delivered() {
    for profile in ["codebase", "knowledge-base"] {
        let fixture = Fixture::new();
        fixture.install(profile);

        let failures = every_gate_over(&fixture);
        assert!(
            failures.is_empty(),
            "a fresh {profile} landing cannot be committed:\n{}",
            failures.join("\n")
        );
    }
}

/// VERIFIES distribution:initialization-preserves-project-content
#[test]
fn a_fresh_landing_verifies_and_reports_no_drift() {
    let fixture = Fixture::new();
    fixture.install("codebase");

    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
}

/// VERIFIES docs-specs:verification-names-a-live-hook
///
/// Every `Verify:` line a seeded spec carries must name something the
/// adopter can run. A hook the landing does not wire is work the project
/// cannot do, whatever the sentence above it says.
#[test]
fn every_seeded_verification_names_a_hook_the_landing_wires() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");

    let block = fixture.read(".pre-commit-config.yaml");
    let wired: Vec<String> = block
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- id: ").map(str::to_string))
        .chain(
            block
                .lines()
                .filter_map(|line| line.trim().strip_prefix("alias: ").map(str::to_string)),
        )
        .collect();

    let specs = std::fs::read_dir(fixture.path().join("_docs/specs")).unwrap();
    let mut missing = Vec::new();
    for entry in specs.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_none_or(|kind| kind != "md") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("Verify: `pre-commit run ") else {
                continue;
            };
            let Some(hook) = rest.split_whitespace().next() else {
                continue;
            };
            if !wired.iter().any(|id| id == hook) {
                missing.push(format!("{}: {hook}", path.display()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "a seeded verification names a hook the landing does not wire:\n{}",
        missing.join("\n")
    );
}
