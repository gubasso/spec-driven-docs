//! Integration: `sdd self-manifest` regenerates the canon's record and
//! refuses anywhere else.

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

#[test]
fn regenerates_a_schema_two_manifest_in_a_canon_shaped_checkout() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"spec-driven-docs\"\nversion = \"0.2.0\"\n",
    );
    fixture.write(".markdownlint/spec.markdownlint-cli2.jsonc", "{}\n");
    fixture.write("skill-shared/plan-gate.md", "# The plan gate\n");
    fixture.write("skill-shared/pre-flight-gate.md", "# The pre-flight gate\n");
    for path in spec_driven_docs::domain::profile::SIMPLE_ENGLISH_MANAGED {
        fixture.write(path, "vendored\n");
    }
    fixture.write(
        "templates/TEMPLATE-tracking.yaml",
        "schema_version: 1\ntracked: []\n",
    );
    fixture.write(
        "_docs/reference/tracking.yaml",
        "schema_version: 1\ntracked: []\n",
    );
    fixture.write(
        "_docs/specs/SPEC-sample.md",
        "### `sample:works` — Works\n\nVerify: `true`\n",
    );
    fixture.write("_docs/decisions/TEMPLATE-adr.md", "# Template\n");
    fixture.write("_docs/reference/TEMPLATE-agents-digest.md", "# Template\n");
    fixture.write(
        ".pre-commit-config.yaml",
        "repos:\n# BEGIN spec-driven-docs managed\n  - repo: local\n# END spec-driven-docs managed\n",
    );
    fixture.write(
        ".spec-driven-docs/manifest.json",
        "{\n  \"installed_at\": \"2026-01-01T00:00:00Z\"\n}\n",
    );

    fixture
        .cmd()
        .args(["self-manifest"])
        .current_dir(fixture.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "OK regenerated .spec-driven-docs/manifest.json",
        ));

    let manifest = fixture.read(".spec-driven-docs/manifest.json");
    assert!(manifest.contains("\"schema_version\": 2"));
    assert!(manifest.contains("\"installed_at\": \"2026-01-01T00:00:00Z\""));
    assert!(manifest.contains("_docs/specs/SPEC-sample.md"));
    assert!(manifest.contains(".markdownlint/spec.markdownlint-cli2.jsonc"));
    assert!(manifest.contains("skill-shared/plan-gate.md"));
    assert!(manifest.contains("skill-shared/pre-flight-gate.md"));
    assert!(manifest.contains("third-party/simpleenglish/skills/simple-english/SKILL.md"));
    assert!(manifest.contains("_docs/reference/tracking.yaml"));

    let repeat = fixture.read(".spec-driven-docs/manifest.json");
    fixture
        .cmd()
        .args(["self-manifest"])
        .current_dir(fixture.path())
        .assert()
        .success();
    assert_eq!(
        repeat,
        fixture.read(".spec-driven-docs/manifest.json"),
        "not stable on a second run"
    );
}
