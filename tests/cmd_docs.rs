//! Integration: the documentation index and the topic resolver.

// Integration tests: assertion style is the point, so the production
// restrictions on unwrap/panic and string building do not apply here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "the panic is this suite's failure signal, not control flow"
)]

mod support;

use predicates::prelude::*;
use support::Fixture;

#[test]
fn docs_is_registered_in_the_parser_and_the_dispatch() {
    let fixture = Fixture::new();
    fixture.cmd().args(["docs", "--help"]).assert().success();
}

#[test]
fn the_index_is_the_default_and_takes_no_index_flag() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .arg("docs")
        .assert()
        .success()
        .stdout(predicate::str::contains("sdd docs <topic>"))
        .stdout(predicate::str::contains("Method chapters"))
        .stdout(predicate::str::contains("Operator tasks"));
    fixture.cmd().args(["docs", "--index"]).assert().failure();
}

#[test]
fn docs_json_declares_the_sdd_docs_schema_one() {
    let fixture = Fixture::new();
    let out = fixture.cmd().args(["docs", "--json"]).output().unwrap();
    assert!(out.status.success());
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["schema"], "sdd.docs/1");
    let topics = report["topics"].as_array().unwrap();
    assert!(
        topics.len() > 40,
        "the catalog reports {} topics",
        topics.len()
    );
    let chapter = topics
        .iter()
        .find(|topic| topic["id"] == "agent-context")
        .expect("the catalog carries the agent-context chapter");
    assert!(chapter["bytes"].as_u64().unwrap() > 0);
    assert_eq!(chapter["target"]["document"]["shelf"], "method");
}

#[test]
fn a_declared_token_estimate_is_passed_through_and_never_invented() {
    let fixture = Fixture::new();
    let out = fixture.cmd().args(["docs", "--json"]).output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let topics = report["topics"].as_array().unwrap();
    let routing = topics
        .iter()
        .find(|topic| topic["id"] == "method-routing")
        .unwrap();
    assert!(
        routing["token_estimate"].as_u64().unwrap() > 0,
        "the digest declares a token estimate that the report drops"
    );
    let glossary = topics
        .iter()
        .find(|topic| topic["id"] == "glossary")
        .unwrap();
    assert!(
        glossary.get("token_estimate").is_none(),
        "a document that declares no estimate reports one anyway"
    );
}

#[test]
fn a_multiword_alias_resolves_from_several_argv_values_and_from_one_quoted_value() {
    let fixture = Fixture::new();
    for query in [vec!["context", "budget"], vec!["context budget"]] {
        let mut command = fixture.cmd();
        command.arg("docs");
        command.args(&query);
        command
            .assert()
            .success()
            .stdout(predicate::str::contains("# Agent Context"));
    }
}

#[test]
fn a_command_target_prints_the_argv_and_runs_nothing() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["docs", "upgrade"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run: sdd reconcile plan --help"))
        .stdout(predicate::str::contains("Usage:").not());
}

#[test]
fn an_ambiguous_query_names_every_candidate_and_guesses_none() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["docs", "template-"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("template-adr"))
        .stderr(predicate::str::contains("name one"));
}

#[test]
fn a_missing_topic_names_the_nearest_ids() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["docs", "quantum-tunnelling"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("nearest:"));
}

#[test]
fn every_list_renders_its_summary_from_the_catalog() {
    let fixture = Fixture::new();
    for (shelf, name, summary) in [
        ("method", "glossary", "resolved at the chapter that owns"),
        ("spec", "distribution", "The ownership classes"),
        ("template", "adr", "The sections a decision record carries"),
    ] {
        fixture
            .cmd()
            .args([shelf, "--list"])
            .assert()
            .success()
            .stdout(predicate::str::contains(name))
            .stdout(predicate::str::contains(summary));
    }
}
