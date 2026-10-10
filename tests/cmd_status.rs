//! Integration: `sdd status` reports an instance's state without gating.

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
    fixture.write("_docs/.markdownlint-cli2.jsonc", "// edited\n");
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

#[test]
fn status_json_declares_sdd_status_schema_four() {
    let fixture = Fixture::new();
    assert_eq!(status_json(&fixture)["schema"], "sdd.status/5");
}

/// Rewrite the recorded canon version, leaving every other byte.
fn record_version(fixture: &Fixture, version: &str) {
    let path = ".spec-driven-docs/manifest.json";
    let mut record: serde_json::Value = serde_json::from_str(&fixture.read(path)).unwrap();
    record["canon_version"] = serde_json::Value::String(version.to_string());
    fixture.write(path, &serde_json::to_string_pretty(&record).unwrap());
}

/// VERIFIES distribution:a-landing-classifies-its-target-first
#[test]
fn status_reports_each_of_the_seven_classifications() {
    let classification = |fixture: &Fixture| status_json(fixture)["classification"].clone();

    let setup = Fixture::new();
    setup.write("README.md", "# Project\n");
    assert_eq!(classification(&setup), "setup");

    let migration = Fixture::new();
    migration.write("docs/architecture.md", "# Architecture\n");
    assert_eq!(classification(&migration), "migration");

    let scattered = Fixture::new();
    scattered.write("notes/design.md", "# Design\n");
    assert_eq!(classification(&scattered), "needs-decision");

    // Notes in the docs scratch are nobody's corpus, so the target is a
    // first landing and a landing proceeds.
    let scratch = Fixture::new();
    scratch.write(".docs-scratch/notes.md", "# Notes\n");
    assert_eq!(classification(&scratch), "setup");
    scratch.install("knowledge-base");

    let upgrade = Fixture::new();
    upgrade.install("knowledge-base");
    record_version(&upgrade, "0.1.0");
    assert_eq!(classification(&upgrade), "upgrade");

    let drift = Fixture::new();
    drift.install("knowledge-base");
    drift.write("_docs/.markdownlint-cli2.jsonc", "// edited\n");
    assert_eq!(classification(&drift), "drift");

    // An older binary cannot judge a newer record, so it names no class.
    let newer = Fixture::new();
    newer.install("knowledge-base");
    record_version(&newer, "99.0.0");
    let report = status_json(&newer);
    assert_eq!(report["alignment"], "instance-newer");
    assert_eq!(report["classification"], serde_json::Value::Null);

    let current = Fixture::new();
    current.install("knowledge-base");
    let report = status_json(&current);
    assert_eq!(report["classification"], "current");
    assert_eq!(report["invalid"], serde_json::Value::Null);

    let invalid = Fixture::new();
    invalid.install("knowledge-base");
    invalid.write(".spec-driven-docs/config.yaml", "reserved: [\n");
    let report = status_json(&invalid);
    assert_eq!(report["classification"], "invalid");
    assert!(
        report["invalid"].as_str().unwrap().contains("config.yaml"),
        "{report}"
    );
}

/// A record nobody can parse is a diagnostic, never a report, so a skill
/// stops before it reads a classification at all.
#[test]
fn a_corrupt_manifest_is_a_diagnostic_and_carries_no_classification() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(".spec-driven-docs/manifest.json", "{\"schema_version\":");
    fixture
        .cmd()
        .args(["status", "--target", &fixture.target(), "--json"])
        .assert()
        .code(65)
        .stdout(predicate::str::is_empty());
}

/// VERIFIES setup:status-reports-what-an-older-release-left
#[test]
fn status_names_the_state_an_older_release_left_and_reads_none_of_it() {
    let fixture = Fixture::new();
    let store = fixture
        .state_root()
        .join("spec-driven-docs/plans/plans/abc");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("plan.json"), "{}").unwrap();
    let report = status_json(&fixture);
    let legacy = report["legacy_state"].as_array().unwrap();
    assert_eq!(legacy.len(), 1, "{legacy:?}");
    assert_eq!(legacy[0]["kind"], "plan-store");
    assert_eq!(legacy[0]["active"], true, "an unfinished plan still binds");
    assert!(
        legacy[0]["path"]
            .as_str()
            .unwrap()
            .ends_with("spec-driven-docs/plans"),
        "{legacy:?}"
    );
    assert_eq!(
        std::fs::read_to_string(store.join("plan.json")).unwrap(),
        "{}"
    );

    let clean = Fixture::new();
    assert_eq!(status_json(&clean)["legacy_state"], serde_json::json!([]));
}

#[test]
fn status_reports_user_scope_paths_without_an_instance() {
    let home = tempfile::tempdir().unwrap();
    let fixture = Fixture::new();
    let output = fixture
        .cmd()
        // This case is about the defaults, so it takes the two variables
        // the fixture sets back out again and lets HOME decide.
        .env_remove("XDG_STATE_HOME")
        .env_remove("XDG_CACHE_HOME")
        .env("HOME", home.path())
        .args(["status", "--target", &fixture.target(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let user = &report["paths"]["user"];
    let home = home.path().to_str().unwrap();
    assert_eq!(
        user["state_root"]["path"],
        format!("{home}/.local/state/spec-driven-docs")
    );
    assert_eq!(user["state_root"]["source"], "default");
    assert_eq!(
        user["cache_root"]["path"],
        format!("{home}/.cache/spec-driven-docs")
    );
    assert_eq!(
        user["skill_receipt"]["path"],
        format!("{home}/.local/state/spec-driven-docs/skills.json")
    );
    let roots = user["agent_roots"].as_array().unwrap();
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0]["id"], "claude");
    assert_eq!(roots[0]["path"], format!("{home}/.claude/skills"));
    assert_eq!(roots[0]["variable"], serde_json::Value::Null);
    assert_eq!(roots[1]["id"], "agents");
    assert_eq!(roots[1]["path"], format!("{home}/.agents/skills"));
}

#[test]
fn status_reports_null_active_and_both_candidate_sets_without_an_instance() {
    let fixture = Fixture::new();
    let report = status_json(&fixture);
    assert_eq!(report["paths"]["active"], serde_json::Value::Null);
    let candidates = &report["paths"]["candidates"];
    assert_eq!(
        candidates["codebase"]["destinations"]["specs"]["path"],
        "docs/specs"
    );
    assert_eq!(
        candidates["knowledge-base"]["destinations"]["specs"]["path"],
        "_docs/specs"
    );
    assert_eq!(
        candidates["codebase"]["destinations"]["specs"]["source"],
        "profile"
    );
}

#[test]
fn status_reports_the_paths_with_their_sources() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let report = status_json(&fixture);
    let active = &report["paths"]["active"];
    assert_eq!(active["profile"], "knowledge-base");
    let held = &active["destinations"];
    assert_eq!(held["declaration"]["path"], ".spec-driven-docs/config.yaml");
    assert_eq!(held["declaration"]["source"], "default");
    assert_eq!(held["hooks_config"]["path"], ".pre-commit-config.yaml");
    assert_eq!(held["agents_digest"]["path"], "AGENTS.md");
    assert_eq!(held["docs_root"]["path"], "_docs");
    assert_eq!(held["docs_root"]["source"], "recorded");
    assert_eq!(held["guides"]["path"], "_docs/guides");
}

#[test]
fn status_derives_project_location_proposals_from_the_target() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let bare = status_json(&fixture);
    let kinds: Vec<&str> = bare["paths"]["proposals"]["docs_scratch"]
        .as_array()
        .unwrap()
        .iter()
        .map(|choice| choice["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["env", "operator", "none"]);
    assert!(bare["paths"]["proposals"].get("plan_zone").is_none());

    std::fs::create_dir_all(fixture.path().join(".docs-scratch")).unwrap();
    let observed = status_json(&fixture);
    let first = &observed["paths"]["proposals"]["docs_scratch"][0];
    assert_eq!(first["kind"], "observed");
    assert_eq!(first["path"], ".docs-scratch");
}

#[test]
fn a_recorded_location_reports_as_recorded_and_an_override_as_env() {
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
            "--docs-scratch",
            "staging",
        ])
        .assert()
        .success();
    let recorded = status_json(&fixture);
    assert_eq!(
        recorded["paths"]["active"]["docs_scratch"],
        serde_json::json!({"kind": "untracked", "path": "staging", "source": "recorded"})
    );
    assert!(recorded["paths"]["active"].get("plan_zone").is_none());
    assert!(recorded.get("plan_zone").is_none());
    assert!(recorded.get("plan_zone_env").is_none());

    let output = fixture
        .cmd()
        .env("SDD_DOCS_SCRATCH", "elsewhere")
        .args(["status", "--target", &fixture.target(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let overridden: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        overridden["paths"]["active"]["docs_scratch"],
        serde_json::json!({
            "kind": "env",
            "variable": "SDD_DOCS_SCRATCH",
            "value": "elsewhere"
        })
    );
}

#[test]
fn the_text_form_prints_the_paths_under_one_heading() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    fixture
        .cmd()
        .args(["status", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("paths:"))
        .stdout(predicate::str::contains("agent root claude:"))
        .stdout(predicate::str::contains(
            "  documentation root: docs (recorded)",
        ));
}
