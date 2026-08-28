//! Integration: `sdd verify` holds an instance to its record.

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
fn a_missing_manifest_exits_sixty_six() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(66)
        .stderr(predicate::str::contains("missing manifest"));
}

#[test]
fn a_corrupt_manifest_exits_sixty_five() {
    let fixture = Fixture::new();
    fixture.write(".spec-driven-docs/manifest.json", "{ not json");
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(65);
}

#[test]
fn a_version_one_manifest_points_at_upgrade() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let downgraded = fixture
        .read(".spec-driven-docs/manifest.json")
        .replace("\"schema_version\": 2", "\"schema_version\": 1");
    fixture.write(".spec-driven-docs/manifest.json", &downgraded);
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(65)
        .stderr(predicate::str::contains("run 'sdd upgrade'"));
}

#[test]
fn managed_drift_fails_red() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        ".spec-driven-docs/markdownlint/spec.markdownlint-cli2.jsonc",
        "{}\n",
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL managed drift: .spec-driven-docs/markdownlint/spec.markdownlint-cli2.jsonc",
        ));
}

#[test]
fn a_tampered_block_fails_red() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let config = fixture
        .read(".pre-commit-config.yaml")
        .replace("entry: sdd verify", "entry: true # neutered");
    fixture.write(".pre-commit-config.yaml", &config);
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL managed block tampered: .pre-commit-config.yaml",
        ));
}

#[test]
fn a_removed_block_fails_red() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let config: String = fixture
        .read(".pre-commit-config.yaml")
        .lines()
        .filter(|line| !line.starts_with("# BEGIN") && !line.starts_with("# END"))
        .map(|line| format!("{line}\n"))
        .collect();
    fixture.write(".pre-commit-config.yaml", &config);
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL missing managed pre-commit block",
        ));
}

#[test]
fn a_duplicate_local_rule_id_fails_red() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        "_docs/specs/SPEC-local.md",
        "# Local Specification\n\n### `instance:the-manifest-stays-readable` — Duplicated\n\nVerify: `true`\n",
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL duplicate rule ID in local specs",
        ))
        .stdout(predicate::str::contains(
            "### `instance:the-manifest-stays-readable`",
        ));
}

#[test]
fn an_instance_ahead_of_the_binary_fails_red() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let manifest = fixture
        .read(".spec-driven-docs/manifest.json")
        .replace(env!("CARGO_PKG_VERSION"), "9.9.9");
    fixture.write(".spec-driven-docs/manifest.json", &manifest);
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("upgrade sdd"));
}

#[test]
fn a_symlinked_managed_file_fails_red() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let outside = tempfile::tempdir().unwrap();
    let managed = ".spec-driven-docs/markdownlint/spec.markdownlint-cli2.jsonc";
    std::fs::copy(
        fixture.path().join(managed),
        outside.path().join("spec.jsonc"),
    )
    .unwrap();
    std::fs::remove_file(fixture.path().join(managed)).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("spec.jsonc"),
        fixture.path().join(managed),
    )
    .unwrap();
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL managed file reached through a symlink",
        ));
}

#[test]
fn a_manifest_omitting_a_declared_projection_fails_red() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    let managed = manifest["managed_files"].as_array_mut().unwrap();
    managed.retain(|entry| {
        !entry["destination"]
            .as_str()
            .unwrap()
            .contains("spec.markdownlint")
    });
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &(serde_json::to_string_pretty(&manifest).unwrap() + "\n"),
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL manifest omits a declared projection: .spec-driven-docs/markdownlint/spec.markdownlint-cli2.jsonc",
        ));
}

#[test]
fn a_symlinked_manifest_is_refused() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let outside = tempfile::tempdir().unwrap();
    let manifest = ".spec-driven-docs/manifest.json";
    std::fs::copy(
        fixture.path().join(manifest),
        outside.path().join("manifest.json"),
    )
    .unwrap();
    std::fs::remove_file(fixture.path().join(manifest)).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("manifest.json"),
        fixture.path().join(manifest),
    )
    .unwrap();
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(65)
        .stderr(predicate::str::contains(
            "manifest reached through a symlink",
        ));
}

#[test]
fn a_canon_looking_cargo_toml_does_not_exempt_projection_completeness() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        "Cargo.toml",
        "# name = \"spec-driven-docs\"\n[package]\nname = \"else\"\n",
    );
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    let managed = manifest["managed_files"].as_array_mut().unwrap();
    managed.retain(|entry| {
        !entry["destination"]
            .as_str()
            .unwrap()
            .contains("spec.markdownlint")
    });
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &(serde_json::to_string_pretty(&manifest).unwrap() + "\n"),
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL manifest omits a declared projection",
        ));
}

#[test]
fn forging_the_self_manifest_layout_does_not_shrink_the_held_set() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    let managed = manifest["managed_files"].as_array_mut().unwrap();
    managed.retain(|entry| {
        !entry["destination"]
            .as_str()
            .unwrap()
            .contains("spec.markdownlint")
    });
    for entry in managed.iter_mut() {
        entry["destination"] = entry["source"].clone();
    }
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &(serde_json::to_string_pretty(&manifest).unwrap() + "\n"),
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL manifest omits a declared projection",
        ));
}
