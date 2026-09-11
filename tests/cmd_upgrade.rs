//! Integration: `sdd upgrade` walks guides, refuses conflicts atomically,
//! reinstalls, and prunes what the new version stopped managing.

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
fn dry_run_reports_the_plan_and_changes_nothing() {
    let fixture = v1_instance();
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target(), "--dry-run"])
        .assert()
        .success()
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
    assert!(manifest.contains("\"schema_version\": 3"));
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
fn any_older_version_upgrades_mechanically() {
    let fixture = v1_instance();
    let manifest = fixture
        .read(".spec-driven-docs/manifest.json")
        .replace("0.1.6", "0.0.1");
    fixture.write(".spec-driven-docs/manifest.json", &manifest);
    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "OK upgraded 0.0.1 to {}",
            env!("CARGO_PKG_VERSION")
        )));
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
}

#[test]
fn a_stale_managed_skill_file_is_pruned() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(".claude/skills/old-skill/SKILL.md", "stale\n");
    let manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    let sha = {
        use sha2::Digest;
        hex::encode(sha2::Sha256::digest(
            fixture.read(".claude/skills/old-skill/SKILL.md").as_bytes(),
        ))
    };
    let mut managed = manifest["managed_files"].as_array().unwrap().clone();
    managed.push(serde_json::json!({
        "source": "skills/old-skill/SKILL.md",
        "destination": ".claude/skills/old-skill/SKILL.md",
        "sha256": sha,
    }));
    let mut older = manifest;
    older["managed_files"] = serde_json::Value::Array(managed);
    older["canon_version"] = serde_json::json!("0.1.6");
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &(serde_json::to_string_pretty(&older).unwrap() + "\n"),
    );
    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "removed managed file no longer owned: .claude/skills/old-skill/SKILL.md",
        ));
    assert!(
        !fixture
            .path()
            .join(".claude/skills/old-skill/SKILL.md")
            .exists()
    );
    // A skill is a directory holding one file, so pruning the file alone
    // would leave the old skill's name behind for an agent to offer.
    assert!(
        !fixture.path().join(".claude/skills/old-skill").exists(),
        "the pruned skill left its directory behind"
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
}

#[test]
fn a_locally_edited_agents_block_aborts_the_upgrade() {
    let fixture = v1_instance();
    // The v1 instance has no AGENTS block; a reinstall adds one. Give it a
    // current-shaped instance instead by reinstalling first, then editing.
    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .success();
    let agents = fixture.read("AGENTS.md").replace(
        "Run `sdd verify` before handoff.",
        "Run something else entirely.",
    );
    fixture.write("AGENTS.md", &agents);
    // Force a version behind so the upgrade runs its conflict scan again.
    let manifest = fixture
        .read(".spec-driven-docs/manifest.json")
        .replace(env!("CARGO_PKG_VERSION"), "0.0.1");
    fixture.write(".spec-driven-docs/manifest.json", &manifest);
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "CONFLICT locally edited managed block: AGENTS.md",
        ));
    assert_eq!(
        digest,
        fixture.tree_digest(),
        "a refused upgrade changed bytes"
    );
}

/// Rewrite a fresh instance into the version-two shape: schema 2, an older
/// canon, and neither declared location — version 2 had no field for either.
fn downgrade_to_v2(fixture: &Fixture) {
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    manifest["schema_version"] = 2.into();
    manifest["canon_version"] = "0.1.6".into();
    let object = manifest.as_object_mut().unwrap();
    object.remove("plan_zone");
    object.remove("docs_scratch");
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &(serde_json::to_string_pretty(&manifest).unwrap() + "\n"),
    );
}

/// A real version-2 record reaches version 3. It declared neither location,
/// so the only honest outcome is the undeclared state, and nothing else in
/// the record moves.
#[test]
fn a_version_two_upgrade_reaches_the_current_schema() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let before: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    downgrade_to_v2(&fixture);

    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "OK upgraded 0.1.6 to {}",
            env!("CARGO_PKG_VERSION")
        )));

    let after: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    assert_eq!(after["schema_version"], 3);
    assert_eq!(after["plan_zone"], serde_json::json!({"kind": "none"}));
    assert!(after.get("docs_scratch").is_none());
    assert_eq!(after["installed_at"], before["installed_at"]);
}

/// A declared location survives a reinstall it did not name, whatever schema
/// version the record carries. This is the read-through the upgrade depends
/// on: `sdd upgrade` reinstalls with no flag at all.
#[test]
fn a_reinstall_over_an_older_record_carries_the_declarations_forward() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "knowledge-base",
            "--apply",
            "--plan-zone",
            "docs/plan",
            "--docs-scratch",
            "staging",
        ])
        .assert()
        .success();
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    manifest["schema_version"] = 2.into();
    manifest["canon_version"] = "0.1.6".into();
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &(serde_json::to_string_pretty(&manifest).unwrap() + "\n"),
    );

    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .success();

    let after: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    assert_eq!(after["schema_version"], 3);
    assert_eq!(after["plan_zone"]["path"], "docs/plan");
    assert_eq!(after["docs_scratch"], "staging");
}

/// The two versions move independently, so an instance can carry this
/// binary's canon version and an older schema. Without a schema check the
/// "already at" shortcut returns before migrating, while every other verb
/// refuses the record and sends the operator back here.
#[test]
fn an_older_schema_at_the_current_version_still_migrates() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    manifest["schema_version"] = 2.into();
    let object = manifest.as_object_mut().unwrap();
    object.remove("plan_zone");
    object.remove("docs_scratch");
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &(serde_json::to_string_pretty(&manifest).unwrap() + "\n"),
    );

    // Every other verb refuses the record and names the upgrade.
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(65)
        .stderr(predicate::str::contains("run 'sdd upgrade'"));

    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN migrate the record"));

    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK migrated the record"));

    let after: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    assert_eq!(after["schema_version"], 3);
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
}

/// A version-2 record carries integration blocks, and the upgrade reads
/// them: without that the edited-block conflict check is skipped and the
/// operator's edits inside the markers are overwritten.
#[test]
fn a_version_two_instance_still_refuses_an_edited_managed_block() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    downgrade_to_v2(&fixture);
    let config = fixture
        .read(".pre-commit-config.yaml")
        .replace("entry: sdd verify", "entry: sdd verify --target .");
    fixture.write(".pre-commit-config.yaml", &config);

    fixture
        .cmd()
        .args(["upgrade", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "CONFLICT locally edited managed block: .pre-commit-config.yaml",
        ));
}
