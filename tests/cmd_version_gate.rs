//! Integration: every command passes the version gate before it runs.

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
use spec_driven_docs::landing::gate::{Class, TABLE, TargetSource, row_of};
use support::Fixture;

/// The refusal of a target with no instance.
const NO_INSTANCE: &str = "no spec-driven-docs instance";

/// Rewrite the version the instance record names.
fn record_at(fixture: &Fixture, version: &str) {
    let mut record: serde_json::Value =
        serde_json::from_str(&fixture.read(".spec-driven-docs/manifest.json")).unwrap();
    record["canon_version"] = version.into();
    fixture.write(
        ".spec-driven-docs/manifest.json",
        &format!("{}\n", serde_json::to_string_pretty(&record).unwrap()),
    );
}

/// An installed instance whose record names `version`.
fn instance_at(version: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    record_at(&fixture, version);
    fixture
}

/// Every action of the command tree, as its command path spells it, with
/// whether it takes a `--target`.
fn actions() -> Vec<(String, bool)> {
    fn walk(command: &clap::Command, prefix: &str, out: &mut Vec<(String, bool)>) {
        for sub in command.get_subcommands() {
            let name = sub.get_name();
            if name == "help" {
                continue;
            }
            let path = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix} {name}")
            };
            let has_children = sub
                .get_subcommands()
                .any(|child| child.get_name() != "help");
            if !has_children || !sub.is_subcommand_required_set() {
                let targeted = sub.get_arguments().any(|arg| arg.get_id() == "target");
                out.push((path.clone(), targeted));
            }
            walk(sub, &path, out);
        }
    }
    let mut out = Vec::new();
    walk(
        &<spec_driven_docs::cli::Cli as clap::CommandFactory>::command(),
        "",
        &mut out,
    );
    out
}

/// VERIFIES setup:every-command-has-a-gate-class
#[test]
fn every_action_has_a_gate_class() {
    let actions = actions();
    assert!(actions.len() > 30, "{actions:?}");
    for (path, targeted) in &actions {
        let row = row_of(path).unwrap_or_else(|| panic!("{path} carries no gate class"));
        if row.class != Class::Exempt && *targeted {
            assert_ne!(
                row.target,
                TargetSource::None,
                "{path} takes --target, so its row must read it"
            );
        }
    }
    for row in TABLE {
        assert!(
            actions.iter().any(|(path, _)| path == row.path),
            "{} is a row no command answers to",
            row.path
        );
    }
}

/// VERIFIES setup:every-command-has-a-gate-class
///
/// The calls the setup skill makes before a landing run on a target with
/// no instance.
#[test]
fn the_setup_sequence_runs_before_any_instance() {
    let fixture = Fixture::new();
    let target = fixture.target();
    let stage = fixture.path().with_extension("stage-under-test");
    let stage = stage.to_str().unwrap();
    let calls: Vec<Vec<&str>> = vec![
        vec!["assess", "--target", &target],
        vec!["status", "--target", &target],
        vec!["env", "--target", &target, "--json"],
        vec!["lint", "--target", &target, "--profile", "knowledge-base"],
        vec![
            "stage",
            "--target",
            &target,
            "--profile",
            "knowledge-base",
            "--output",
            stage,
        ],
        vec!["stage", "compare", stage, "--target", &target],
        vec!["init", "--target", &target, "--profile", "knowledge-base"],
    ];
    for call in calls {
        fixture
            .cmd()
            .args(&call)
            .assert()
            .success()
            .stderr(predicate::str::contains(NO_INSTANCE).not());
    }
}

/// VERIFIES setup:every-command-has-a-gate-class
///
/// The calls an upgrade makes run on an instance an older release landed.
#[test]
fn the_upgrade_sequence_runs_on_an_older_instance() {
    let fixture = instance_at("0.16.0");
    let target = fixture.target();
    let stage = fixture.path().with_extension("stage-under-test");
    let stage = stage.to_str().unwrap();
    for call in [
        vec!["stage", "--target", &target, "--output", stage],
        vec!["stage", "compare", stage, "--target", &target],
        vec!["upgrade", "--target", &target, "--dry-run"],
        vec!["upgrade", "--target", &target],
    ] {
        fixture.cmd().args(&call).assert().success();
    }
}

/// VERIFIES setup:a-gated-command-names-its-fix
#[test]
fn a_writing_action_refuses_a_target_with_no_instance() {
    let fixture = Fixture::new();
    let target = fixture.target();
    for call in [
        vec!["policy", "reconcile", "--target", &target, "--apply"],
        vec!["hooks", "--target", &target, "--apply"],
    ] {
        let before = fixture.tree_digest();
        fixture
            .cmd()
            .args(&call)
            .assert()
            .code(73)
            .stderr(predicate::str::contains(NO_INSTANCE))
            .stderr(predicate::str::contains("sdd init"))
            .stderr(predicate::str::contains(
                "sdd skill show sdd-setup --file <path>",
            ));
        assert_eq!(before, fixture.tree_digest(), "{call:?} wrote");
    }
}

/// VERIFIES setup:a-gated-command-names-its-fix
#[test]
fn a_writing_action_refuses_when_the_binary_is_newer() {
    let fixture = instance_at("0.16.0");
    let before = fixture.tree_digest();
    fixture
        .cmd()
        .args([
            "policy",
            "reconcile",
            "--target",
            &fixture.target(),
            "--apply",
        ])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("at sdd 0.16.0"))
        .stderr(predicate::str::contains("Run `sdd upgrade`"));
    assert_eq!(
        before,
        fixture.tree_digest(),
        "the refusal changed the target"
    );
}

/// VERIFIES setup:a-gated-command-names-its-fix
#[test]
fn migrate_mark_follows_the_writing_rows_on_an_instance() {
    for (version, fix) in [("0.16.0", "sdd upgrade"), ("9.9.9", "direnv exec . sdd")] {
        let fixture = instance_at(version);
        let before = fixture.tree_digest();
        fixture
            .cmd()
            .args([
                "migrate",
                "mark",
                "notes.md",
                "mapped",
                "--target",
                &fixture.target(),
            ])
            .assert()
            .code(73)
            .stderr(predicate::str::contains(fix));
        assert_eq!(
            before,
            fixture.tree_digest(),
            "{version}: the migration state moved"
        );
    }
    // With no instance, the action is pre-landing: the gate lets it run, and
    // whatever it says next is the migration's own answer.
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args([
            "migrate",
            "mark",
            "notes.md",
            "mapped",
            "--target",
            &fixture.target(),
        ])
        .assert()
        .stderr(predicate::str::contains(NO_INSTANCE).not());
}

/// VERIFIES setup:a-gated-command-names-its-fix
///
/// Where the project pins an sdd older than the record, the refusal names
/// the pin as the fix.
#[test]
fn a_record_newer_than_the_binary_names_the_environment() {
    let fixture = instance_at("9.9.9");
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(73)
        .stderr(predicate::str::contains("which is older"))
        .stderr(predicate::str::contains("The pin").not());
    fixture.write(
        "flake.nix",
        "{\n  inputs.spec-driven-docs.url = \"github:gubasso/spec-driven-docs/v0.1.0\";\n}\n",
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(73)
        .stderr(predicate::str::contains(
            "The pin names 0.1.0. Move it to 9.9.9.",
        ));
}

/// VERIFIES setup:every-command-has-a-gate-class
#[test]
fn an_exempt_action_runs_without_an_instance() {
    let bare = Fixture::new();
    let older = instance_at("0.16.0");
    let newer = instance_at("9.9.9");
    let malformed = Fixture::new();
    malformed.write(".spec-driven-docs/manifest.json", "{ not json");
    for fixture in [&bare, &older, &newer, &malformed] {
        let home = fixture.path().with_extension("home");
        std::fs::create_dir_all(&home).unwrap();
        for call in [
            vec!["doctor"],
            vec!["skill", "show", "sdd-setup"],
            vec!["skill", "uninstall"],
            vec!["method", "writing-style"],
            vec!["self-depend", "check"],
        ] {
            fixture
                .cmd()
                .env("HOME", &home)
                .current_dir(fixture.path())
                .args(&call)
                .assert()
                .success()
                .stderr(predicate::str::contains("setup:a-gated-command-names-its-fix").not());
        }
    }
}

/// VERIFIES setup:a-gated-command-names-its-fix
#[test]
fn the_warning_never_enters_the_json_body() {
    let fixture = instance_at("0.16.0");
    let output = fixture
        .cmd()
        .args(["ki", "list", "--target", &fixture.target(), "--json"])
        .assert()
        .success()
        .stderr(predicate::str::contains("run `sdd upgrade`"))
        .get_output()
        .stdout
        .clone();
    let body: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(body.is_array() || body.is_object(), "{body}");
}
