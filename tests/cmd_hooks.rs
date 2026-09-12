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

/// The declaration selects the writing-style route the documentation block
/// carries, so the verb that brings the pre-commit block back into agreement
/// brings that block too, and records both hashes.
///
/// VERIFIES writing-policy:the-project-selects-one-source
#[test]
fn changing_the_selection_and_applying_rewrites_the_route() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    assert!(
        fixture
            .read("AGENTS.md")
            .contains("`sdd method writing-style`.")
    );

    let declaration = fixture.read(".spec-driven-docs/config.yaml").replace(
        "writing_style:\n  source: builtin\n  path: null\n",
        "writing_style:\n  source: project\n  path: docs/STYLE.md\n",
    );
    fixture.write(".spec-driven-docs/config.yaml", &declaration);

    // The edit alone leaves the block stale, and both checks say so.
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL the documentation block in AGENTS.md does not match the declaration; run 'sdd hooks --apply'",
        ));
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--check"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("documentation block"));

    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "OK rewrote the documentation block",
        ));
    let agents = fixture.read("AGENTS.md");
    assert!(
        agents.contains("Read the writing style before you author or edit prose: `docs/STYLE.md`.")
    );
    assert!(!agents.contains("sdd method writing-style"));
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--apply"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already matches the declaration"));

    // Selecting none removes the route and leaves every other line.
    let declaration = fixture.read(".spec-driven-docs/config.yaml").replace(
        "writing_style:\n  source: project\n  path: docs/STYLE.md\n",
        "writing_style:\n  source: none\n  path: null\n",
    );
    fixture.write(".spec-driven-docs/config.yaml", &declaration);
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--apply"])
        .assert()
        .success();
    let agents = fixture.read("AGENTS.md");
    assert!(!agents.contains("writing style"), "{agents}");
    assert!(agents.contains("Run `sdd verify` before handoff."));
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
}

#[test]
fn a_disagreeing_selection_is_refused_at_the_declaration() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let declaration = fixture.read(".spec-driven-docs/config.yaml").replace(
        "writing_style:\n  source: builtin\n  path: null\n",
        "writing_style:\n  source: project\n  path: null\n",
    );
    fixture.write(".spec-driven-docs/config.yaml", &declaration);
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--apply"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("writing_style"))
        .stderr(predicate::str::contains("path"));
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("writing_style"));
}

/// A recorded documentation block that is gone is never a green check: the
/// verify counterpart reports the missing block, and this verb agrees
/// rather than reporting the declaration as satisfied.
#[test]
fn a_recorded_block_that_is_gone_fails_the_check_and_refuses_the_apply() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--check"])
        .assert()
        .success();
    fixture.write("AGENTS.md", "# Project\n\nNo block here.\n");
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--check"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("recorded a documentation block"))
        .stdout(predicate::str::contains("sdd init --apply"));
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--apply"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("sdd init --apply"));
    assert_eq!(fixture.read("AGENTS.md"), "# Project\n\nNo block here.\n");

    std::fs::remove_file(fixture.path().join("AGENTS.md")).unwrap();
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--check"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("the file is absent"));
}

/// A target that never recorded a block owes none: the canon's own root
/// digest is release-kit-owned, and its check stays green.
#[test]
fn an_unrecorded_agents_file_is_not_the_verbs_to_judge() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    manifest["integration_blocks"]
        .as_array_mut()
        .unwrap()
        .retain(|block| block["path"] != "AGENTS.md");
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &(serde_json::to_string_pretty(&manifest).unwrap() + "\n"),
    );
    fixture.write("AGENTS.md", "# Project\n");
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--check"])
        .assert()
        .success();
}

/// A manifest that cannot be written puts the region back, and the verb
/// says so rather than reporting the rewrite as complete.
#[test]
fn a_failed_record_update_puts_the_region_back() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let before = fixture.read(".pre-commit-config.yaml");
    let declaration = fixture
        .read(".spec-driven-docs/config.yaml")
        .replace("reserved: []", "reserved:\n  - 'vendor/**'");
    fixture.write(".spec-driven-docs/config.yaml", &declaration);
    std::fs::create_dir(
        fixture
            .path()
            .join(".spec-driven-docs/manifest.json.sdd-tmp"),
    )
    .unwrap();
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--apply"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("put back"));
    assert_eq!(fixture.read(".pre-commit-config.yaml"), before);
}

/// A manifest that records no block for the region is not silently
/// skipped: the rewrite cannot be brought into agreement with a record it
/// does not have, so the region goes back.
#[test]
fn a_manifest_without_the_regions_record_refuses_the_apply() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let before = fixture.read(".pre-commit-config.yaml");
    let declaration = fixture
        .read(".spec-driven-docs/config.yaml")
        .replace("reserved: []", "reserved:\n  - 'vendor/**'");
    fixture.write(".spec-driven-docs/config.yaml", &declaration);
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    manifest["integration_blocks"]
        .as_array_mut()
        .unwrap()
        .retain(|block| block["path"] != ".pre-commit-config.yaml");
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &(serde_json::to_string_pretty(&manifest).unwrap() + "\n"),
    );
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--apply"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("expected one"));
    assert_eq!(fixture.read(".pre-commit-config.yaml"), before);
}
