//! Integration: `sdd init` installs, previews, refuses, and rolls back.

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
fn installs_both_profiles_and_they_verify() {
    for (profile, root) in [("codebase", "docs"), ("knowledge-base", "_docs")] {
        let fixture = Fixture::new();
        fixture.install(profile);
        assert!(
            fixture
                .path()
                .join(root)
                .join("specs/SPEC-instance.md")
                .is_file()
        );
        assert!(
            fixture
                .path()
                .join(".spec-driven-docs/markdownlint")
                .is_dir()
        );
        // VERIFIES distribution:a-skill-has-one-owner: an instance carries no
        // skill, so a session opened here sees the user-scope copy alone.
        for root in [".claude/skills", ".agents/skills"] {
            assert!(
                !fixture.path().join(root).exists(),
                "the instance installed {root}"
            );
        }
        assert!(
            !fixture
                .read(".spec-driven-docs/manifest.json")
                .contains("skills/")
        );
        fixture
            .cmd()
            .args(["verify", "--target", &fixture.target()])
            .assert()
            .success()
            .stdout(predicate::str::contains("OK spec-driven-docs"));
    }
}

#[test]
fn installs_into_a_target_with_spaces() {
    let parent = tempfile::tempdir().unwrap();
    let spaced = parent.path().join("codebase with spaces");
    std::fs::create_dir_all(spaced.join(".git")).unwrap();
    assert_cmd::Command::cargo_bin("sdd")
        .unwrap()
        .args([
            "init",
            "--target",
            spaced.to_str().unwrap(),
            "--profile",
            "codebase",
            "--apply",
        ])
        .assert()
        .success();
    assert!(spaced.join("docs/specs").is_dir());
}

#[test]
fn reinstall_is_byte_stable() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let before = fixture.tree_digest();
    fixture.install("knowledge-base");
    assert_eq!(before, fixture.tree_digest(), "reinstall changed bytes");
}

#[test]
fn an_adopted_edit_survives_reinstall_and_reports_drift_until_rerecorded() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let spec = "_docs/specs/SPEC-instance.md";
    let edited = fixture.read(spec) + "\nLocal addition.\n";
    fixture.write(spec, &edited);

    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "DRIFT adopted file requires reconciliation",
        ));

    fixture.install("knowledge-base");
    assert_eq!(
        fixture.read(spec),
        edited,
        "reinstall rewrote an adopted file"
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRIFT").not());
}

#[test]
fn a_non_empty_target_without_an_instance_defaults_to_dry_run() {
    let fixture = Fixture::new();
    fixture.write("README.md", "# Existing project\n");
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "knowledge-base",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("re-run with --apply"))
        .stdout(predicate::str::contains("DRY RUN: no files written"));
    assert_eq!(digest, fixture.tree_digest());
}

#[test]
fn an_explicit_dry_run_writes_nothing() {
    let fixture = Fixture::new();
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "knowledge-base",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN: no files written"));
    assert_eq!(digest, fixture.tree_digest());
}

#[test]
fn rejects_relative_root_and_canon_targets() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["init", "--target", "relative/path", "--profile", "codebase"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("target must be absolute"));
    fixture
        .cmd()
        .args(["init", "--target", "/", "--profile", "codebase"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("refusing root target"));

    let canon = Fixture::new();
    canon.write("Cargo.toml", "[package]\nname = \"spec-driven-docs\"\n");
    canon.write("inner/.git/keep", "");
    for target in [canon.target(), format!("{}/inner", canon.target())] {
        canon
            .cmd()
            .args(["init", "--target", &target, "--profile", "codebase"])
            .assert()
            .code(64)
            .stderr(predicate::str::contains(
                "target is inside the canon checkout",
            ));
    }
}

#[test]
fn an_unknown_profile_is_a_clap_error() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "strictest",
        ])
        .assert()
        .code(2);
}

#[test]
fn splicing_preserves_every_byte_outside_the_markers() {
    let fixture = Fixture::new();
    let config = "# above the sequence\nrepos:\n  # a hand comment\n  - repo: https://example.invalid/hooks\n    rev: v1 # trailing\nci:\n  autofix_prs: false\n";
    fixture.write(".pre-commit-config.yaml", config);
    fixture.install("knowledge-base");
    let spliced = fixture.read(".pre-commit-config.yaml");
    assert!(spliced.starts_with("# above the sequence\nrepos:\n  # a hand comment\n"));
    assert!(spliced.contains("    rev: v1 # trailing\n# BEGIN spec-driven-docs managed\n"));
    assert!(spliced.contains("# END spec-driven-docs managed\nci:\n  autofix_prs: false\n"));
    assert!(spliced.contains("entry: sdd gate adr-filename-shape"));
    assert!(spliced.contains("entry: sdd verify"));
}

#[test]
fn every_registered_gate_is_wired_into_the_block() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let config = fixture.read(".pre-commit-config.yaml");
    for gate in spec_driven_docs::gates::GATES {
        assert!(
            config.contains(&format!("- id: {}\n", gate.id)),
            "{} is unwired",
            gate.id
        );
    }
    assert!(
        config.contains("files: '^docs/decisions/.*\\.md$'"),
        "docs_root not substituted"
    );
}

#[test]
fn malformed_markers_are_refused_byte_untouched() {
    for config in [
        "repos:\n# BEGIN spec-driven-docs managed\n",
        "repos:\n# END spec-driven-docs managed\n",
        "repos:\n# END spec-driven-docs managed\n# BEGIN spec-driven-docs managed\n",
        "repos:\n# BEGIN spec-driven-docs managed\n# END spec-driven-docs managed\n# BEGIN spec-driven-docs managed\n# END spec-driven-docs managed\n",
    ] {
        let fixture = Fixture::new();
        fixture.write(".pre-commit-config.yaml", config);
        let digest = fixture.tree_digest();
        fixture
            .cmd()
            .args([
                "init",
                "--target",
                &fixture.target(),
                "--profile",
                "codebase",
                "--apply",
            ])
            .assert()
            .code(65);
        assert_eq!(
            digest,
            fixture.tree_digest(),
            "a refused install changed bytes"
        );
    }
}

#[test]
fn a_config_without_a_repos_key_is_refused() {
    let fixture = Fixture::new();
    fixture.write(".pre-commit-config.yaml", "ci:\n  autofix_prs: false\n");
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "codebase",
            "--apply",
        ])
        .assert()
        .code(65)
        .stderr(predicate::str::contains("no top-level repos: key"));
}

#[test]
fn a_symlinked_destination_is_refused_and_nothing_is_written() {
    let fixture = Fixture::new();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), fixture.path().join("_docs")).unwrap();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "knowledge-base",
            "--apply",
        ])
        .assert()
        .code(73)
        .stderr(predicate::str::contains(
            "escapes the target through a symlink",
        ));
    assert!(
        !fixture.path().join(".spec-driven-docs").exists(),
        "refusal still wrote files"
    );
}

#[test]
fn a_directory_at_a_file_destination_is_refused() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.path().join("_docs/specs/SPEC-instance.md")).unwrap();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "knowledge-base",
            "--apply",
        ])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("not a regular file"));
}

#[test]
fn a_dangling_symlink_counts_as_content() {
    let fixture = Fixture::new();
    std::os::unix::fs::symlink("/nonexistent-target", fixture.path().join("dangling")).unwrap();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "knowledge-base",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN"));
}

/// VERIFIES distribution:initialization-preserves-project-content
#[test]
fn a_refused_apply_names_what_made_it_refuse() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    // A second spec claiming a seeded rule ID: the target writes, then fails
    // verification. The refusal has to carry that reason, because the tree it
    // describes is rolled back before the caller can look at it.
    fixture.write(
        "_docs/specs/SPEC-copy.md",
        "# Copy Specification\n\n### `instance:the-manifest-stays-readable` — Duplicated\n\nVerify: `true`\n",
    );
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "knowledge-base",
            "--apply",
        ])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("apply aborted"))
        .stderr(predicate::str::contains("duplicate rule ID in local specs"));
    assert_eq!(
        digest,
        fixture.tree_digest(),
        "a refused apply changed bytes"
    );
}
