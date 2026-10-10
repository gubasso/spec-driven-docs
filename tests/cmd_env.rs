//! Integration: `sdd env` reports the gates' environment and writes one
//! item through the loader the repository records.

// Integration tests: assertion style is the point, so the production
// restrictions on unwrap/panic and string building do not apply here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "the panic is this suite's failure signal, not control flow"
)]

mod support;

use support::Fixture;

fn env_json(fixture: &Fixture, extra: &[&str]) -> serde_json::Value {
    let output = fixture
        .cmd()
        .args(["env", "--target", &fixture.target(), "--json"])
        .args(extra)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

fn item<'a>(report: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    report["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["item"] == name)
        .unwrap()
}

/// An instance recording a docs scratch inside the repository.
fn landed_with_scratch() -> Fixture {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "codebase",
            "--docs-scratch",
            ".docs-scratch",
            "--apply",
        ])
        .assert()
        .success();
    fixture
}

/// VERIFIES setup:env-reports-the-loader-and-each-item
#[test]
fn the_report_reads_each_loader_case_and_writes_nothing() {
    let none = Fixture::new();
    let before = none.tree_digest();
    let report = env_json(&none, &[]);
    assert_eq!(report["schema"], "sdd.env/1");
    assert_eq!(report["loader"], "none");
    assert_eq!(item(&report, "docs-scratch-variable")["state"], "no-loader");
    assert_eq!(item(&report, "docs-scratch-ignore")["state"], "undeclared");
    assert_eq!(none.tree_digest(), before);

    let direnv = landed_with_scratch();
    direnv.write(".envrc", "use flake\n");
    let report = env_json(&direnv, &[]);
    assert_eq!(report["loader"], "direnv");
    assert_eq!(item(&report, "docs-scratch-variable")["state"], "unwired");
    assert_eq!(
        item(&report, "docs-scratch-variable")["file"],
        ".envrc.local"
    );
    assert_eq!(item(&report, "docs-scratch-ignore")["state"], "unwired");
    assert_eq!(
        item(&report, "docs-scratch-ignore")["value"],
        ".docs-scratch"
    );

    let dotenv = Fixture::new();
    dotenv.write(".env", "OTHER=1\n");
    let report = env_json(&dotenv, &[]);
    assert_eq!(report["loader"], "dotenv");
    assert_eq!(item(&report, "docs-scratch-variable")["file"], ".env");

    let outside = Fixture::new();
    outside
        .cmd()
        .args([
            "init",
            "--target",
            &outside.target(),
            "--profile",
            "codebase",
            "--docs-scratch",
            "../notes",
            "--apply",
        ])
        .assert()
        .success();
    let report = env_json(&outside, &[]);
    assert_eq!(item(&report, "docs-scratch-ignore")["state"], "outside");
}

/// VERIFIES setup:env-writes-one-item-through-the-recorded-loader
#[test]
fn each_item_writes_its_line_and_ignore_entry_once() {
    let fixture = landed_with_scratch();
    fixture.write(".envrc", "use flake\n");

    let report = env_json(
        &fixture,
        &["--apply", "docs-scratch-variable", "--value", "../my notes"],
    );
    assert_eq!(item(&report, "docs-scratch-variable")["state"], "wired");
    assert_eq!(
        fixture.read(".envrc.local"),
        "export SDD_DOCS_SCRATCH='../my notes'\n"
    );
    assert!(
        fixture
            .read(".envrc")
            .contains("source_env_if_exists .envrc.local")
    );
    assert!(fixture.read(".gitignore").contains(".envrc.local"));

    let report = env_json(&fixture, &["--apply", "docs-scratch-ignore"]);
    assert_eq!(item(&report, "docs-scratch-ignore")["state"], "wired");
    assert!(fixture.read(".gitignore").contains("/.docs-scratch/"));

    // A second run of each finds every line in place and writes nothing.
    let before = fixture.tree_digest();
    let again = env_json(
        &fixture,
        &["--apply", "docs-scratch-variable", "--value", "../my notes"],
    );
    assert_eq!(again["wrote"], serde_json::json!([]));
    let again = env_json(&fixture, &["--apply", "docs-scratch-ignore"]);
    assert_eq!(again["wrote"], serde_json::json!([]));
    assert_eq!(fixture.tree_digest(), before);

    // A new value replaces the assignment rather than adding a second one.
    env_json(
        &fixture,
        &[
            "--apply",
            "docs-scratch-variable",
            "--value",
            "../elsewhere",
        ],
    );
    assert_eq!(
        fixture.read(".envrc.local"),
        "export SDD_DOCS_SCRATCH='../elsewhere'\n"
    );
}

/// VERIFIES setup:env-writes-one-item-through-the-recorded-loader
#[test]
fn a_line_that_only_names_the_local_file_does_not_load_it() {
    let fixture = landed_with_scratch();
    fixture.write(
        ".envrc",
        "use flake\nwatch_file .envrc.local\nsource_env_if_exists .envrc.private # .envrc.local\n",
    );
    fixture.write(".envrc.local", "export SDD_DOCS_SCRATCH='../notes'\n");
    let report = env_json(&fixture, &[]);
    assert_eq!(item(&report, "docs-scratch-variable")["state"], "unwired");

    let report = env_json(
        &fixture,
        &["--apply", "docs-scratch-variable", "--value", "../notes"],
    );
    assert_eq!(item(&report, "docs-scratch-variable")["state"], "wired");
    assert_eq!(
        fixture.read(".envrc"),
        "use flake\nwatch_file .envrc.local\nsource_env_if_exists .envrc.private # .envrc.local\nsource_env_if_exists .envrc.local\n"
    );
}

/// VERIFIES setup:env-writes-one-item-through-the-recorded-loader
#[test]
fn a_source_command_ended_by_an_operator_is_not_written_twice() {
    let fixture = landed_with_scratch();
    let envrc = "source_env_if_exists .envrc.local; use flake\n";
    fixture.write(".envrc", envrc);
    let report = env_json(
        &fixture,
        &["--apply", "docs-scratch-variable", "--value", "../notes"],
    );
    assert_eq!(item(&report, "docs-scratch-variable")["state"], "wired");
    assert_eq!(fixture.read(".envrc"), envrc);

    let quoted = landed_with_scratch();
    quoted.write(".envrc", "source_env_if_exists \".envrc.local backup\"\n");
    quoted.write(".envrc.local", "export SDD_DOCS_SCRATCH='../notes'\n");
    let report = env_json(&quoted, &[]);
    assert_eq!(item(&report, "docs-scratch-variable")["state"], "unwired");
}

#[test]
fn a_dotenv_loader_takes_the_plain_assignment() {
    let fixture = Fixture::new();
    fixture.write(".env", "OTHER=1\n");
    env_json(
        &fixture,
        &["--apply", "docs-scratch-variable", "--value", ".notes"],
    );
    assert_eq!(fixture.read(".env"), "OTHER=1\nSDD_DOCS_SCRATCH='.notes'\n");
    assert!(fixture.read(".gitignore").contains(".env"));
}

#[test]
fn an_item_with_nowhere_to_go_refuses_and_writes_nothing() {
    let none = Fixture::new();
    let before = none.tree_digest();
    none.cmd()
        .args([
            "env",
            "--target",
            &none.target(),
            "--apply",
            "docs-scratch-variable",
            "--value",
            ".notes",
        ])
        .assert()
        .code(73);
    none.cmd()
        .args([
            "env",
            "--target",
            &none.target(),
            "--apply",
            "docs-scratch-ignore",
        ])
        .assert()
        .code(73);
    assert_eq!(none.tree_digest(), before);

    let unvalued = landed_with_scratch();
    unvalued.write(".envrc", "use flake\n");
    unvalued
        .cmd()
        .args([
            "env",
            "--target",
            &unvalued.target(),
            "--apply",
            "docs-scratch-variable",
        ])
        .assert()
        .code(64);
}

/// A directory-only pattern covers a scratch Git has never seen, as Git's
/// own ignore check reads it.
#[test]
fn a_directory_pattern_covers_a_scratch_that_does_not_exist_yet() {
    let fixture = landed_with_scratch();
    std::fs::remove_dir_all(fixture.path().join(".git")).unwrap();
    let git = support::git(fixture.path()).args(["init", "-q"]).status();
    if !git.is_ok_and(|status| status.success()) {
        return;
    }
    fixture.write(".gitignore", ".docs-scratch/\n");
    let report = env_json(&fixture, &[]);
    assert_eq!(item(&report, "docs-scratch-ignore")["state"], "wired");
}
