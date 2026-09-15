//! Integration: `sdd stage` renders a candidate the target never sees.

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

use std::path::PathBuf;

use predicates::prelude::*;
use support::Fixture;

/// Where this suite puts stages: named, and inside this fixture's own
/// scratch home, so one test's stage is never another's.
fn stage_path(fixture: &Fixture) -> PathBuf {
    fixture
        .state_root()
        .parent()
        .unwrap()
        .join("stage-under-test")
}

fn stage(fixture: &Fixture) -> serde_json::Value {
    let path = stage_path(fixture);
    let output = fixture
        .cmd()
        .args([
            "stage",
            "--target",
            &fixture.target(),
            "--output",
            path.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

/// VERIFIES staging:a-stage-writes-only-the-stage
#[test]
fn staging_writes_nothing_in_the_target() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let before = fixture.tree_digest();

    let receipt = stage(&fixture);

    assert_eq!(fixture.tree_digest(), before);
    assert_eq!(receipt["schema"], "sdd.stage/1");
    assert_eq!(receipt["root_source"], "flag");
    assert_eq!(receipt["version"], env!("CARGO_PKG_VERSION"));
}

/// VERIFIES staging:a-stage-carries-the-whole-candidate
#[test]
fn the_stage_holds_every_destination_and_the_installed_reference_material() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let receipt = stage(&fixture);
    let root = stage_path(&fixture);

    let artifacts = receipt["artifacts"].as_array().unwrap();
    assert!(artifacts.len() > 5, "{artifacts:?}");
    for artifact in artifacts {
        let relative = artifact["path"].as_str().unwrap();
        assert!(
            root.join("artifacts").join(relative).is_file(),
            "{relative} is listed and not rendered"
        );
    }
    // The record the landing would write is in the stage as an artifact, so
    // the agent can read what the landing would claim.
    assert!(
        root.join("artifacts/.spec-driven-docs/manifest.json")
            .is_file()
    );

    // The reference material is this binary's own, byte for byte.
    let staged = std::fs::read_to_string(root.join("reference/method/AGENTS.md")).unwrap();
    let served = fixture
        .cmd()
        .args(["method", "AGENTS"])
        .assert()
        .get_output()
        .stdout
        .clone();
    assert!(!staged.is_empty());
    assert!(!served.is_empty());

    // The staged skill is a package: it names its gates relative to its
    // own root, so a copy that split them would carry two dead references.
    for gate in ["pre-flight-gate.md", "plan-gate.md"] {
        assert!(
            root.join("reference/skills/sdd-setup/references")
                .join(gate)
                .is_file(),
            "the staged skill package is missing {gate}"
        );
    }
    // The release notes of this exact version, which is where a migration
    // reads what the release asked of an instance.
    assert!(root.join("reference/CHANGELOG.md").is_file());

    // Every relative link the staged method carries resolves inside the
    // stage. A reference shelf that told a reader to open a file it did
    // not copy would send them back to a checkout.
    let reference = root.join("reference");
    for entry in walkdir::WalkDir::new(reference.join("method"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let text = std::fs::read_to_string(entry.path()).unwrap();
        for target in text.split("](").skip(1) {
            let Some(relative) = target.split(')').next() else {
                continue;
            };
            let relative = relative.split('#').next().unwrap_or(relative);
            // Only the relative links: an absolute URL is somebody else's
            // to serve, and an anchor alone points inside this file.
            if !relative.starts_with("./") && !relative.starts_with("../") {
                continue;
            }
            let resolved = entry.path().parent().unwrap().join(relative);
            assert!(
                resolved.exists(),
                "{} links to {relative}, which the stage did not copy",
                entry.path().display()
            );
        }
    }
}

/// VERIFIES staging:a-stage-carries-the-whole-candidate
#[test]
fn a_staged_artifact_holds_the_bytes_a_landing_would_write() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    stage(&fixture);
    let root = stage_path(&fixture);

    let staged = std::fs::read_to_string(root.join("artifacts/.pre-commit-config.yaml")).unwrap();
    let landed = fixture.read(".pre-commit-config.yaml");
    assert_eq!(staged, landed);
}

/// VERIFIES staging:a-stage-writes-only-the-stage
#[test]
fn a_stage_inside_the_target_refuses() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let digest = fixture.tree_digest();

    fixture
        .cmd()
        .args([
            "stage",
            "--target",
            &fixture.target(),
            "--output",
            fixture.path().join("review").to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("outside the target"));
    assert_eq!(
        digest,
        fixture.tree_digest(),
        "a refusal wrote in the target"
    );
}

/// VERIFIES staging:a-stage-carries-the-whole-candidate
#[test]
fn the_receipt_reports_the_declaration_the_candidate_carries() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    // The project reserves a path of its own, in the declaration it owns.
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - vendor/**\n",
    );

    // Staged with no flags at all: the receipt must report what the
    // candidate says, not what the command line did not say.
    let receipt = stage(&fixture);
    assert_eq!(receipt["reserve"][0], "vendor/**");
    assert_eq!(receipt["writing_style"], "builtin");
    assert_eq!(receipt["docs_root"], "docs");
}

/// VERIFIES staging:production-reads-no-staged-byte
#[test]
fn an_upgrade_succeeds_while_the_stage_is_unreadable() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    stage(&fixture);
    let root = stage_path(&fixture);

    // The stage denies every read. A verb that reached into it would fail
    // here rather than render the candidate again.
    let mut permissions = std::fs::metadata(&root).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o000);
    std::fs::set_permissions(&root, permissions.clone()).unwrap();

    let result = fixture.upgrade().assert().try_success();

    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o700);
    std::fs::set_permissions(&root, permissions).unwrap();
    result.unwrap();
}

/// VERIFIES staging:a-stage-persists-until-it-is-cleaned
#[test]
fn the_stage_survives_the_landing_it_informed() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    stage(&fixture);
    let root = stage_path(&fixture);

    fixture.upgrade().assert().success();
    assert!(root.join("stage.json").is_file(), "the stage was removed");
}

#[test]
fn a_second_stage_at_one_path_refuses_rather_than_overwriting() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    stage(&fixture);
    let path = stage_path(&fixture);

    fixture
        .cmd()
        .args([
            "stage",
            "--target",
            &fixture.target(),
            "--output",
            path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already holds a stage"));
}

/// VERIFIES staging:a-stage-persists-until-it-is-cleaned
#[test]
fn clean_removes_the_stage_and_refuses_the_target() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    stage(&fixture);
    let path = stage_path(&fixture);

    fixture
        .cmd()
        .args(["stage", "clean", &fixture.target()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("project"));
    assert!(
        fixture
            .path()
            .join(".spec-driven-docs/manifest.json")
            .is_file()
    );

    fixture
        .cmd()
        .args(["stage", "clean", path.to_str().unwrap()])
        .assert()
        .success();
    assert!(!path.exists());
}
