//! Integration: `sdd upgrade` walks guides, refuses conflicts atomically,
//! reinstalls, and prunes what the new version stopped managing.

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

/// Rewrite a fresh instance into the version-one shape the previous
/// distribution produced: schema 1, canon 0.1.6, a vendored verifier and
/// one hook recorded as managed.
fn downgrade_to_v1(fixture: &Fixture) {
    fixture.write(".spec-driven-docs/verify.sh", "#!/bin/sh\nexit 0\n");
    fixture.write(
        ".spec-driven-docs/hooks/sample-gate.sh",
        "#!/bin/sh\nexit 0\n",
    );
    let manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    let sha = |relative: &str| {
        use sha2::Digest;
        hex::encode(sha2::Sha256::digest(fixture.read(relative).as_bytes()))
    };
    let mut managed = manifest["managed_files"].as_array().unwrap().clone();
    managed.push(serde_json::json!({
        "source": "scripts/verify.sh",
        "destination": ".spec-driven-docs/verify.sh",
        "sha256": sha(".spec-driven-docs/verify.sh"),
    }));
    managed.push(serde_json::json!({
        "source": "gates/instance/sample-gate.sh",
        "destination": ".spec-driven-docs/hooks/sample-gate.sh",
        "sha256": sha(".spec-driven-docs/hooks/sample-gate.sh"),
    }));
    let v1 = serde_json::json!({
        "schema_version": 1,
        "canon_version": "0.1.6",
        "canon_source": manifest["canon_source"],
        "canon_ref": "v0.1.6",
        "profile": manifest["profile"],
        "docs_root": manifest["docs_root"],
        "installed_at": manifest["installed_at"],
        "managed_files": managed,
        "adopted_files": manifest["adopted_files"],
        "integration_blocks": manifest["integration_blocks"],
    });
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &(serde_json::to_string_pretty(&v1).unwrap() + "\n"),
    );
}

fn v1_instance() -> Fixture {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    downgrade_to_v1(&fixture);
    fixture
}

#[test]
fn an_instance_at_the_current_version_is_already_done() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "OK already at {}",
            env!("CARGO_PKG_VERSION")
        )));
}

#[test]
fn dry_run_names_the_guides_and_changes_nothing() {
    let fixture = v1_instance();
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "consult migration: 0.1.6-to-0.2.0.md",
        ))
        .stdout(predicate::str::contains("DRY RUN upgrade 0.1.6 to"));
    assert_eq!(digest, fixture.tree_digest(), "dry run changed bytes");
}

#[test]
fn conflicts_are_collected_and_abort_atomically() {
    let fixture = v1_instance();
    fixture.write(".spec-driven-docs/verify.sh", "#!/bin/sh\nlocally edited\n");
    std::fs::remove_file(
        fixture
            .path()
            .join(".spec-driven-docs/hooks/sample-gate.sh"),
    )
    .unwrap();
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "CONFLICT locally edited managed file: .spec-driven-docs/verify.sh",
        ))
        .stdout(predicate::str::contains(
            "CONFLICT missing managed file: .spec-driven-docs/hooks/sample-gate.sh",
        ));
    assert_eq!(
        digest,
        fixture.tree_digest(),
        "a refused upgrade changed bytes"
    );
}

#[test]
fn an_upgrade_reinstalls_prunes_and_reports() {
    let fixture = v1_instance();
    let readonly_object = fixture.path().join(".git/objects/ab/cd");
    std::fs::create_dir_all(readonly_object.parent().unwrap()).unwrap();
    std::fs::write(&readonly_object, "loose object").unwrap();
    let mut permissions = std::fs::metadata(&readonly_object).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o444);
    std::fs::set_permissions(&readonly_object, permissions).unwrap();

    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "consult migration: 0.1.6-to-0.2.0.md",
        ))
        .stdout(predicate::str::contains(
            "removed managed file no longer owned: .spec-driven-docs/verify.sh",
        ))
        .stdout(predicate::str::contains(
            "removed managed file no longer owned: .spec-driven-docs/hooks/sample-gate.sh",
        ))
        .stdout(predicate::str::contains(format!(
            "OK upgraded 0.1.6 to {}",
            env!("CARGO_PKG_VERSION")
        )));

    assert!(!fixture.path().join(".spec-driven-docs/verify.sh").exists());
    assert!(
        !fixture
            .path()
            .join(".spec-driven-docs/hooks")
            .join("sample-gate.sh")
            .exists()
    );
    assert_eq!(fixture.read(".git/objects/ab/cd"), "loose object");
    let manifest = fixture.read(".spec-driven-docs/manifest.json");
    assert!(manifest.contains("\"schema_version\": 2"));
    let config = fixture.read(".pre-commit-config.yaml");
    assert!(config.contains("entry: sdd verify"));
    assert!(!config.contains("verify.sh"));
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
}

#[test]
fn a_symlinked_prune_destination_is_refused() {
    let fixture = v1_instance();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("verify.sh");
    std::fs::write(&outside_file, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::remove_file(fixture.path().join(".spec-driven-docs/verify.sh")).unwrap();
    std::os::unix::fs::symlink(
        &outside_file,
        fixture.path().join(".spec-driven-docs/verify.sh"),
    )
    .unwrap();

    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "refused to remove a destination reached through a symlink: .spec-driven-docs/verify.sh",
        ));
    assert!(
        outside_file.exists(),
        "the upgrade deleted through the symlink"
    );
}

#[test]
fn a_version_with_no_guide_is_refused() {
    let fixture = v1_instance();
    let manifest = fixture
        .read(".spec-driven-docs/manifest.json")
        .replace("0.1.6", "0.0.1");
    fixture.write(".spec-driven-docs/manifest.json", &manifest);
    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("no migration guide from 0.0.1"));
}

#[test]
fn show_guides_prints_the_guide_body() {
    let fixture = v1_instance();
    fixture
        .cmd()
        .args([
            "upgrade",
            "--target",
            &fixture.target(),
            "--dry-run",
            "--show-guides",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Migration from 0.1.6 to 0.2.0"));
}
