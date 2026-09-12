//! Integration: `sdd reconcile plan` computes one plan and writes nothing.

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

/// One plan, as JSON.
fn plan(fixture: &Fixture, extra: &[&str]) -> serde_json::Value {
    let mut args = vec!["reconcile", "plan", "--target", "TARGET", "--json"];
    args[3] = "TARGET";
    let target = fixture.target();
    let mut command = fixture.cmd();
    command.args(["reconcile", "plan", "--target", &target, "--json"]);
    command.args(extra);
    let out = command.assert().success().get_output().stdout.clone();
    let _ = args;
    serde_json::from_slice(&out).unwrap()
}

#[test]
fn an_empty_target_plans_a_setup_and_waits_for_the_profile() {
    let fixture = Fixture::new();
    let held = plan(&fixture, &[]);
    assert_eq!(held["identity"]["schema"], "sdd.plan/1");
    assert_eq!(held["classification"], "setup");
    assert_eq!(held["readiness"], "needs-decision");
    assert_eq!(held["operations"].as_array().unwrap().len(), 0);
    let decisions: Vec<&str> = held["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|decision| decision["id"].as_str().unwrap())
        .collect();
    assert_eq!(decisions, ["profile"]);
}

/// VERIFIES reconcile:a-decision-precedes-what-depends-on-it
#[test]
fn profile_selection_precedes_profile_relative_operations() {
    let fixture = Fixture::new();
    let held = plan(&fixture, &["--set", "profile=codebase"]);
    assert_eq!(held["classification"], "setup");
    let operations = held["operations"].as_array().unwrap();
    assert!(operations.len() > 20, "{} operations", operations.len());
    for operation in operations {
        assert_eq!(operation["kind"], "write-file");
        // Every operation names digests and no bytes.
        assert!(operation.get("bytes").is_none());
        assert_eq!(operation["after"].as_str().unwrap().len(), 64);
    }
    assert!(
        operations
            .iter()
            .any(|operation| operation["path"] == "docs/specs/SPEC-instance.md"),
        "the codebase profile did not choose docs/"
    );
    // The decisions the profile unlocks are now offered.
    let decisions: Vec<&str> = held["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|decision| decision["id"].as_str().unwrap())
        .collect();
    assert!(decisions.contains(&"plan-zone"));
    assert!(decisions.contains(&"writing-style"));
}

#[test]
fn a_landed_target_plans_current_with_no_operations() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let held = plan(&fixture, &[]);
    assert_eq!(held["classification"], "current");
    assert_eq!(held["readiness"], "ready");
    assert_eq!(held["operations"].as_array().unwrap().len(), 0);
    assert_eq!(held["desired_state"]["profile"], "knowledge-base");
}

#[test]
fn an_edited_managed_file_is_a_blocked_conflict() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        ".spec-driven-docs/markdownlint/adr.markdownlint-cli2.jsonc",
        "{}\n",
    );
    let held = plan(&fixture, &[]);
    assert_eq!(held["classification"], "drift");
    assert_eq!(held["readiness"], "blocked");
    let blocked = held["preconditions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|precondition| precondition["id"] == "managed-files-are-unedited")
        .expect("the conflict precondition");
    assert_eq!(blocked["requirement"], "required");
    assert!(
        blocked["evaluation"]["reason"]
            .as_str()
            .unwrap()
            .contains("adr.markdownlint-cli2.jsonc"),
        "{blocked}"
    );
}

#[test]
fn a_settled_corpus_plans_a_migration_with_its_findings() {
    let fixture = Fixture::new();
    fixture.write("docs/01-intro.md", "# intro\n");
    fixture.write("docs/specs/SPEC-x.md", "# x\n");
    fixture.write("docs/ADR-a-choice.md", "# a\n");
    let held = plan(&fixture, &["--set", "profile=knowledge-base"]);
    assert_eq!(held["classification"], "migration");
    let rules: Vec<&str> = held["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|found| found["rule"].as_str().unwrap())
        .collect();
    assert!(rules.contains(&"foreign-documentation-root"), "{rules:?}");
    assert!(rules.contains(&"ordinal-filename"), "{rules:?}");
    assert!(rules.contains(&"spec-without-a-rule-id"), "{rules:?}");
    assert!(
        rules.contains(&"record-outside-the-decisions-directory"),
        "{rules:?}"
    );
    // Every document that predates the instance is named, never judged.
    assert!(!held["style_candidates"].as_array().unwrap().is_empty());
}

/// VERIFIES reconcile:an-incremental-scope-leaves-no-structural-finding
#[test]
fn incremental_is_offered_only_with_zero_structural_findings() {
    let fixture = Fixture::new();
    // A settled corpus that already keeps its rules where the convention
    // does: the same root, a specifications directory, subject-named
    // documents, and a specification that defines a rule.
    fixture.write("_docs/guide.md", "# guide\n");
    fixture.write("_docs/specs/SPEC-x.md", "# x\n\n### `a-b:c-d` - A rule\n");
    let clean = plan(&fixture, &["--set", "profile=knowledge-base"]);
    assert_eq!(clean["classification"], "migration");
    assert_eq!(clean["findings"].as_array().unwrap().len(), 0);
    let scope = clean["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|decision| decision["id"] == "migration-scope")
        .expect("the scope decision");
    let choices: Vec<&str> = scope["schema"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|choice| choice["id"].as_str().unwrap())
        .collect();
    assert_eq!(choices, ["sweep", "incremental"]);

    fixture.write("_docs/01-intro.md", "# intro\n");
    let structural = plan(&fixture, &["--set", "profile=knowledge-base"]);
    let scope = structural["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|decision| decision["id"] == "migration-scope")
        .expect("the scope decision");
    let choices: Vec<&str> = scope["schema"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|choice| choice["id"].as_str().unwrap())
        .collect();
    assert_eq!(choices, ["sweep"], "incremental was offered over a finding");
}

#[test]
fn malformed_instance_metadata_is_an_invalid_blocked_plan_and_never_setup() {
    let fixture = Fixture::new();
    fixture.write(".spec-driven-docs/manifest.json", "{not json");
    let held = plan(&fixture, &[]);
    assert_eq!(held["classification"], "invalid");
    assert_eq!(held["readiness"], "blocked");
    assert_eq!(held["operations"].as_array().unwrap().len(), 0);
}

#[test]
fn the_planner_is_deterministic_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let before = fixture.tree_digest();
    let one = plan(&fixture, &[]);
    let two = plan(&fixture, &[]);
    assert_eq!(one["input_fingerprint"], two["input_fingerprint"]);
    assert_eq!(one["identity"]["plan_id"], two["identity"]["plan_id"]);
    assert_eq!(before, fixture.tree_digest(), "the plan verb wrote");
}

#[test]
fn changing_a_selected_decision_changes_the_fingerprint() {
    let fixture = Fixture::new();
    let one = plan(&fixture, &["--set", "profile=codebase"]);
    let two = plan(&fixture, &["--set", "profile=knowledge-base"]);
    assert_ne!(one["input_fingerprint"], two["input_fingerprint"]);
}

#[test]
fn an_unknown_or_stale_decision_answer_is_a_usage_error() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args([
            "reconcile",
            "plan",
            "--target",
            &fixture.target(),
            "--set",
            "no-such-decision=x",
        ])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("offers no decision"));
    fixture
        .cmd()
        .args([
            "reconcile",
            "plan",
            "--target",
            &fixture.target(),
            "--set",
            "profile=nonsense",
        ])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("does not offer that answer"));
    fixture
        .cmd()
        .args([
            "reconcile",
            "plan",
            "--target",
            &fixture.target(),
            "--set",
            "nonsense",
        ])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("no '='"));
}

#[test]
fn every_observed_field_cites_evidence() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let held = plan(&fixture, &[]);
    let ids: Vec<&str> = held["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"target"));
    assert!(ids.contains(&"record"));
    assert!(ids.contains(&"release"));
    for cited in held["observed_state"]["evidence_refs"].as_array().unwrap() {
        assert!(
            ids.contains(&cited.as_str().unwrap()),
            "{cited} is cited and not in the ledger"
        );
    }
    for cited in held["release"]["evidence_refs"].as_array().unwrap() {
        assert!(ids.contains(&cited.as_str().unwrap()), "{cited}");
    }
}

/// VERIFIES reconcile:a-plan-writes-nothing
#[test]
fn reconcile_plan_is_offline_by_default() {
    let fixture = Fixture::new();
    // The suite forbids the network outright, so a default that reached it
    // would fail here rather than pass slowly.
    fixture
        .cmd()
        .args(["reconcile", "plan", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("toward"));
}
