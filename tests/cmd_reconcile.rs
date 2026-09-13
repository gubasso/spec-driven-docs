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
    // The profile alone unlocks the questions that depend on it and no
    // operation: a first landing is a whole projection, so a partial one
    // is not offered.
    let held = plan(&fixture, &["--set", "profile=codebase"]);
    assert_eq!(held["classification"], "setup");
    assert!(
        held["operations"].as_array().unwrap().is_empty(),
        "a landing was offered before its declarations were settled"
    );
    let decisions: Vec<&str> = held["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|decision| decision["id"].as_str().unwrap())
        .collect();
    assert!(decisions.contains(&"plan-zone"));
    assert!(decisions.contains(&"writing-style"));

    // Every declaration settled, the whole landing appears at once.
    let held = plan(
        &fixture,
        &[
            "--set",
            "profile=codebase",
            "--set",
            "plan-zone=none",
            "--set",
            "docs-scratch=none",
            "--set",
            "writing-style=builtin",
        ],
    );
    let operations = held["operations"].as_array().unwrap();
    assert!(operations.len() > 20, "{} operations", operations.len());
    for operation in operations {
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
    let kinds: std::collections::BTreeSet<&str> = operations
        .iter()
        .map(|operation| operation["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains("write-record"), "{kinds:?}");
    assert!(kinds.contains("splice-block"), "{kinds:?}");
    assert!(kinds.contains("write-file"), "{kinds:?}");
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

/// VERIFIES reconcile:a-plan-is-stored-and-applied-by-its-id
#[test]
fn a_plan_is_stored_under_its_id_and_shown_back() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let held = plan(&fixture, &[]);
    let id = held["identity"]["plan_id"].as_str().unwrap();
    let shown = fixture
        .cmd()
        .args(["reconcile", "show", id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let shown: serde_json::Value = serde_json::from_slice(&shown).unwrap();
    assert_eq!(shown["identity"]["plan_id"], id);
}

#[test]
fn identical_inputs_reuse_the_fingerprint_plan_id() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let one = plan(&fixture, &[]);
    let two = plan(&fixture, &[]);
    assert_eq!(one["identity"]["plan_id"], two["identity"]["plan_id"]);
}

#[test]
fn an_unknown_plan_id_refuses_with_the_next_command() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["reconcile", "show", "not-a-plan"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("sdd reconcile plan"));
}

/// VERIFIES reconcile:a-plan-is-stored-and-applied-by-its-id
///
/// A target that already holds what the plan describes applies to nothing
/// and says so, and the tree is byte-identical afterwards.
#[test]
fn applying_a_current_plan_writes_nothing_and_succeeds() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let held = plan(&fixture, &[]);
    let id = held["identity"]["plan_id"].as_str().unwrap().to_string();
    let before = fixture.tree_digest();
    let out = fixture
        .cmd()
        .args([
            "reconcile",
            "apply",
            &id,
            "--target",
            &fixture.target(),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let result: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(result["schema"], "sdd.result/1");
    assert_eq!(result["disposition"], "succeeded");
    assert_eq!(before, fixture.tree_digest(), "the apply wrote");
}

/// VERIFIES reconcile:an-apply-refuses-a-plan-whose-inputs-moved
#[test]
fn apply_refuses_after_a_destination_changes_and_writes_nothing() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let held = plan(&fixture, &[]);
    let id = held["identity"]["plan_id"].as_str().unwrap().to_string();
    fixture.write(
        ".spec-driven-docs/markdownlint/adr.markdownlint-cli2.jsonc",
        "{}\n",
    );
    let before = fixture.tree_digest();
    fixture
        .cmd()
        .args(["reconcile", "apply", &id, "--target", &fixture.target()])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("no longer describes the target"));
    assert_eq!(before, fixture.tree_digest(), "the refusal wrote");
}

/// VERIFIES reconcile:an-apply-refuses-a-plan-whose-inputs-moved
#[test]
fn apply_refuses_a_plan_that_waits_on_a_decision() {
    let fixture = Fixture::new();
    let held = plan(&fixture, &[]);
    let id = held["identity"]["plan_id"].as_str().unwrap().to_string();
    let before = fixture.tree_digest();
    fixture
        .cmd()
        .args(["reconcile", "apply", &id, "--target", &fixture.target()])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("waits on a decision"))
        .stderr(predicate::str::contains("profile"));
    assert_eq!(before, fixture.tree_digest());
}

/// A first landing is ready once every declaration it records is answered,
/// and it carries the record and both marked regions.
#[test]
fn a_first_landing_plans_the_record_and_both_marked_regions() {
    let fixture = Fixture::new();
    let held = plan(
        &fixture,
        &[
            "--set",
            "profile=codebase",
            "--set",
            "plan-zone=none",
            "--set",
            "docs-scratch=none",
            "--set",
            "writing-style=builtin",
        ],
    );
    assert_eq!(held["readiness"], "ready");
    let operations = held["operations"].as_array().unwrap();
    let kinds: Vec<&str> = operations
        .iter()
        .map(|operation| operation["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds.iter().filter(|kind| **kind == "write-record").count(),
        1,
        "the plan writes no record"
    );
    assert_eq!(
        kinds.iter().filter(|kind| **kind == "splice-block").count(),
        2,
        "the plan splices neither marked region"
    );
}

/// The landing the engine plans is the landing the verb produces.
#[test]
fn the_engine_lands_what_the_verb_lands() {
    let through_verb = Fixture::new();
    through_verb.install("codebase");

    let through_engine = Fixture::new();
    let held = plan(
        &through_engine,
        &[
            "--set",
            "profile=codebase",
            "--set",
            "plan-zone=none",
            "--set",
            "docs-scratch=none",
            "--set",
            "writing-style=builtin",
        ],
    );
    let id = held["identity"]["plan_id"].as_str().unwrap().to_string();
    through_engine
        .cmd()
        .args([
            "reconcile",
            "apply",
            &id,
            "--target",
            &through_engine.target(),
        ])
        .assert()
        .success();
    through_engine
        .cmd()
        .args(["verify", "--target", &through_engine.target()])
        .assert()
        .success();

    // Every file but the record, whose one difference is the moment of
    // installation.
    for relative in [
        ".spec-driven-docs/config.yaml",
        ".spec-driven-docs/markdownlint/adr.markdownlint-cli2.jsonc",
        "docs/specs/SPEC-instance.md",
        ".pre-commit-config.yaml",
        "AGENTS.md",
    ] {
        assert_eq!(
            through_verb.read(relative),
            through_engine.read(relative),
            "{relative} differs between the verb and the engine"
        );
    }
}

/// VERIFIES reconcile:the-target-decides-its-classification
#[test]
fn the_landing_verb_refuses_a_settled_corpus_without_writing() {
    let fixture = Fixture::new();
    fixture.write("_docs/guide.md", "# guide\n");
    let before = fixture.tree_digest();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "knowledge-base",
            "--apply",
        ])
        .assert()
        .code(73)
        .stderr(predicate::str::contains(
            "does not serve a migration target",
        ))
        .stderr(predicate::str::contains("sdd reconcile plan"));
    assert_eq!(before, fixture.tree_digest(), "the refusal wrote");
}

/// VERIFIES reconcile:one-plan-is-the-input-to-every-write
///
/// The compatibility fronts are short forms over the engine, not a second
/// write path. What proves it is the store: a landing they performed left
/// a recorded result under the plan's own fingerprint.
#[test]
fn the_landing_fronts_write_through_the_engine() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "codebase",
            "--apply",
        ])
        .assert()
        .success();

    let state = std::fs::read_dir(fixture.state_root().join("spec-driven-docs/plans/results"))
        .or_else(|_| std::fs::read_dir(fixture.state_root().join("spec-driven-docs/results")))
        .expect("the front recorded no result, so it wrote outside the engine");
    let results: Vec<_> = state.filter_map(Result::ok).collect();
    assert_eq!(
        results.len(),
        1,
        "one landing left {} recorded results",
        results.len()
    );
    let fingerprint = results[0].file_name().to_str().unwrap().to_string();
    assert_eq!(fingerprint.len(), 64, "the result is not under a plan id");

    let attempts: Vec<_> = std::fs::read_dir(results[0].path())
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    let result: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(attempts[0].path().join("result.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(result["schema"], "sdd.result/1");
    assert_eq!(result["disposition"], "succeeded");
    assert!(
        result["postconditions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|held| held["held"] == true),
        "a landing reported an unproven postcondition: {result}"
    );
}

/// VERIFIES reconcile:a-finding-is-something-the-program-proved
///
/// A budget finding is a measurement, and it is the same measurement
/// `sdd debt` records. A corpus over a cap gets a finding and, where the
/// operator asks for it, one debt operation carrying the ceiling.
#[test]
fn an_oversized_document_is_a_budget_finding_and_an_operator_can_record_it() {
    let fixture = Fixture::new();
    let long: String = std::iter::repeat_n("A sentence here.\n\n", 300).collect();
    fixture.write("docs/specs/SPEC-held.md", "# Held\n\nRules.\n");
    fixture.write(
        "docs/decisions/ADR-too-long.md",
        &format!("# Too long\n\n{long}"),
    );

    let settled = [
        "--set",
        "profile=codebase",
        "--set",
        "plan-zone=none",
        "--set",
        "docs-scratch=none",
        "--set",
        "writing-style=builtin",
        "--set",
        "migration-scope=sweep",
    ];
    let held = plan(&fixture, &settled);
    let budget: Vec<&serde_json::Value> = held["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["kind"] == "budget")
        .collect();
    assert!(
        !budget.is_empty(),
        "no budget finding: {}",
        held["findings"]
    );
    assert!(budget[0]["measurement"]["found"].as_u64().unwrap() > 0);
    assert!(
        held["operations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|operation| operation["kind"] != "write-debt"),
        "a ceiling was recorded without the operator asking"
    );

    let mut answered: Vec<&str> = settled.to_vec();
    answered.extend(["--set", "debt-baseline=record"]);
    let held = plan(&fixture, &answered);
    assert!(
        held["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|operation| operation["kind"] == "write-debt"),
        "the selected baseline produced no debt operation"
    );
}
