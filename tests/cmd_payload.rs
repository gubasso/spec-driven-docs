//! Integration: `sdd payload` reports what a release carries.
//!
//! The registry-reading cases are behind `SDD_TEST_NETWORK`, because a
//! suite that reaches crates.io is red wherever the network is absent and
//! slow wherever it is not. The offline and refusal cases run always: they
//! are what an operator meets first.

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
use support::{Fixture, Home};

/// Whether this run may reach the registry.
fn networked() -> bool {
    std::env::var("SDD_TEST_NETWORK").is_ok_and(|value| !value.is_empty())
}

#[test]
fn the_embedded_release_reports_itself_and_touches_no_network() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["payload"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "spec-driven-docs {}",
            env!("CARGO_PKG_VERSION")
        )))
        .stdout(predicate::str::contains("payload schema: 1"))
        .stdout(predicate::str::contains("provenance: native"));
}

#[test]
fn the_json_form_declares_its_own_machine_schema() {
    let fixture = Fixture::new();
    let out = fixture
        .cmd()
        .args(["payload", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let manifest: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(manifest["schema"], "sdd.payload/1");
    assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(manifest["payload_schema"], 1);
    assert_eq!(manifest["provenance"], "native");
    assert_eq!(manifest["descriptor_sha256"], serde_json::Value::Null);
    let artifacts = manifest["artifacts"].as_array().unwrap();
    assert!(
        artifacts.len() > 20,
        "the manifest lists {} artifacts",
        artifacts.len()
    );
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact["path"] == "instance/projection.toml"),
        "the manifest omits the declaration"
    );
    for artifact in artifacts {
        assert_eq!(artifact["role"], "payload");
        assert_eq!(artifact["sha256"].as_str().unwrap().len(), 64);
    }
}

#[test]
fn a_version_that_is_not_semantic_is_a_usage_error() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["payload", "--release", "not-a-version"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains(
            "embedded, latest, or a semantic version",
        ));
}

/// VERIFIES bundle:resolution-happens-once-and-writes-only-the-cache
#[test]
fn offline_latest_refuses_with_its_reason() {
    let home = Home::new();
    home.cmd()
        .args(["payload", "--release", "latest", "--offline"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("cannot resolve latest"));
}

/// VERIFIES bundle:resolution-happens-once-and-writes-only-the-cache
#[test]
fn offline_refuses_an_exact_release_the_cache_does_not_hold() {
    let home = Home::new();
    home.cmd()
        .args(["payload", "--release", "0.8.0", "--offline"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("--offline forbids"));
}

/// VERIFIES bundle:a-pre-schema-release-is-cataloged-or-unavailable
///
/// The refusal comes from the catalog, so it needs no network: a release
/// the audit already classified unavailable is refused before a fetch.
#[test]
fn a_release_below_the_capability_floor_is_unavailable_with_its_evidence() {
    let home = Home::new();
    home.cmd()
        .args(["payload", "--release", "0.6.5"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("unavailable as a destination"))
        .stderr(predicate::str::contains("instance/seeds"))
        .stderr(predicate::str::contains("0.6.6"));
}

/// VERIFIES bundle:a-pre-schema-release-is-cataloged-or-unavailable
#[test]
fn payload_release_returns_another_releases_manifest() {
    if !networked() {
        return;
    }
    let home = Home::new();
    let out = home
        .cmd()
        .env_remove("SDD_OFFLINE")
        .args(["payload", "--release", "0.8.0", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let manifest: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(manifest["version"], "0.8.0");
    assert_eq!(manifest["provenance"], "legacy-adapted");
    assert_eq!(manifest["payload_schema"], 0);
    assert!(manifest["descriptor_sha256"].is_string());
    assert!(
        manifest["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact["role"] == "metadata"),
        "the adapted manifest carries no virtual metadata"
    );

    // A second exact call reads the cache and touches no network.
    home.cmd()
        .args(["payload", "--release", "0.8.0", "--offline"])
        .assert()
        .success()
        .stdout(predicate::str::contains("spec-driven-docs 0.8.0"));
}
