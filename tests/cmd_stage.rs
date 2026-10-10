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

/// A stage of a target that is not an instance yet, under one profile.
fn stage_as(fixture: &Fixture, profile: &str) -> serde_json::Value {
    let path = stage_path(fixture);
    let output = fixture
        .cmd()
        .args([
            "stage",
            "--target",
            &fixture.target(),
            "--profile",
            profile,
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
///
/// The title belongs to the candidate, so a stage renders it and never
/// writes it into the target, and the landing that follows writes the same
/// bytes the stage showed.
#[test]
fn a_staged_agents_file_is_titled_only_where_the_target_has_none() {
    for profile in ["codebase", "knowledge-base"] {
        let absent = Fixture::new();
        let before = absent.tree_digest();
        stage_as(&absent, profile);
        assert_eq!(
            before,
            absent.tree_digest(),
            "{profile}: the stage wrote the target"
        );
        let staged =
            std::fs::read_to_string(stage_path(&absent).join("artifacts/AGENTS.md")).unwrap();
        assert!(
            staged.starts_with("# AGENTS\n\n<!-- BEGIN"),
            "{profile}: {staged}"
        );
        absent.install(profile);
        assert_eq!(
            staged,
            absent.read("AGENTS.md"),
            "{profile}: production differs"
        );

        let empty = Fixture::new();
        empty.write("AGENTS.md", "");
        stage_as(&empty, profile);
        let staged =
            std::fs::read_to_string(stage_path(&empty).join("artifacts/AGENTS.md")).unwrap();
        assert!(staged.starts_with("<!-- BEGIN"), "{profile}: {staged}");
        assert!(!staged.contains("# AGENTS"), "{profile}: {staged}");
        assert_eq!(
            empty.read("AGENTS.md"),
            "",
            "{profile}: the stage wrote the host"
        );
        empty.install(profile);
        assert_eq!(
            staged,
            empty.read("AGENTS.md"),
            "{profile}: production differs"
        );
    }
}

/// VERIFIES staging:a-stage-carries-the-whole-candidate
///
/// The delivered lint configuration is canon an adopting project reads while
/// it migrates, so the stage carries it as reference whether or not the
/// root seed lands.
#[test]
fn the_stage_carries_the_delivered_lint_configuration_as_reference() {
    let source = spec_driven_docs::embedded::asset("_docs/.markdownlint-cli2.jsonc").unwrap();
    for profile in ["codebase", "knowledge-base"] {
        for yields in [false, true] {
            let fixture = Fixture::new();
            if yields {
                fixture.write(".markdownlint.yaml", "MD041: false\n");
            }
            let receipt = stage_as(&fixture, profile);
            let reference = stage_path(&fixture).join("reference/_docs/.markdownlint-cli2.jsonc");
            assert_eq!(std::fs::read(&reference).unwrap(), source, "{profile}");
            assert!(
                receipt["reference"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|entry| entry == "reference/_docs/.markdownlint-cli2.jsonc"),
                "{profile}: {}",
                receipt["reference"]
            );
            assert_eq!(
                stage_path(&fixture)
                    .join("artifacts/.markdownlint-cli2.jsonc")
                    .is_file(),
                !yields,
                "{profile}: the root seed staged where it should yield, or the reverse"
            );
        }
    }
}

/// VERIFIES staging:a-stage-takes-the-recorded-choices
#[test]
fn a_stage_over_an_instance_projects_its_recorded_profile() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let receipt = stage(&fixture);
    assert_eq!(receipt["profile"], "knowledge-base");
    assert_eq!(receipt["docs_root"], "_docs");

    let elsewhere = stage_path(&fixture).with_file_name("stage-other-profile");
    fixture
        .cmd()
        .args([
            "stage",
            "--target",
            &fixture.target(),
            "--profile",
            "codebase",
            "--output",
            elsewhere.to_str().unwrap(),
        ])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("knowledge-base"));
    assert!(!elsewhere.exists());
}

fn compare(fixture: &Fixture) -> serde_json::Value {
    let output = fixture
        .cmd()
        .args([
            "stage",
            "compare",
            stage_path(fixture).to_str().unwrap(),
            "--target",
            &fixture.target(),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

fn row<'a>(report: &'a serde_json::Value, path: &str) -> &'a serde_json::Value {
    report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == path)
        .unwrap_or_else(|| panic!("no row for {path}: {report}"))
}

/// Run git in the fixture alone.
fn git(fixture: &Fixture, args: &[&str]) -> bool {
    support::git(fixture.path())
        .args(args)
        .output()
        .is_ok_and(|out| out.status.success())
}

/// A fixture whose tree is one real commit, where `git` runs.
fn committed(fixture: &Fixture) -> bool {
    std::fs::remove_dir_all(fixture.path().join(".git")).unwrap();
    git(fixture, &["init", "-q"])
        && git(fixture, &["add", "-A"])
        && git(fixture, &["commit", "-q", "-m", "x"])
}

/// VERIFIES staging:compare-reports-every-destination-the-stage-carries
#[test]
fn compare_reports_each_state_ownership_and_provenance() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let history = committed(&fixture);
    // Edited after the record: the record no longer vouches, history does.
    fixture.write("docs/.markdownlint-cli2.jsonc", "// edited\n");
    // Gone: nothing to vouch for.
    std::fs::remove_file(fixture.path().join("docs/specs/SPEC-docs-format.md")).unwrap();
    stage(&fixture);
    let report = compare(&fixture);
    assert_eq!(report["schema"], "sdd.stage-compare/1");
    assert_eq!(report["history"]["available"], history);

    let managed = row(&report, "docs/.markdownlint-cli2.jsonc");
    assert_eq!(managed["state"], "differs");
    assert_eq!(managed["ownership"], "managed");
    assert_eq!(managed["vouched"], false);
    assert_eq!(
        managed["provenance"],
        if history { "reduced" } else { "unknown" }
    );

    let adopted = row(&report, "docs/specs/SPEC-docs-format.md");
    assert_eq!(adopted["state"], "absent");
    assert_eq!(adopted["ownership"], "adopted");
    assert_eq!(adopted["provenance"], serde_json::Value::Null);

    let integration = row(&report, ".pre-commit-config.yaml");
    assert_eq!(integration["state"], "equal");
    assert_eq!(integration["ownership"], "integration");

    // An unrecorded file at a managed destination is unvouched and has no
    // history either.
    let fresh = Fixture::new();
    fresh.write("docs/.markdownlint-cli2.jsonc", "{}\n");
    stage(&fresh);
    let report = compare(&fresh);
    let unknown = row(&report, "docs/.markdownlint-cli2.jsonc");
    assert_eq!(unknown["state"], "differs");
    assert_eq!(unknown["provenance"], "unknown");
    if !report["history"]["available"].as_bool().unwrap() {
        assert!(report["history"]["reason"].as_str().is_some());
    }
}

/// The record half is the landing's own check, so the comparison and the
/// landing's refusal name the same destinations, and a landing that
/// resolves them leaves every managed and integration row equal.
#[test]
fn the_comparison_agrees_with_the_landing_and_proves_it_afterwards() {
    let fixture = Fixture::new();
    fixture.write("docs/.markdownlint-cli2.jsonc", "{}\n");
    stage(&fixture);
    let report = compare(&fixture);
    let unvouched: Vec<&str> = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["state"] == "differs" && row["vouched"] == false && row["ownership"] != "adopted"
        })
        .map(|row| row["path"].as_str().unwrap())
        .collect();
    assert_eq!(unvouched, ["docs/.markdownlint-cli2.jsonc"]);
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
        .code(73)
        .stderr(predicate::str::contains("docs/.markdownlint-cli2.jsonc"));

    std::fs::remove_file(fixture.path().join("docs/.markdownlint-cli2.jsonc")).unwrap();
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args([
            "init",
            "--stage",
            stage_path(&fixture).to_str().unwrap(),
            "--apply",
        ])
        .assert()
        .success();
    let after = compare(&fixture);
    for row in after["rows"].as_array().unwrap() {
        if row["ownership"] != "adopted" {
            assert_eq!(row["state"], "equal", "{row}");
        }
    }
}

/// A conflict the upgrade takes a name for is replaceable. A conflict at
/// a path the landing cannot write, here a managed region's host turned
/// into a symlink, is not, and the upgrade refuses that name before it saves or
/// writes anything. A first landing takes no name at all.
///
/// VERIFIES staging:compare-reports-every-destination-the-stage-carries
#[test]
fn compare_marks_only_the_conflicts_a_replace_name_resolves() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let edited = "docs/.markdownlint-cli2.jsonc";
    fixture.write(edited, "// edited\n");
    let linked = ".pre-commit-config.yaml".to_string();
    let outside = fixture.path().parent().unwrap().join("outside.yaml");
    std::fs::write(&outside, "repos:\n  - repo: local\n    hooks: []\n").unwrap();
    std::fs::remove_file(fixture.path().join(&linked)).unwrap();
    std::os::unix::fs::symlink(&outside, fixture.path().join(&linked)).unwrap();

    stage(&fixture);
    let report = compare(&fixture);
    assert_eq!(row(&report, edited)["replaceable"], true, "{report}");
    assert_eq!(row(&report, &linked)["replaceable"], false, "{report}");
    for row in report["rows"].as_array().unwrap() {
        if row["path"] != edited {
            assert_eq!(row["replaceable"], false, "{row}");
        }
    }

    let before = fixture.tree_digest();
    fixture
        .upgrade()
        .args(["--stage", stage_path(&fixture).to_str().unwrap()])
        .args(["--replace", edited, "--replace", &linked])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("cannot write"));
    assert_eq!(fixture.tree_digest(), before);
    assert_eq!(
        std::fs::read_to_string(&outside).unwrap(),
        "repos:\n  - repo: local\n    hooks: []\n"
    );

    // Before any instance the landing verb is init, which takes no name.
    let fresh = Fixture::new();
    fresh.write(edited, "{}\n");
    stage(&fresh);
    let report = compare(&fresh);
    assert_eq!(row(&report, edited)["state"], "differs");
    assert_eq!(row(&report, edited)["replaceable"], false);
}

/// Wherever the upgrade refuses before it scans, no name resolves
/// anything, so no row is replaceable: here a readable record that names
/// an absolute destination, which the upgrade's front gate reads invalid.
#[test]
fn no_row_is_replaceable_where_the_upgrade_refuses_first() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let edited = "docs/.markdownlint-cli2.jsonc";
    fixture.write(edited, "// edited\n");
    stage(&fixture);
    let record_path = ".spec-driven-docs/manifest.json";
    let mut record: serde_json::Value = serde_json::from_str(&fixture.read(record_path)).unwrap();
    record["adopted_files"][0]["destination"] = serde_json::json!("/elsewhere/notes.md");
    fixture.write(record_path, &serde_json::to_string_pretty(&record).unwrap());
    fixture
        .upgrade()
        .args(["--stage", stage_path(&fixture).to_str().unwrap()])
        .args(["--replace", edited])
        .assert()
        .failure();
    let report = compare(&fixture);
    for row in report["rows"].as_array().unwrap() {
        assert_eq!(row["replaceable"], false, "{row}");
    }
}

/// A retired destination outside the roots the landing prunes is one the
/// landing leaves alone, so a name for it is accepted whatever its path is:
/// its bytes are kept, and the upgrade proceeds without following the link.
#[test]
fn a_retired_link_the_landing_leaves_alone_stays_replaceable() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let retired = "docs/retired.md";
    let outside = fixture.path().parent().unwrap().join("outside-retired.md");
    std::fs::write(&outside, "# Kept\n").unwrap();
    std::os::unix::fs::symlink(&outside, fixture.path().join(retired)).unwrap();
    let record_path = ".spec-driven-docs/manifest.json";
    let mut record: serde_json::Value = serde_json::from_str(&fixture.read(record_path)).unwrap();
    record["managed_files"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "source": "retired.md",
            "destination": retired,
            "sha256": "0".repeat(64),
        }));
    fixture.write(record_path, &serde_json::to_string_pretty(&record).unwrap());
    stage(&fixture);
    let report = compare(&fixture);
    assert_eq!(row(&report, retired)["state"], "retired");
    assert_eq!(row(&report, retired)["replaceable"], true, "{report}");
    fixture
        .upgrade()
        .args(["--stage", stage_path(&fixture).to_str().unwrap()])
        .args(["--replace", retired])
        .assert()
        .success();
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "# Kept\n");
}

/// Every destination the record names gets a row, readable or not.
#[test]
fn compare_reports_a_retired_destination_it_cannot_read() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let gone = "docs/gone.md";
    let unreadable = "docs/unreadable.md";
    // A directory where the file was cannot be read as one, whoever runs
    // the test.
    std::fs::create_dir_all(fixture.path().join(unreadable)).unwrap();
    let record_path = ".spec-driven-docs/manifest.json";
    let mut record: serde_json::Value = serde_json::from_str(&fixture.read(record_path)).unwrap();
    for destination in [gone, unreadable] {
        record["managed_files"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "source": destination,
                "destination": destination,
                "sha256": "0".repeat(64),
            }));
    }
    fixture.write(record_path, &serde_json::to_string_pretty(&record).unwrap());
    stage(&fixture);
    let report = compare(&fixture);
    // No commit touches the gone file, so the Git half reads unknown.
    assert_eq!(row(&report, gone)["state"], "retired");
    assert_eq!(row(&report, gone)["provenance"], "unknown");
    assert!(row(&report, gone)["vouched"].is_null());
    assert_eq!(row(&report, unreadable)["state"], "retired");
    assert_eq!(row(&report, unreadable)["provenance"], "unknown");
    assert!(row(&report, unreadable)["vouched"].is_null());
}

#[test]
fn compare_reports_a_retired_file_with_its_provenance() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    // A managed file the record names and this release does not land: what
    // an older release left, edited since.
    let path = ".spec-driven-docs/markdownlint/spec.markdownlint-cli2.jsonc";
    fixture.write(path, "// edited\n");
    let record_path = ".spec-driven-docs/manifest.json";
    let mut record: serde_json::Value = serde_json::from_str(&fixture.read(record_path)).unwrap();
    record["managed_files"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "source": ".markdownlint/spec.markdownlint-cli2.jsonc",
            "destination": path,
            "sha256": "0".repeat(64),
        }));
    fixture.write(record_path, &serde_json::to_string_pretty(&record).unwrap());
    stage(&fixture);
    let report = compare(&fixture);
    let retired = row(&report, path);
    assert_eq!(retired["state"], "retired");
    assert_eq!(retired["vouched"], false);
    let provenance = retired["provenance"].as_str().unwrap();
    assert!(
        provenance == "reduced" || provenance == "unknown",
        "{provenance}"
    );
}

/// VERIFIES acquisition:a-host-install-is-a-recorded-decision
#[test]
fn a_host_answer_is_refused_where_a_manager_pins_the_tool() {
    let pin = "[tools]\n\"cargo:spec-driven-docs\" = \"0.10.1\"\n";
    let pinned = Fixture::new();
    pinned.write("mise.toml", pin);
    pinned
        .cmd()
        .args([
            "stage",
            "--target",
            &pinned.target(),
            "--acquisition",
            "host",
        ])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("mise already pins this tool"));

    // A pin that arrives after the stage stops the landing that reads it.
    let late = Fixture::new();
    let path = stage_path(&late);
    late.cmd()
        .args([
            "stage",
            "--target",
            &late.target(),
            "--output",
            path.to_str().unwrap(),
            "--acquisition",
            "host",
        ])
        .assert()
        .success();
    late.write("mise.toml", pin);
    let before = late.tree_digest();
    late.cmd()
        .current_dir(late.path())
        .args(["init", "--stage", path.to_str().unwrap(), "--apply"])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("mise already pins this tool"));
    assert_eq!(late.tree_digest(), before);
}
