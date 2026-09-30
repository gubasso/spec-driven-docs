//! Integration: `sdd verify` holds an instance to its record.

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
        .replace("\"schema_version\": 3", "\"schema_version\": 1");
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
    fixture.write("_docs/.markdownlint-cli2.jsonc", "{}\n");
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL managed drift: _docs/.markdownlint-cli2.jsonc",
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
            "FAIL missing managed block: .pre-commit-config.yaml",
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
    let managed = "_docs/.markdownlint-cli2.jsonc";
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
    manifest["adopted_files"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["destination"] != "_docs/specs/SPEC-instance.md");
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
            "FAIL manifest omits a declared projection: _docs/specs/SPEC-instance.md",
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
    manifest["adopted_files"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["destination"] != "_docs/specs/SPEC-instance.md");
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

/// A record whose managed destinations are rewritten to their authored
/// paths still reads as a consumer's, because the canon's record is told
/// apart by what only it records, not by where a managed file sits.
#[test]
fn a_record_rewritten_to_authored_paths_is_still_held_as_a_consumer() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    manifest["adopted_files"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["destination"] != "_docs/specs/SPEC-instance.md");
    let managed = manifest["managed_files"].as_array_mut().unwrap();
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

/// SATISFIES instance:the-lint-configuration-composes
#[test]
fn a_library_configuration_at_the_documentation_root_fails_naming_it() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    fixture.write("docs/.markdownlint.jsonc", "{ \"MD013\": true }\n");
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL docs/.markdownlint.jsonc replaces the delivered lint configuration",
        ))
        .stdout(predicate::str::contains(
            "(instance:the-lint-configuration-composes)",
        ));
}

/// SATISFIES instance:the-lint-configuration-composes
///
/// The walk is recursive: a configuration nested deeper than the directory
/// itself replaces the shapes for everything beneath it just the same.
#[test]
fn a_configuration_beneath_the_specs_or_the_records_fails_at_any_depth() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    fixture.write("docs/specs/.markdownlint-cli2.jsonc", "{}\n");
    fixture.write(
        "docs/decisions/archive/old/.markdownlint.yaml",
        "MD043: false\n",
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL docs/specs/.markdownlint-cli2.jsonc replaces",
        ))
        .stdout(predicate::str::contains(
            "FAIL docs/decisions/archive/old/.markdownlint.yaml replaces",
        ));
}

/// SATISFIES instance:the-lint-configuration-composes
///
/// The linter reads a configuration through a symbolic link, so a linked
/// one replaces the shapes exactly as a regular file does, in either
/// profile, and verification names it by the path the link sits at.
#[test]
fn a_symlinked_configuration_fails_like_a_regular_one() {
    for root in ["docs", "_docs"] {
        let profile = if root == "docs" {
            "codebase"
        } else {
            "knowledge-base"
        };
        let fixture = Fixture::new();
        fixture.install(profile);
        let outside = tempfile::tempdir().unwrap();
        let held = outside.path().join("overrides.jsonc");
        std::fs::write(&held, "{ \"overrides\": [] }\n").unwrap();
        for link in [
            format!("{root}/specs/.markdownlint-cli2.jsonc"),
            format!("{root}/.markdownlint.json"),
        ] {
            std::os::unix::fs::symlink(&held, fixture.path().join(&link)).unwrap();
        }
        // A dangling link is still a configuration the linter tries to read.
        std::os::unix::fs::symlink(
            outside.path().join("absent.yaml"),
            fixture
                .path()
                .join(format!("{root}/decisions/.markdownlint.yaml")),
        )
        .unwrap();
        let assert = fixture
            .cmd()
            .args(["verify", "--target", &fixture.target()])
            .assert()
            .code(1);
        let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
        for named in [
            format!("FAIL {root}/specs/.markdownlint-cli2.jsonc replaces"),
            format!("FAIL {root}/.markdownlint.json replaces"),
            format!("FAIL {root}/decisions/.markdownlint.yaml replaces"),
        ] {
            assert!(
                stdout.contains(&named),
                "{profile}: missing '{named}':\n{stdout}"
            );
        }
        assert!(stdout.contains("(instance:the-lint-configuration-composes)"));
    }
}

/// Linked scan roots and their ancestors stay outside the composition scan,
/// and verification names the link itself as the failure.
/// VERIFIES instance:the-lint-configuration-composes
#[test]
fn a_linked_directory_fails_ownership_without_scanning_its_contents() {
    for (profile, root) in [("codebase", "docs"), ("knowledge-base", "_docs")] {
        for boundary in ["", "specs", "decisions"] {
            let fixture = Fixture::new();
            fixture.install(profile);
            let relative = if boundary.is_empty() {
                std::path::PathBuf::from(root)
            } else {
                std::path::Path::new(root).join(boundary)
            };
            let outside = tempfile::tempdir().unwrap();
            let moved = outside.path().join("held");
            std::fs::rename(fixture.path().join(&relative), &moved).unwrap();
            std::fs::write(moved.join(".markdownlint.json"), "{}\n").unwrap();
            std::os::unix::fs::symlink(&moved, fixture.path().join(&relative)).unwrap();

            let assert = fixture
                .cmd()
                .args(["verify", "--target", &fixture.target()])
                .assert()
                .code(1);
            let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
            let linked = relative.display();
            assert!(
                stdout.contains(&format!(
                    "FAIL {linked} is reached through a symlink, so no check sees the lint configuration beneath it"
                )),
                "{profile}/{boundary}: the linked directory was not named:\n{stdout}"
            );
            assert!(stdout.contains("(instance:the-lint-configuration-composes)"));
            assert!(
                !stdout.contains("replaces the delivered lint configuration"),
                "{profile}/{boundary}: the scan enumerated external contents:\n{stdout}"
            );
        }
    }
}

/// The repository root is the project's: a configuration there merges
/// beneath the documentation root's and cannot turn the shapes off.
#[test]
fn a_configuration_at_the_repository_root_passes() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    fixture.write(".markdownlint.yaml", "MD043: false\n");
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
}

/// The managed destination is templated, so a knowledge-base instance holds
/// it at the path the canon authors it at. That equality once read the
/// instance as the canon, which owes specs and templates no instance holds.
#[test]
fn a_fresh_knowledge_base_landing_verifies() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    assert!(
        fixture
            .path()
            .join("_docs/.markdownlint-cli2.jsonc")
            .is_file()
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("FAIL").not());
}

/// A project overruling a specification it owns is exercising ownership:
/// the disagreement is reported, and nothing fails.
#[test]
fn debt_without_the_sentinel_is_a_note_and_the_gates_stay_green() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("method/legacy.md", &"line\n".repeat(250));
    fixture
        .cmd()
        .args(["debt", "baseline", "--target", &fixture.target(), "--apply"])
        .assert()
        .success();
    // The project's own copy of the specification predates the sentinel.
    let spec = "_docs/specs/SPEC-budget-debt.md";
    let older = fixture.read(spec).replace(
        "budget-debt:a-recorded-dimension-only-shrinks",
        "budget-debt:an-older-sentence",
    );
    fixture.write(spec, &older);
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "note: .spec-driven-docs/debt.yaml records budget debt and no local specification defines `budget-debt:a-recorded-dimension-only-shrinks`",
        ))
        .stdout(predicate::str::contains("_docs/specs/SPEC-budget-debt.md owns it"))
        .stdout(predicate::str::contains("run 'sdd policy reconcile'"));
    fixture
        .cmd()
        .args(["gate", "chapter-size-cap"])
        .current_dir(fixture.path())
        .assert()
        .success();
}

fn select(fixture: &Fixture, block: &str) {
    let declaration = fixture
        .read(".spec-driven-docs/config.yaml")
        .replace("writing_style:\n  source: builtin\n  path: null\n", block);
    fixture.write(".spec-driven-docs/config.yaml", &declaration);
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--apply"])
        .assert()
        .success();
}

fn drop_sentinel(fixture: &Fixture, spec: &str, sentinel: &str) {
    let older = fixture
        .read(spec)
        .replace(sentinel, &sentinel.replace(':', ":older-"));
    fixture.write(spec, &older);
}

#[test]
fn a_non_builtin_selection_without_the_sentinel_is_a_note() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    select(&fixture, "writing_style:\n  source: none\n  path: null\n");
    drop_sentinel(
        &fixture,
        "_docs/specs/SPEC-writing-policy.md",
        "writing-policy:the-project-selects-one-source",
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "note: .spec-driven-docs/config.yaml selects a writing style other than builtin and no local specification defines `writing-policy:the-project-selects-one-source`",
        ))
        .stdout(predicate::str::contains("_docs/specs/SPEC-writing-policy.md owns it"));
}

#[test]
fn a_builtin_selection_needs_no_sentinel() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    drop_sentinel(
        &fixture,
        "_docs/specs/SPEC-writing-policy.md",
        "writing-policy:the-project-selects-one-source",
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("no local specification defines").not());
}
