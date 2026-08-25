//! Integration: the embedded readers and the license surface.

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
fn shelves_list_and_serve_their_documents() {
    let fixture = Fixture::new();
    for (command, listed, name, expected) in [
        ("method", "glossary", "glossary", "canon"),
        (
            "spec",
            "distribution",
            "distribution",
            "distribution:instances-operate-offline",
        ),
        ("template", "adr", "adr", "Context and Problem Statement"),
    ] {
        fixture
            .cmd()
            .args([command, "--list"])
            .assert()
            .success()
            .stdout(predicate::str::contains(listed));
        fixture
            .cmd()
            .args([command, name])
            .assert()
            .success()
            .stdout(predicate::str::contains(expected));
    }
}

#[test]
fn an_unknown_document_is_a_usage_error() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["method", "no-such-chapter"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("no such document"));
}

#[test]
fn the_license_travels_with_the_binary() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["license"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MIT AND CC-BY-4.0"));
    fixture
        .cmd()
        .args(["license", "--method"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Creative Commons Attribution 4.0"));
    fixture
        .cmd()
        .args(["license", "--payload"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MIT License"));
}

#[test]
fn completions_and_man_render() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sdd"));
    fixture
        .cmd()
        .args(["man"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".TH"));
}
