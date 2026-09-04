//! Integration: `sdd ki list` derives the known-issue index from the records.

// Integration tests: assertion style is the point, so the production
// restrictions on unwrap/panic and string building do not apply here.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use predicates::prelude::*;
use support::Fixture;

fn seed(fixture: &Fixture, name: &str, record: &str) {
    let zone = fixture.path().join("_docs/reference/known-issues");
    std::fs::create_dir_all(&zone).unwrap();
    std::fs::write(zone.join(name), record).unwrap();
}

const MASKED: &str = "---\nupstream: https://example.invalid/issues/1234\nstate: masked\nfiling: filed\nretire_when: release >= 2.4.0\n---\n# Vendor issue\n";
const OPEN: &str = "---\nupstream: https://example.invalid/issues\nstate: investigating\nfiling: gathering\n---\n# Other issue\n";

#[test]
fn a_target_with_no_zone_lists_nothing_and_exits_zero() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["ki", "list", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn the_listing_names_each_case_with_both_axes() {
    let fixture = Fixture::new();
    seed(&fixture, "KI-vendor-replays.md", MASKED);
    seed(&fixture, "KI-other.md", OPEN);
    fixture
        .cmd()
        .args(["ki", "list", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "KI-vendor-replays  masked         filed      https://example.invalid/issues/1234",
        ))
        .stdout(predicate::str::contains(
            "KI-other           investigating  gathering  https://example.invalid/issues",
        ));
}

#[test]
fn the_json_listing_carries_one_object_per_record() {
    let fixture = Fixture::new();
    seed(&fixture, "KI-vendor-replays.md", MASKED);
    let output = fixture
        .cmd()
        .args(["ki", "list", "--target", &fixture.target(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let cases: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(cases.as_array().unwrap().len(), 1);
    assert_eq!(cases[0]["id"], "KI-vendor-replays");
    assert_eq!(cases[0]["state"], "masked");
    assert_eq!(cases[0]["filing"], "filed");
    assert_eq!(
        cases[0]["path"],
        "_docs/reference/known-issues/KI-vendor-replays.md"
    );
}

#[test]
fn a_record_stating_no_axis_is_listed_with_the_value_missing() {
    let fixture = Fixture::new();
    seed(
        &fixture,
        "KI-bare.md",
        "---\naffects: client\n---\n# Bare\n",
    );
    fixture
        .cmd()
        .args(["ki", "list", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("KI-bare  -"));
}

#[test]
fn a_target_that_is_not_a_directory_is_reported_rather_than_listed_empty() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args([
            "ki",
            "list",
            "--target",
            &format!("{}/nowhere", fixture.target()),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no directory at"));
}

#[test]
fn a_relative_target_that_is_not_the_working_directory_is_a_usage_error() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["ki", "list", "--target", "somewhere"])
        .assert()
        .failure();
}
