//! Integration: `sdd self-depend` reads, serves, moves, and cleans a
//! consumer's pin on this tool.

// Integration tests: assertion style is the point, so the production
// restrictions on unwrap/panic and string building do not apply here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect,
    reason = "the panic is this suite's failure signal, not control flow"
)]

mod support;

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use spec_driven_docs::self_depend::{SYNC_LINE, coordinates, slug};
use support::Fixture;

const THIS: &str = env!("CARGO_PKG_VERSION");

/// An invocation with the loop's own switches cleared, so a developer's
/// shell or a continuous-integration runner cannot switch it off.
fn sdd(fixture: &Fixture) -> Command {
    let mut cmd = fixture.cmd();
    cmd.env_remove("CI");
    cmd.env_remove("SDD_SELF_DEPEND_OFF");
    cmd
}

fn json(cmd: &mut Command) -> serde_json::Value {
    let output = cmd.assert().success().get_output().stdout.clone();
    serde_json::from_slice(&output).unwrap()
}

fn flake_pinned(version: &str) -> String {
    format!(
        "{{\n  inputs = {{\n    nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\";\n    docs-tool = {{\n      url = \"github:{}/v{version}\";\n      inputs.nixpkgs.follows = \"nixpkgs\";\n    }};\n  }};\n  outputs = {{ ... }}: {{ }};\n}}\n",
        slug()
    )
}

fn lock_at(rev: &str) -> String {
    let (owner, repo) = coordinates();
    format!(
        "{{\"nodes\":{{\"root\":{{\"inputs\":{{\"docs-tool\":\"docs-tool\"}}}},\"docs-tool\":{{\"locked\":{{\"rev\":\"{rev}\",\"type\":\"github\"}},\"original\":{{\"owner\":\"{owner}\",\"repo\":\"{repo}\",\"type\":\"github\"}}}}}},\"version\":7}}\n"
    )
}

/// A `nix` on `PATH` that either refreshes the lock or fails.
struct FakeNix {
    dir: tempfile::TempDir,
}

impl FakeNix {
    fn new(succeeds: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let script = if succeeds {
            format!(
                "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/calls\"\nprintf '%s' '{}' > flake.lock\nexit 0\n",
                lock_at("refreshed").trim_end()
            )
        } else {
            "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/calls\"\necho 'error: refused by the fake' >&2\nexit 1\n".to_string()
        };
        let path = dir.path().join("nix");
        std::fs::write(&path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self { dir }
    }

    fn path_env(&self) -> std::ffi::OsString {
        let mut paths = vec![self.dir.path().to_path_buf()];
        if let Some(held) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&held));
        }
        std::env::join_paths(paths).unwrap()
    }

    fn calls(&self) -> Option<String> {
        std::fs::read_to_string(self.dir.path().join("calls")).ok()
    }
}

fn digest(path: &Path) -> String {
    support::tree_digest(path)
}

// --- status -----------------------------------------------------------------

/// VERIFIES acquisition:status-reports-and-never-judges
#[test]
fn status_reports_every_manager_in_order_and_exits_zero_for_an_empty_target() {
    let fixture = Fixture::new();
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["schema"], "sdd.self-depend-status/1");
    assert_eq!(report["state"], "unwired");
    assert_eq!(report["wired"], serde_json::Value::Null);
    let managers: Vec<&str> = report["managers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["manager"].as_str().unwrap())
        .collect();
    assert_eq!(managers, ["flake", "mise", "asdf", "devbox"]);
    for entry in report["managers"].as_array().unwrap() {
        assert_eq!(entry["present"], "absent");
        assert_eq!(entry["pinned"], false);
    }
    // The shell loader is reported beside the list, because it pins nothing.
    assert_eq!(report["envrc"], "absent");
    assert_eq!(report["envrc_sync"], false);
    assert!(
        report["next"].as_array().unwrap()[0]
            .as_str()
            .unwrap()
            .contains("self-depend add")
    );
}

/// VERIFIES acquisition:status-reports-and-never-judges
#[test]
fn status_exits_zero_for_every_state_it_reports() {
    let fixture = Fixture::new();
    fixture.write("flake.nix", &flake_pinned("0.10.1"));
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["state"], "line-absent");
    assert_eq!(report["wired"], "flake");
    let flake = &report["managers"][0];
    assert_eq!(flake["present"], "present");
    assert_eq!(flake["file"], "flake.nix");
    assert_eq!(flake["version"], "v0.10.1");
    assert_eq!(flake["venue"], "flake");
    assert_eq!(flake["lock"], "absent");

    fixture.write(".envrc", &format!("use flake\n{SYNC_LINE}\n"));
    fixture.write("flake.lock", &lock_at("abc123"));
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["state"], "ready");
    assert_eq!(report["envrc_sync"], true);
    assert_eq!(report["managers"][0]["lock"], "present");
    assert_eq!(report["managers"][0]["locked_rev"], "abc123");
    assert_eq!(report["leftovers"].as_array().unwrap().len(), 0);

    fixture.write(".tool-versions", "spec-driven-docs 0.10.1\n");
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["state"], "ambiguous");
    assert_eq!(report["wired"], serde_json::Value::Null);

    // One manager alone, by request.
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--manager",
        "asdf",
        "--json",
    ]));
    assert_eq!(report["managers"].as_array().unwrap().len(), 1);
    assert_eq!(report["managers"][0]["manager"], "asdf");
    assert_eq!(report["managers"][0]["version"], "0.10.1");
}

#[test]
fn status_reports_the_pin_against_this_binary() {
    let fixture = Fixture::new();
    fixture.write(
        "mise.toml",
        &format!("[tools]\n\"cargo:spec-driven-docs\" = \"{THIS}\"\n"),
    );
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["managers"][1]["freshness"], "current");
    assert_eq!(report["managers"][1]["venue"], "crates");
    fixture.write(
        "mise.toml",
        "[tools]\n\"cargo:spec-driven-docs\" = \"0.1.0\"\n",
    );
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["managers"][1]["freshness"], "behind");
}

#[test]
fn status_names_the_switch_where_the_loop_is_off() {
    let fixture = Fixture::new();
    let report = json(sdd(&fixture).env("CI", "true").args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["off"], true);
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["off"], false);
}

// --- add --------------------------------------------------------------------

/// VERIFIES acquisition:the-tool-serves-and-does-not-edit
#[test]
fn add_writes_nothing_without_apply_and_seeds_only_what_the_target_lacks() {
    let fixture = Fixture::new();
    let before = digest(fixture.path());
    let report = json(sdd(&fixture).args([
        "self-depend",
        "add",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["schema"], "sdd.self-depend-add/1");
    assert_eq!(report["manager"], "flake");
    assert_eq!(report["venue"], "flake");
    assert_eq!(report["verdict"], "renders");
    assert_eq!(report["applied"], false);
    let seeds: Vec<&str> = report["seeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert_eq!(seeds, ["flake.nix", ".envrc"]);
    assert_eq!(digest(fixture.path()), before, "add wrote without --apply");

    json(sdd(&fixture).args([
        "self-depend",
        "add",
        "--target",
        &fixture.target(),
        "--apply",
        "--json",
    ]));
    let flake = fixture.read("flake.nix");
    assert!(flake.contains(&format!("github:{}/v{THIS}", slug())));
    assert_eq!(fixture.read(".envrc").matches(SYNC_LINE).count(), 1);
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["state"], "ready");
}

/// VERIFIES acquisition:the-tool-serves-and-does-not-edit
#[test]
fn add_leaves_a_manager_file_the_target_owns_byte_identical() {
    let fixture = Fixture::new();
    let owned = "{\n  inputs = { nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\"; };\n  outputs = { ... }: { };\n}\n";
    fixture.write("flake.nix", owned);
    fixture.write(".envrc", "use flake\n");
    let report = json(sdd(&fixture).args([
        "self-depend",
        "add",
        "--target",
        &fixture.target(),
        "--apply",
        "--json",
    ]));
    assert_eq!(report["seeds"].as_array().unwrap().len(), 0);
    assert_eq!(fixture.read("flake.nix"), owned);
    assert_eq!(fixture.read(".envrc"), "use flake\n");
    let fragments = report["fragments"].as_array().unwrap();
    assert_eq!(fragments.len(), 3);
    assert_eq!(fragments[0]["file"], "flake.nix");
    assert_eq!(fragments[0]["anchor"], "inputs = {");
    assert!(
        fragments[0]["text"]
            .as_str()
            .unwrap()
            .contains(&format!("github:{}/v{THIS}", slug()))
    );
    assert_eq!(fragments[2]["file"], ".envrc");
    assert_eq!(fragments[2]["text"].as_str().unwrap().trim(), SYNC_LINE);
}

/// VERIFIES acquisition:a-fragment-names-the-venue-form
#[test]
fn a_registry_fragment_names_the_crate_and_an_archive_fragment_names_the_binary() {
    let fixture = Fixture::new();
    let registry = json(sdd(&fixture).args([
        "self-depend",
        "add",
        "--target",
        &fixture.target(),
        "--manager",
        "mise",
        "--venue",
        "crates",
        "--tag",
        "v0.9.0",
        "--json",
    ]));
    let text = registry["fragments"][0]["text"].as_str().unwrap();
    assert_eq!(text.trim(), "\"cargo:spec-driven-docs\" = \"0.9.0\"");
    let archive = json(sdd(&fixture).args([
        "self-depend",
        "add",
        "--target",
        &fixture.target(),
        "--manager",
        "mise",
        "--venue",
        "github-release",
        "--json",
    ]));
    let text = archive["fragments"][0]["text"].as_str().unwrap();
    assert!(text.contains(&format!("\"ubi:{}\"", slug())));
    assert!(text.contains("exe = \"sdd\""));
    // The shell loader is served for every pair, seeded for the flake alone.
    assert_eq!(archive["fragments"][1]["file"], ".envrc");
    let seeds: Vec<&str> = archive["seeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert_eq!(seeds, ["mise.toml"]);
}

/// VERIFIES acquisition:one-target-runs-one-mechanism
#[test]
fn add_refuses_a_manager_other_than_the_one_naming_this_tool() {
    let fixture = Fixture::new();
    fixture.write(
        "mise.toml",
        "[tools]\n\"cargo:spec-driven-docs\" = \"0.10.1\"\n",
    );
    let before = digest(fixture.path());
    sdd(&fixture)
        .args([
            "self-depend",
            "add",
            "--target",
            &fixture.target(),
            "--manager",
            "flake",
            "--apply",
        ])
        .assert()
        .failure()
        .code(73)
        .stderr(predicate::str::contains("mise already pins this tool"));
    assert_eq!(digest(fixture.path()), before);
    let report = json(sdd(&fixture).args([
        "self-depend",
        "add",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["manager"], "mise");
}

/// VERIFIES acquisition:every-pair-carries-a-verdict
#[test]
fn a_manual_pair_is_refused_with_its_reason() {
    let fixture = Fixture::new();
    sdd(&fixture)
        .args([
            "self-depend",
            "add",
            "--target",
            &fixture.target(),
            "--manager",
            "asdf",
        ])
        .assert()
        .failure()
        .code(73)
        .stderr(predicate::str::contains("every pair is manual"));
    sdd(&fixture)
        .args([
            "self-depend",
            "add",
            "--target",
            &fixture.target(),
            "--manager",
            "flake",
            "--venue",
            "crates",
        ])
        .assert()
        .failure()
        .code(73)
        .stderr(predicate::str::contains("no package-set attribute"));
}

#[test]
fn add_refuses_to_guess_among_several_manager_files() {
    let fixture = Fixture::new();
    fixture.write("mise.toml", "[tools]\n");
    fixture.write(".tool-versions", "nodejs 20.0.0\n");
    sdd(&fixture)
        .args(["self-depend", "add", "--target", &fixture.target()])
        .assert()
        .failure()
        .code(64)
        .stderr(predicate::str::contains("--manager"));
}

// --- sync -------------------------------------------------------------------

/// VERIFIES acquisition:the-pin-moves-in-one-transaction
#[test]
fn a_flake_sync_moves_the_tag_and_the_lock_together() {
    let fixture = Fixture::new();
    fixture.write("flake.nix", &flake_pinned("0.10.1"));
    fixture.write("flake.lock", &lock_at("abc123"));
    let nix = FakeNix::new(true);
    let report = json(sdd(&fixture).env("PATH", nix.path_env()).args([
        "self-depend",
        "sync",
        "--target",
        &fixture.target(),
        "--caller",
        "operator",
        "--tag",
        "v0.10.2",
        "--apply",
        "--json",
    ]));
    assert_eq!(report["schema"], "sdd.self-depend-sync/1");
    assert_eq!(report["outcome"], "moved");
    assert_eq!(report["from"], "v0.10.1");
    assert_eq!(report["to"], "v0.10.2");
    assert_eq!(
        report["moved"]["files"],
        serde_json::json!(["flake.nix", "flake.lock"])
    );
    assert!(
        fixture
            .read("flake.nix")
            .contains(&format!("github:{}/v0.10.2\"", slug()))
    );
    assert!(fixture.read("flake.lock").contains("refreshed"));
    assert_eq!(nix.calls().unwrap().trim(), "flake update docs-tool");
}

/// VERIFIES acquisition:the-pin-moves-in-one-transaction
#[test]
fn a_failed_lock_refresh_puts_both_files_back() {
    let fixture = Fixture::new();
    fixture.write("flake.nix", &flake_pinned("0.10.1"));
    fixture.write("flake.lock", &lock_at("abc123"));
    let before = digest(fixture.path());
    let nix = FakeNix::new(false);
    sdd(&fixture)
        .env("PATH", nix.path_env())
        .args([
            "self-depend",
            "sync",
            "--target",
            &fixture.target(),
            "--caller",
            "operator",
            "--tag",
            "v0.10.2",
            "--apply",
        ])
        .assert()
        .failure()
        .code(73)
        .stderr(predicate::str::contains("both files were put back"));
    assert_eq!(digest(fixture.path()), before);
    assert!(nix.calls().is_some(), "the fake nix was not called");
}

/// VERIFIES acquisition:the-pin-moves-in-one-transaction
#[test]
fn a_flake_with_no_lock_yet_is_left_without_one_on_failure() {
    let fixture = Fixture::new();
    fixture.write("flake.nix", &flake_pinned("0.10.1"));
    let nix = FakeNix::new(false);
    sdd(&fixture)
        .env("PATH", nix.path_env())
        .args([
            "self-depend",
            "sync",
            "--target",
            &fixture.target(),
            "--caller",
            "operator",
            "--tag",
            "v0.10.2",
            "--apply",
        ])
        .assert()
        .failure()
        .code(73);
    assert!(!fixture.path().join("flake.lock").exists());
    assert_eq!(fixture.read("flake.nix"), flake_pinned("0.10.1"));
}

/// VERIFIES acquisition:the-sync-leaves-a-diff-nobody-committed
#[test]
fn a_one_fact_manager_moves_in_place_and_names_from_and_to_in_its_form() {
    let fixture = Fixture::new();
    fixture.write(".tool-versions", "nodejs 20.0.0\nspec-driven-docs 0.10.1\n");
    fixture.write("README.md", "# untouched\n");
    let report = json(sdd(&fixture).args([
        "self-depend",
        "sync",
        "--target",
        &fixture.target(),
        "--caller",
        "operator",
        "--tag",
        "0.10.2",
        "--apply",
        "--json",
    ]));
    assert_eq!(report["outcome"], "moved");
    assert_eq!(report["manager"], "asdf");
    assert_eq!(report["from"], "0.10.1");
    assert_eq!(report["to"], "0.10.2");
    assert_eq!(
        report["moved"]["files"],
        serde_json::json!([".tool-versions"])
    );
    assert_eq!(
        fixture.read(".tool-versions"),
        "nodejs 20.0.0\nspec-driven-docs 0.10.2\n"
    );
    assert_eq!(fixture.read("README.md"), "# untouched\n");
    assert!(
        !fixture.path().join(".git/index").exists(),
        "the sync touched git"
    );
}

#[test]
fn a_sync_without_apply_reports_the_bump_and_moves_nothing() {
    let fixture = Fixture::new();
    fixture.write(
        "devbox.json",
        &format!(
            "{{ \"packages\": [\"github:{}/v0.10.1#default\"] }}\n",
            slug()
        ),
    );
    let before = digest(fixture.path());
    let report = json(sdd(&fixture).args([
        "self-depend",
        "sync",
        "--target",
        &fixture.target(),
        "--caller",
        "operator",
        "--tag",
        "v0.10.2",
        "--json",
    ]));
    assert_eq!(report["outcome"], "pending");
    assert_eq!(digest(fixture.path()), before);
    let report = json(sdd(&fixture).args([
        "self-depend",
        "sync",
        "--target",
        &fixture.target(),
        "--caller",
        "operator",
        "--tag",
        "v0.10.1",
        "--json",
    ]));
    assert_eq!(report["outcome"], "current");
}

/// VERIFIES acquisition:one-target-runs-one-mechanism
#[test]
fn a_target_with_one_wired_manager_chooses_it_and_several_refuse_without_manager() {
    let fixture = Fixture::new();
    fixture.write(
        "mise.toml",
        "[tools]\n\"cargo:spec-driven-docs\" = \"0.10.1\"\n",
    );
    let report = json(sdd(&fixture).args([
        "self-depend",
        "sync",
        "--target",
        &fixture.target(),
        "--caller",
        "operator",
        "--tag",
        "0.10.1",
        "--json",
    ]));
    assert_eq!(report["manager"], "mise");
    fixture.write(".tool-versions", "spec-driven-docs 0.10.1\n");
    sdd(&fixture)
        .args([
            "self-depend",
            "sync",
            "--target",
            &fixture.target(),
            "--caller",
            "operator",
            "--tag",
            "0.10.1",
        ])
        .assert()
        .failure()
        .code(64)
        .stderr(predicate::str::contains("--manager"));
    let report = json(sdd(&fixture).args([
        "self-depend",
        "sync",
        "--target",
        &fixture.target(),
        "--caller",
        "operator",
        "--manager",
        "asdf",
        "--tag",
        "0.10.1",
        "--json",
    ]));
    assert_eq!(report["manager"], "asdf");
}

/// VERIFIES acquisition:the-operator-caller-reports-every-outcome
#[test]
fn the_operator_caller_reports_each_outcome_with_its_own_exit_code() {
    // No wire: usage.
    let unwired = Fixture::new();
    sdd(&unwired)
        .args([
            "self-depend",
            "sync",
            "--target",
            &unwired.target(),
            "--caller",
            "operator",
        ])
        .assert()
        .failure()
        .code(64)
        .stderr(predicate::str::contains("self-depend add"));
    // The latest release cannot be read: the fixture forbids the network.
    let offline = Fixture::new();
    offline.write(".tool-versions", "spec-driven-docs 0.10.1\n");
    sdd(&offline)
        .args([
            "self-depend",
            "sync",
            "--target",
            &offline.target(),
            "--caller",
            "operator",
        ])
        .assert()
        .failure()
        .code(70)
        .stderr(predicate::str::contains("latest release could not be read"));
    // A transaction that was put back: refused.
    let failed = Fixture::new();
    failed.write("flake.nix", &flake_pinned("0.10.1"));
    let nix = FakeNix::new(false);
    sdd(&failed)
        .env("PATH", nix.path_env())
        .args([
            "self-depend",
            "sync",
            "--target",
            &failed.target(),
            "--caller",
            "operator",
            "--tag",
            "v0.10.2",
            "--apply",
        ])
        .assert()
        .failure()
        .code(73);
    // Success: zero, with the outcome on stdout.
    let moved = Fixture::new();
    moved.write(".tool-versions", "spec-driven-docs 0.10.1\n");
    sdd(&moved)
        .args([
            "self-depend",
            "sync",
            "--target",
            &moved.target(),
            "--caller",
            "operator",
            "--tag",
            "0.10.2",
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("asdf moved from 0.10.1 to 0.10.2"));
}

/// VERIFIES acquisition:the-shell-entry-caller-is-rate-limited-and-silent
#[test]
fn the_shell_entry_caller_exits_zero_silently_on_every_outcome() {
    // No wire.
    let unwired = Fixture::new();
    sdd(&unwired)
        .args([
            "self-depend",
            "sync",
            "--target",
            &unwired.target(),
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
    // An unreachable network.
    let offline = Fixture::new();
    offline.write(".tool-versions", "spec-driven-docs 0.10.1\n");
    sdd(&offline)
        .args([
            "self-depend",
            "sync",
            "--target",
            &offline.target(),
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
    // A failed sync.
    let failed = Fixture::new();
    failed.write("flake.nix", &flake_pinned("0.10.1"));
    let before = digest(failed.path());
    let nix = FakeNix::new(false);
    sdd(&failed)
        .env("PATH", nix.path_env())
        .args([
            "self-depend",
            "sync",
            "--target",
            &failed.target(),
            "--tag",
            "v0.10.2",
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
    assert_eq!(digest(failed.path()), before);
    // A successful sync.
    let moved = Fixture::new();
    moved.write("flake.nix", &flake_pinned("0.10.1"));
    let nix = FakeNix::new(true);
    sdd(&moved)
        .env("PATH", nix.path_env())
        .args([
            "self-depend",
            "sync",
            "--target",
            &moved.target(),
            "--tag",
            "v0.10.2",
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
    assert!(moved.read("flake.nix").contains("v0.10.2"));
}

/// VERIFIES acquisition:the-shell-entry-caller-is-rate-limited-and-silent
#[test]
fn the_shell_entry_caller_attempts_at_most_once_a_day_per_checkout() {
    let fixture = Fixture::new();
    fixture.write(".tool-versions", "spec-driven-docs 0.10.1\n");
    // The first attempt reaches for the latest release, which the fixture
    // forbids; the day is stamped all the same.
    let first = json(sdd(&fixture).args([
        "self-depend",
        "sync",
        "--target",
        &fixture.target(),
        "--apply",
        "--json",
    ]));
    assert_eq!(first["outcome"], "failed");
    let stamps: Vec<_> =
        walkdir::WalkDir::new(fixture.state_root().join("spec-driven-docs/self-depend"))
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .collect();
    assert_eq!(stamps.len(), 1, "one stamp per checkout");
    let second = json(sdd(&fixture).args([
        "self-depend",
        "sync",
        "--target",
        &fixture.target(),
        "--apply",
        "--json",
    ]));
    assert_eq!(second["outcome"], "stamped");
    // Another checkout of the same project keeps its own stamp.
    let other = Fixture::new();
    other.write(".tool-versions", "spec-driven-docs 0.10.1\n");
    let mut cmd = sdd(&other);
    cmd.env(
        "XDG_STATE_HOME",
        fixture.state_root().parent().unwrap().join("state"),
    );
    let third = json(cmd.args([
        "self-depend",
        "sync",
        "--target",
        &other.target(),
        "--apply",
        "--json",
    ]));
    assert_eq!(third["outcome"], "failed");
    // The operator caller is not stamped.
    let report = json(sdd(&fixture).args([
        "self-depend",
        "sync",
        "--target",
        &fixture.target(),
        "--caller",
        "operator",
        "--tag",
        "0.10.1",
        "--json",
    ]));
    assert_eq!(report["outcome"], "current");
}

/// VERIFIES acquisition:the-shell-entry-caller-is-rate-limited-and-silent
#[test]
fn the_loop_is_off_under_ci_and_under_the_operator_variable() {
    let fixture = Fixture::new();
    fixture.write(".tool-versions", "spec-driven-docs 0.10.1\n");
    for (name, value) in [("CI", "1"), ("SDD_SELF_DEPEND_OFF", "1")] {
        let report = json(sdd(&fixture).env(name, value).args([
            "self-depend",
            "sync",
            "--target",
            &fixture.target(),
            "--caller",
            "operator",
            "--tag",
            "0.10.2",
            "--apply",
            "--json",
        ]));
        assert_eq!(report["outcome"], "off");
    }
    assert_eq!(fixture.read(".tool-versions"), "spec-driven-docs 0.10.1\n");
    assert!(
        !fixture
            .state_root()
            .join("spec-driven-docs/self-depend")
            .exists(),
        "a switched-off run stamped"
    );
}

// --- clean ------------------------------------------------------------------

/// VERIFIES acquisition:clean-removes-only-a-named-leftover
#[test]
fn clean_names_a_leftover_and_what_it_keeps_and_removes_only_with_apply() {
    let fixture = Fixture::new();
    fixture.write("flake.nix", &flake_pinned("0.10.1"));
    fixture.write("flake.lock", &lock_at("abc123"));
    fixture.write(".envrc", &format!("use flake\n{SYNC_LINE}\n"));
    fixture.write("scripts/bump.sh", "#!/bin/sh\n");
    let before = digest(fixture.path());
    let report = json(sdd(&fixture).args([
        "self-depend",
        "clean",
        "--target",
        &fixture.target(),
        "--also",
        "scripts/bump.sh",
        "--json",
    ]));
    assert_eq!(report["schema"], "sdd.self-depend-clean/1");
    assert_eq!(report["leftovers"][0]["file"], "scripts/bump.sh");
    assert_eq!(report["removed"].as_array().unwrap().len(), 0);
    let kept: Vec<(&str, Option<&str>)> = report["kept"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| (k["file"].as_str().unwrap(), k["line"].as_str()))
        .collect();
    assert!(kept.contains(&("flake.nix", Some("line 5"))));
    assert!(kept.contains(&("flake.lock", None)));
    assert!(kept.contains(&(".envrc", Some(SYNC_LINE))));
    assert_eq!(digest(fixture.path()), before);

    let report = json(sdd(&fixture).args([
        "self-depend",
        "clean",
        "--target",
        &fixture.target(),
        "--also",
        "scripts/bump.sh",
        "--apply",
        "--json",
    ]));
    assert_eq!(report["removed"], serde_json::json!(["scripts/bump.sh"]));
    assert!(!fixture.path().join("scripts/bump.sh").exists());
    assert_eq!(fixture.read(".envrc"), format!("use flake\n{SYNC_LINE}\n"));
    assert_eq!(fixture.read("flake.nix"), flake_pinned("0.10.1"));
}

#[test]
fn clean_refuses_a_path_outside_the_target() {
    let fixture = Fixture::new();
    sdd(&fixture)
        .args([
            "self-depend",
            "clean",
            "--target",
            &fixture.target(),
            "--also",
            "../elsewhere",
        ])
        .assert()
        .failure()
        .code(64);
}
