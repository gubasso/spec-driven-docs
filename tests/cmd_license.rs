//! Integration: `sdd license` prints the terms the binary carries.

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
fn third_party_prints_the_notice_offline() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["license", "--third-party"])
        .env("HOME", "/nonexistent")
        .assert()
        .success()
        .stdout(predicate::str::contains("SimpleEnglish"))
        .stdout(predicate::str::contains(
            "d9e523409686e88df175623f7a692d025aff95b1",
        ))
        .stdout(predicate::str::contains("MIT License"))
        .stdout(predicate::str::contains("ASD-STE100"));
}

#[test]
fn the_bare_license_still_names_both_halves() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["license"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MIT"))
        .stdout(predicate::str::contains("Creative Commons Attribution"));
}
