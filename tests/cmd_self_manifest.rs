//! Integration: `sdd self-manifest` writes the payload record of a canon
//! checkout and refuses anywhere else.

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
fn refuses_outside_the_canon_checkout() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["self-manifest"])
        .current_dir(fixture.path())
        .assert()
        .code(73)
        .stderr(predicate::str::contains("not the canon checkout"));
}

/// VERIFIES release:the-payload-record-describes-its-tree
///
/// The record carries the checkout's own crate version, whatever the binary
/// that writes it, hashes each file under the payload roots, and leaves the
/// instance record alone.
#[test]
fn writes_the_payload_record_in_a_canon_shaped_checkout() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"spec-driven-docs\"\nversion = \"0.2.0\"\n",
    );
    fixture.write("instance/payload/_docs/.markdownlint-cli2.jsonc", "{}\n");
    fixture.write("skill-shared/plan-gate.md", "# The plan gate\n");
    fixture.write("_docs/specs/SPEC-sample.md", "# Sample\n");
    fixture.write(
        ".spec-driven-docs/manifest.json",
        "{ \"untouched\": true }\n",
    );

    fixture
        .cmd()
        .args(["self-manifest"])
        .current_dir(fixture.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "OK regenerated instance/payload-manifest.json",
        ));

    let record: serde_json::Value =
        serde_json::from_str(&fixture.read("instance/payload-manifest.json")).unwrap();
    assert_eq!(record["schema"], "sdd.payload-manifest/1");
    assert_eq!(record["version"], env!("CARGO_PKG_VERSION"));
    let paths: Vec<&str> = record["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        paths,
        [
            "_docs/specs/SPEC-sample.md",
            "instance/payload/_docs/.markdownlint-cli2.jsonc",
            "skill-shared/plan-gate.md",
        ]
    );
    assert_eq!(
        fixture.read(".spec-driven-docs/manifest.json"),
        "{ \"untouched\": true }\n"
    );

    let first = fixture.read("instance/payload-manifest.json");
    fixture
        .cmd()
        .args(["self-manifest"])
        .current_dir(fixture.path())
        .assert()
        .success();
    assert_eq!(
        first,
        fixture.read("instance/payload-manifest.json"),
        "not stable on a second run"
    );
}
