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
use spec_driven_docs::self_depend::{CHECK_LINE, OLD_SYNC_LINE, coordinates, slug};
use support::Fixture;

const THIS: &str = env!("CARGO_PKG_VERSION");

/// An invocation with the loop's own switches cleared, so a developer's
/// shell or a continuous-integration runner cannot switch it off.
fn sdd(fixture: &Fixture) -> Command {
    let mut cmd = fixture.cmd();
    cmd.env_remove("CI");
    cmd.env_remove("SDD_SELF_DEPEND_OFF");
    cmd.env_remove("DIRENV_IN_ENVRC");
    cmd.env_remove("SDD_HTTP_ORIGIN");
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
    assert_eq!(report["schema"], "sdd.self-depend-status/2");
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
    assert_eq!(report["envrc_check"], false);
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

    fixture.write(".envrc", &format!("use flake\n{CHECK_LINE}\n"));
    fixture.write("flake.lock", &lock_at("abc123"));
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["state"], "ready");
    assert_eq!(report["envrc_check"], true);
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

/// The setup skill offers the wire from `wired` and `envrc_check` alone, so
/// the three states it branches on are held here: nothing wired, wired
/// with the line absent, and wired with the line landed.
///
/// VERIFIES acquisition:the-setup-path-offers-the-wire
#[test]
fn the_setup_offer_follows_the_wiring_state() {
    let fixture = Fixture::new();
    let status = |fixture: &Fixture| {
        json(sdd(fixture).args([
            "self-depend",
            "status",
            "--target",
            &fixture.target(),
            "--json",
        ]))
    };

    // No wired manager: nothing to keep fresh, so no offer.
    let report = status(&fixture);
    assert_eq!(report["state"], "unwired");
    assert_eq!(report["wired"], serde_json::Value::Null);
    assert_eq!(report["envrc_check"], false);

    // Wired with the line absent: the one state that produces the offer,
    // and the next line names the verb that serves the line.
    fixture.write(
        "mise.toml",
        "[tools]\n\"cargo:spec-driven-docs\" = \"0.10.1\"\n",
    );
    let report = status(&fixture);
    assert_eq!(report["state"], "line-absent");
    assert_eq!(report["wired"], "mise");
    assert_eq!(report["envrc_check"], false);
    let next = report["next"][0].as_str().unwrap();
    assert!(next.contains(CHECK_LINE), "{next}");
    assert!(next.contains("sdd self-depend add"), "{next}");

    // The line already landed: no offer.
    fixture.write(".envrc", &format!("{CHECK_LINE}\n"));
    let report = status(&fixture);
    assert_eq!(report["state"], "ready");
    assert_eq!(report["wired"], "mise");
    assert_eq!(report["envrc_check"], true);
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
    assert_eq!(fixture.read(".envrc").matches(CHECK_LINE).count(), 1);
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
    assert_eq!(fragments[2]["text"].as_str().unwrap().trim(), CHECK_LINE);
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
    assert_eq!(report["schema"], "sdd.self-depend-sync/2");
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

// --- check ------------------------------------------------------------------

/// A registry index on the loopback that counts the requests it answers.
///
/// It serves one stable version, or answers 404 where it serves none, and
/// waits `delay` before each answer so two checks can overlap.
struct FakeIndex {
    origin: String,
    hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl FakeIndex {
    fn serving(version: Option<&str>) -> Self {
        Self::slow(version, std::time::Duration::ZERO)
    }

    fn slow(version: Option<&str>, delay: std::time::Duration) -> Self {
        use std::io::{Read, Write};
        use std::sync::atomic::Ordering;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = hits.clone();
        let body = version.map(|version| {
            format!("{{\"name\":\"spec-driven-docs\",\"vers\":\"{version}\",\"yanked\":false}}\n")
        });
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut seen = Vec::new();
                let mut buf = [0_u8; 1024];
                while !seen.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => seen.extend_from_slice(&buf[..n]),
                    }
                }
                counter.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(delay);
                let answer = body.as_ref().map_or_else(
                    || "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n".to_string(),
                    |body| {
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                            body.len()
                        )
                    },
                );
                let _ = stream.write_all(answer.as_bytes());
            }
        });
        Self { origin, hits }
    }

    fn hits(&self) -> usize {
        self.hits.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// An invocation that reads `index` in place of the public registry.
fn against(fixture: &Fixture, index: &FakeIndex) -> Command {
    let mut cmd = sdd(fixture);
    cmd.env_remove("SDD_OFFLINE");
    cmd.env("SDD_HTTP_ORIGIN", &index.origin);
    cmd
}

/// `sdd self-depend check` on the fixture, against `index`.
fn check(fixture: &Fixture, index: &FakeIndex) -> Command {
    let mut cmd = against(fixture, index);
    cmd.args(["self-depend", "check", "--target", &fixture.target()]);
    cmd
}

/// The cache file the check keeps for this fixture's checkout.
fn cache_file(fixture: &Fixture) -> std::path::PathBuf {
    let root = fixture.cache_root().join("spec-driven-docs");
    let path = spec_driven_docs::self_depend::cache::path(
        camino::Utf8Path::from_path(&root).unwrap(),
        camino::Utf8Path::new(&fixture.target()),
    );
    path.into_std_path_buf()
}

fn ago(minutes: i64) -> String {
    jiff::Timestamp::now()
        .checked_sub(jiff::SignedDuration::from_mins(minutes))
        .unwrap()
        .to_string()
}

const DAY: i64 = 24 * 60;

/// Seed the cache with the given fields.
fn seed_cache(
    fixture: &Fixture,
    attempt: Option<i64>,
    success: Option<i64>,
    latest: Option<&str>,
    error: Option<&str>,
) {
    let path = cache_file(fixture);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let entry = serde_json::json!({
        "last_attempt": attempt.map(ago),
        "last_success": success.map(ago),
        "latest": latest,
        "error": error,
    });
    std::fs::write(path, entry.to_string()).unwrap();
}

fn read_cache(fixture: &Fixture) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(cache_file(fixture)).unwrap()).unwrap()
}

/// A checkout that pins this tool at 0.10.1 through asdf.
fn pinned() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(".tool-versions", "spec-driven-docs 0.10.1\n");
    fixture
}

/// VERIFIES acquisition:the-check-refreshes-weekly-and-retries-hourly
#[test]
fn a_fresh_cache_reads_no_network() {
    let fixture = pinned();
    let index = FakeIndex::serving(Some("0.10.1"));
    seed_cache(&fixture, Some(6 * DAY), Some(6 * DAY), Some("0.10.1"), None);
    check(&fixture, &index).assert().success();
    assert_eq!(index.hits(), 0, "a six-day-old success read the network");
    seed_cache(
        &fixture,
        Some(7 * DAY + 1),
        Some(7 * DAY + 1),
        Some("0.10.1"),
        None,
    );
    check(&fixture, &index).assert().success();
    assert_eq!(index.hits(), 1, "a seven-day-old success did not refresh");
}

/// VERIFIES acquisition:the-check-warns-and-writes-only-its-cache
#[test]
fn an_update_warns_from_the_cache() {
    let fixture = pinned();
    let index = FakeIndex::serving(Some("0.11.0"));
    seed_cache(&fixture, Some(60), Some(60), Some("0.11.0"), None);
    for _ in 0..2 {
        check(&fixture, &index)
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains(
                "sdd 0.11.0 is out, this project pins 0.10.1. To move: sdd self-depend sync --apply",
            ));
    }
    assert_eq!(index.hits(), 0);
}

/// VERIFIES acquisition:the-check-warns-and-writes-only-its-cache
#[test]
fn check_is_silent_when_current() {
    let fixture = pinned();
    let index = FakeIndex::serving(Some("0.10.1"));
    check(&fixture, &index)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
    assert_eq!(index.hits(), 1);
    let cache = read_cache(&fixture);
    assert_eq!(cache["latest"], "0.10.1");
    assert_eq!(cache["error"], serde_json::Value::Null);
    assert_eq!(cache["last_attempt"], cache["last_success"]);
}

/// VERIFIES acquisition:the-check-warns-and-writes-only-its-cache
#[test]
fn a_failed_read_warns_with_its_reason_and_exits_zero() {
    let fixture = pinned();
    let index = FakeIndex::serving(None);
    check(&fixture, &index)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "sdd could not check for a newer release: ",
        ))
        .stderr(predicate::str::contains("404"))
        .stderr(predicate::str::contains(". Next try after "));
    assert!(read_cache(&fixture)["error"].is_string());
}

/// VERIFIES acquisition:the-check-refreshes-weekly-and-retries-hourly
#[test]
fn a_failure_after_an_overdue_success_waits_one_hour() {
    let fixture = pinned();
    let index = FakeIndex::serving(Some("0.10.1"));
    seed_cache(
        &fixture,
        Some(1),
        Some(8 * DAY),
        Some("0.10.1"),
        Some("down"),
    );
    check(&fixture, &index)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "could not check for a newer release: down",
        ));
    assert_eq!(index.hits(), 0, "a failure one minute old read the network");
    seed_cache(
        &fixture,
        Some(61),
        Some(8 * DAY),
        Some("0.10.1"),
        Some("down"),
    );
    check(&fixture, &index)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
    assert_eq!(index.hits(), 1);
}

/// VERIFIES acquisition:the-check-refreshes-weekly-and-retries-hourly
#[test]
fn a_failed_first_check_retries_after_one_hour() {
    let fixture = pinned();
    let index = FakeIndex::serving(None);
    check(&fixture, &index).assert().success();
    assert_eq!(index.hits(), 1);
    check(&fixture, &index).assert().success();
    assert_eq!(index.hits(), 1, "a fresh failure was retried at once");
    let error = read_cache(&fixture)["error"].as_str().unwrap().to_string();
    seed_cache(&fixture, Some(61), None, None, Some(&error));
    check(&fixture, &index).assert().success();
    assert_eq!(index.hits(), 2);
}

/// sdd's pin move keeps no transaction files, so what must survive the
/// stamp cleanup is every other file of the state root: another
/// checkout's skill lock and receipt, a staged landing, and any file in the
/// stamp folder that is not a stamp.
///
/// VERIFIES acquisition:clean-swaps-the-retired-line
#[test]
fn the_stamp_cleanup_keeps_locks_and_transactions() {
    let fixture = pinned();
    let state = fixture.state_root().join("spec-driven-docs");
    let write = |relative: &str| {
        let path = state.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "held\n").unwrap();
        path
    };
    let stamps = [
        write("self-depend/0123456789abcdef.stamp"),
        write("self-depend/fedcba9876543210.stamp"),
    ];
    let survivors = [
        write("skills.lock"),
        write("skills.json"),
        write("stage/candidate/manifest.json"),
        write("self-depend/notes.txt"),
    ];
    seed_cache(&fixture, Some(60), Some(60), Some("0.10.1"), None);
    let preview = json(sdd(&fixture).args([
        "self-depend",
        "clean",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(preview["stamps"].as_array().unwrap().len(), 2);
    assert!(
        stamps.iter().all(|stamp| stamp.exists()),
        "a preview removed a stamp"
    );
    let report = json(sdd(&fixture).args([
        "self-depend",
        "clean",
        "--target",
        &fixture.target(),
        "--apply",
        "--json",
    ]));
    assert_eq!(report["stamps"].as_array().unwrap().len(), 2);
    for stamp in &stamps {
        assert!(!stamp.exists(), "{} survived", stamp.display());
    }
    for survivor in &survivors {
        assert!(survivor.exists(), "{} was removed", survivor.display());
    }
    assert!(cache_file(&fixture).exists(), "the clean removed the cache");
}

/// VERIFIES acquisition:the-check-refreshes-weekly-and-retries-hourly
#[test]
fn cached_error_warnings_do_not_postpone_the_retry() {
    let fixture = pinned();
    let index = FakeIndex::serving(None);
    seed_cache(&fixture, Some(58), None, None, Some("down"));
    let before = read_cache(&fixture)["last_attempt"].clone();
    for _ in 0..3 {
        check(&fixture, &index)
            .assert()
            .success()
            .stderr(predicate::str::contains(
                "could not check for a newer release: down",
            ));
    }
    assert_eq!(
        index.hits(),
        0,
        "an entry 58 minutes after the failure read the network"
    );
    assert_eq!(
        read_cache(&fixture)["last_attempt"],
        before,
        "a warning moved the attempt time"
    );
    seed_cache(&fixture, Some(61), None, None, Some("down"));
    check(&fixture, &index).assert().success();
    assert_eq!(index.hits(), 1);
}

/// VERIFIES acquisition:the-check-warns-and-writes-only-its-cache
#[test]
fn an_unwritable_cache_warns_and_reads_no_network() {
    let fixture = pinned();
    let index = FakeIndex::serving(Some("0.11.0"));
    let blocker = fixture.cache_root();
    std::fs::create_dir_all(blocker.parent().unwrap()).unwrap();
    std::fs::write(&blocker, "a file where the cache root goes\n").unwrap();
    check(&fixture, &index)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "sdd cannot record its version check at ",
        ));
    assert_eq!(index.hits(), 0);
}

/// VERIFIES acquisition:the-check-refreshes-weekly-and-retries-hourly
#[test]
fn overlapping_checks_read_the_network_once() {
    let fixture = pinned();
    let index = FakeIndex::slow(Some("0.10.1"), std::time::Duration::from_millis(1500));
    std::thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| check(&fixture, &index).assert().success());
        }
    });
    assert_eq!(index.hits(), 1);
    let lock = std::fs::File::open(cache_file(&fixture).with_extension("lock")).unwrap();
    assert!(lock.try_lock().is_ok(), "the lock outlived the refresh");
}

/// VERIFIES acquisition:the-check-warns-and-writes-only-its-cache
#[test]
fn check_writes_no_file_but_its_cache() {
    let fixture = Fixture::new();
    fixture.write("flake.nix", &flake_pinned("0.10.1"));
    fixture.write("flake.lock", &lock_at("abc123"));
    fixture.write(".envrc", &format!("use flake\n{CHECK_LINE}\n"));
    let before = digest(fixture.path());
    let index = FakeIndex::serving(Some("0.11.0"));
    check(&fixture, &index)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "sdd 0.11.0 is out, this project pins 0.10.1",
        ));
    assert_eq!(
        digest(fixture.path()),
        before,
        "the check wrote into the project"
    );
    assert!(
        !fixture.state_root().exists(),
        "the check wrote under the state root"
    );
    let mut written: Vec<_> = walkdir::WalkDir::new(fixture.cache_root())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .collect();
    written.sort();
    // The cache, and the lock file beside it, which stays in place.
    assert_eq!(
        written,
        [
            cache_file(&fixture),
            cache_file(&fixture).with_extension("lock")
        ]
    );
}

/// VERIFIES acquisition:sync-is-the-operators-act
#[test]
fn sync_apply_from_a_shell_moves_the_pin() {
    let fixture = pinned();
    let index = FakeIndex::serving(Some("0.10.2"));
    against(&fixture, &index)
        .args([
            "self-depend",
            "sync",
            "--target",
            &fixture.target(),
            "--apply",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("asdf moved from 0.10.1 to 0.10.2"));
    assert_eq!(fixture.read(".tool-versions"), "spec-driven-docs 0.10.2\n");
}

/// VERIFIES acquisition:the-off-switch-stops-the-check-alone
#[test]
fn the_off_switch_stops_check_but_not_an_operator_sync() {
    for (name, value) in [("CI", "1"), ("SDD_SELF_DEPEND_OFF", "1")] {
        let fixture = pinned();
        let index = FakeIndex::serving(Some("0.10.2"));
        check(&fixture, &index)
            .env(name, value)
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty());
        assert_eq!(index.hits(), 0, "{name} did not stop the check");
        assert!(
            !cache_file(&fixture).exists(),
            "{name}: the check wrote its cache"
        );
        let report = json(against(&fixture, &index).env(name, value).args([
            "self-depend",
            "sync",
            "--target",
            &fixture.target(),
            "--apply",
            "--json",
        ]));
        assert_eq!(
            report["outcome"], "moved",
            "{name} stopped the operator's sync"
        );
        assert_eq!(fixture.read(".tool-versions"), "spec-driven-docs 0.10.2\n");
    }
}

/// VERIFIES acquisition:sync-is-the-operators-act
#[test]
fn sync_inside_an_envrc_moves_nothing() {
    let fixture = pinned();
    let before = digest(fixture.path());
    sdd(&fixture)
        .env("DIRENV_IN_ENVRC", "1")
        .args([
            "self-depend",
            "sync",
            "--target",
            &fixture.target(),
            "--tag",
            "0.10.2",
            "--apply",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "moved nothing: DIRENV_IN_ENVRC is set",
        ))
        .stderr(predicate::str::contains(
            "nix develop -c sdd self-depend sync --apply",
        ))
        .stderr(predicate::str::contains(CHECK_LINE));
    let output = sdd(&fixture)
        .env("DIRENV_IN_ENVRC", "1")
        .args([
            "self-depend",
            "sync",
            "--target",
            &fixture.target(),
            "--tag",
            "0.10.2",
            "--apply",
            "--json",
        ])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["outcome"], "inside-envrc");
    assert_eq!(digest(fixture.path()), before);
    // The retired line still loads the shell, because it ends in `|| true`.
    let loaded = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("{OLD_SYNC_LINE}; echo loaded"))
        .env("PATH", sdd_on_path())
        .env("DIRENV_IN_ENVRC", "1")
        .env("XDG_CACHE_HOME", fixture.cache_root())
        .env("XDG_STATE_HOME", fixture.state_root())
        .env_remove("SDD_SELF_DEPEND_OFF")
        .current_dir(fixture.path())
        .output()
        .unwrap();
    assert!(loaded.status.success());
    assert_eq!(String::from_utf8_lossy(&loaded.stdout).trim(), "loaded");
    assert_eq!(digest(fixture.path()), before);
}

/// A `PATH` whose first entry holds the `sdd` this suite built.
fn sdd_on_path() -> std::ffi::OsString {
    let bin = assert_cmd::cargo::cargo_bin("sdd");
    let mut paths = vec![bin.parent().unwrap().to_path_buf()];
    if let Some(held) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&held));
    }
    std::env::join_paths(paths).unwrap()
}

/// `direnv exec` exports `DIRENV_IN_ENVRC=1` too, so a sync under it is
/// refused the same way.
///
/// VERIFIES acquisition:sync-is-the-operators-act
#[test]
fn sync_under_direnv_exec_exits_one_and_moves_nothing() {
    let fixture = Fixture::new();
    fixture.write("flake.nix", &flake_pinned("0.10.1"));
    fixture.write("flake.lock", &lock_at("abc123"));
    let nix = FakeNix::new(true);
    sdd(&fixture)
        .env("DIRENV_IN_ENVRC", "1")
        .env("PATH", nix.path_env())
        .args([
            "self-depend",
            "sync",
            "--target",
            &fixture.target(),
            "--tag",
            "v0.10.2",
            "--apply",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("`direnv exec`"));
    assert_eq!(fixture.read("flake.nix"), flake_pinned("0.10.1"));
    assert_eq!(fixture.read("flake.lock"), lock_at("abc123"));
    assert!(nix.calls().is_none(), "the lock refresh ran");
}

/// VERIFIES acquisition:the-check-refreshes-weekly-and-retries-hourly
#[test]
fn a_held_lock_makes_check_read_the_cache_only() {
    let fixture = pinned();
    let index = FakeIndex::serving(Some("0.10.1"));
    seed_cache(&fixture, Some(8 * DAY), Some(8 * DAY), Some("0.11.0"), None);
    // Another refresh holds the lock: this test process holds it here.
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(cache_file(&fixture).with_extension("lock"))
        .unwrap();
    lock.lock().unwrap();
    check(&fixture, &index)
        .assert()
        .success()
        .stderr(predicate::str::contains("sdd 0.11.0 is out"));
    assert_eq!(index.hits(), 0, "a check read the network past a held lock");
    drop(lock);
}

/// A process that died holding the lock released it with its death, so
/// the lock file it left blocks nothing.
///
/// VERIFIES acquisition:the-check-refreshes-weekly-and-retries-hourly
#[test]
fn a_lock_left_by_a_dead_process_does_not_block() {
    let fixture = pinned();
    let index = FakeIndex::serving(Some("0.10.1"));
    let lock = cache_file(&fixture).with_extension("lock");
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    std::fs::write(&lock, "").unwrap();
    check(&fixture, &index).assert().success();
    assert_eq!(index.hits(), 1, "an unheld lock file blocked the refresh");
    assert_eq!(read_cache(&fixture)["latest"], "0.10.1");
    assert!(lock.exists(), "the lock file was removed");
}

/// VERIFIES acquisition:sync-is-the-operators-act
#[test]
fn the_envrc_caller_is_a_usage_error() {
    let fixture = pinned();
    let before = digest(fixture.path());
    sdd(&fixture)
        .args([
            "self-depend",
            "sync",
            "--target",
            &fixture.target(),
            "--caller",
            "envrc",
            "--tag",
            "0.10.2",
            "--apply",
        ])
        .assert()
        .failure()
        .code(64)
        .stderr(predicate::str::contains("sdd self-depend check"));
    assert_eq!(digest(fixture.path()), before);
}

/// VERIFIES acquisition:the-check-warns-and-writes-only-its-cache
#[test]
fn the_seeded_envrc_line_is_the_check() {
    let fixture = Fixture::new();
    let report = json(sdd(&fixture).args([
        "self-depend",
        "add",
        "--target",
        &fixture.target(),
        "--apply",
        "--json",
    ]));
    let fragments = report["fragments"].as_array().unwrap();
    let envrc = fragments.iter().find(|f| f["file"] == ".envrc").unwrap();
    assert_eq!(envrc["text"].as_str().unwrap().trim(), CHECK_LINE);
    let seeded = fixture.read(".envrc");
    assert_eq!(seeded.matches(CHECK_LINE).count(), 1);
    assert!(!seeded.contains(OLD_SYNC_LINE));
    assert!(!fixture.read("flake.nix").contains(OLD_SYNC_LINE));
}

// --- clean ------------------------------------------------------------------

/// VERIFIES acquisition:clean-removes-only-a-named-leftover
#[test]
fn clean_names_a_leftover_and_what_it_keeps_and_removes_only_with_apply() {
    let fixture = Fixture::new();
    fixture.write("flake.nix", &flake_pinned("0.10.1"));
    fixture.write("flake.lock", &lock_at("abc123"));
    fixture.write(".envrc", &format!("use flake\n{CHECK_LINE}\n"));
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
    assert_eq!(report["schema"], "sdd.self-depend-clean/2");
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
    assert!(kept.contains(&(".envrc", Some(CHECK_LINE))));
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
    assert_eq!(fixture.read(".envrc"), format!("use flake\n{CHECK_LINE}\n"));
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

/// VERIFIES acquisition:a-host-install-is-a-recorded-decision
#[test]
fn a_recorded_host_decision_reads_as_host_and_its_absence_as_unwired() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["state"], "unwired");
    assert_eq!(report["acquisition"], serde_json::Value::Null);

    // Without --apply the decision is shown and nothing is written.
    let declaration = ".spec-driven-docs/config.yaml";
    let before = fixture.read(declaration);
    sdd(&fixture)
        .args([
            "self-depend",
            "add",
            "--host",
            "--target",
            &fixture.target(),
        ])
        .assert()
        .success();
    assert_eq!(fixture.read(declaration), before);

    sdd(&fixture)
        .args([
            "self-depend",
            "add",
            "--host",
            "--target",
            &fixture.target(),
            "--apply",
        ])
        .assert()
        .success();
    let written = fixture.read(declaration);
    assert!(written.starts_with(&before), "a comment was dropped");
    assert!(written.contains("source: host"), "{written}");

    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["state"], "host");
    assert_eq!(report["acquisition"]["source"], "host");
    assert_eq!(report["acquisition"]["minimum"], env!("CARGO_PKG_VERSION"));
    // The declaration still verifies: the key is one this release reads.
    sdd(&fixture)
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
}

#[test]
fn a_host_decision_before_a_landing_rides_the_stage_and_lands_with_it() {
    let fixture = Fixture::new();
    sdd(&fixture)
        .args([
            "self-depend",
            "add",
            "--host",
            "--target",
            &fixture.target(),
            "--apply",
        ])
        .assert()
        .code(73)
        .stderr(predicates::str::contains("sdd stage --acquisition host"));
    assert!(!fixture.path().join(".spec-driven-docs").exists());

    let stage = fixture.state_root().parent().unwrap().join("stage-host");
    sdd(&fixture)
        .args([
            "stage",
            "--target",
            &fixture.target(),
            "--acquisition",
            "host",
            "--output",
            stage.to_str().unwrap(),
        ])
        .assert()
        .success();
    sdd(&fixture)
        .current_dir(fixture.path())
        .args(["init", "--stage", stage.to_str().unwrap(), "--apply"])
        .assert()
        .success();
    let report = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(report["state"], "host");
}

#[test]
fn a_host_decision_refuses_where_a_manager_pins_the_tool() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    fixture.write(".tool-versions", "spec-driven-docs 0.1.0\n");
    sdd(&fixture)
        .args([
            "self-depend",
            "add",
            "--host",
            "--target",
            &fixture.target(),
            "--apply",
        ])
        .assert()
        .code(73);
}

/// VERIFIES acquisition:clean-swaps-the-retired-line
#[test]
fn clean_previews_the_envrc_swap() {
    let fixture = pinned();
    fixture.write(".envrc", &format!("use flake\n{OLD_SYNC_LINE}\n"));
    let status = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(status["state"], "old-line");
    assert_eq!(status["envrc_old_line"], true);
    let next = status["next"][0].as_str().unwrap();
    assert!(next.contains("sdd self-depend clean"), "{next}");
    assert!(next.contains("sdd self-depend sync --apply"), "{next}");
    let before = digest(fixture.path());
    let report = json(sdd(&fixture).args([
        "self-depend",
        "clean",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(
        report["swaps"],
        serde_json::json!([{ "file": ".envrc", "action": "swap", "from": OLD_SYNC_LINE, "to": CHECK_LINE }])
    );
    assert_eq!(report["applied"], false);
    assert_eq!(digest(fixture.path()), before, "a preview rewrote .envrc");
}

/// VERIFIES acquisition:clean-swaps-the-retired-line
#[test]
fn clean_apply_swaps_the_old_line() {
    let fixture = pinned();
    fixture.write(".envrc", &format!("use flake\n{OLD_SYNC_LINE}\n"));
    let report = json(sdd(&fixture).args([
        "self-depend",
        "clean",
        "--target",
        &fixture.target(),
        "--apply",
        "--json",
    ]));
    assert_eq!(report["swaps"][0]["to"], CHECK_LINE);
    assert_eq!(fixture.read(".envrc"), format!("use flake\n{CHECK_LINE}\n"));
    let status = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(status["state"], "ready");
    assert_eq!(status["envrc_old_line"], false);
}

/// VERIFIES acquisition:clean-swaps-the-retired-line
#[test]
fn clean_twice_changes_nothing() {
    let fixture = pinned();
    fixture.write(".envrc", &format!("use flake\n{OLD_SYNC_LINE}\n"));
    let apply = |fixture: &Fixture| {
        json(sdd(fixture).args([
            "self-depend",
            "clean",
            "--target",
            &fixture.target(),
            "--apply",
            "--json",
        ]))
    };
    apply(&fixture);
    let once = digest(fixture.path());
    let second = apply(&fixture);
    assert_eq!(second["swaps"].as_array().unwrap().len(), 0);
    assert_eq!(second["stamps"].as_array().unwrap().len(), 0);
    assert_eq!(digest(fixture.path()), once);
}

/// VERIFIES acquisition:clean-swaps-the-retired-line
#[test]
fn clean_keeps_unrelated_envrc_lines() {
    let fixture = pinned();
    let around = |line: &str| {
        format!(
            "# Load the shell.\r\nuse flake\r\nexport GH_PAGER=\"\"\r\n\r\nif has sdd; then\r\n  {line}\r\nfi\r\n# {OLD_SYNC_LINE} in a comment stays\r\nsource_env_if_exists .envrc.local"
        )
    };
    fixture.write(".envrc", &around(OLD_SYNC_LINE));
    json(sdd(&fixture).args([
        "self-depend",
        "clean",
        "--target",
        &fixture.target(),
        "--apply",
        "--json",
    ]));
    assert_eq!(fixture.read(".envrc"), around(CHECK_LINE));
}

/// VERIFIES acquisition:clean-swaps-the-retired-line
#[test]
fn clean_reports_a_removal_where_the_check_line_is_there() {
    let fixture = pinned();
    fixture.write(
        ".envrc",
        &format!("use flake\n  {CHECK_LINE}\n{OLD_SYNC_LINE}\n"),
    );
    let report = json(sdd(&fixture).args([
        "self-depend",
        "clean",
        "--target",
        &fixture.target(),
        "--apply",
        "--json",
    ]));
    assert_eq!(report["swaps"][0]["action"], "remove");
    assert_eq!(report["swaps"][0]["to"], serde_json::Value::Null);
    assert_eq!(
        fixture.read(".envrc"),
        format!("use flake\n  {CHECK_LINE}\n")
    );
    let kept: Vec<&str> = report["kept"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|k| k["line"].as_str())
        .collect();
    assert!(kept.contains(&CHECK_LINE), "{kept:?}");
    let status = json(sdd(&fixture).args([
        "self-depend",
        "status",
        "--target",
        &fixture.target(),
        "--json",
    ]));
    assert_eq!(status["envrc_check"], true);
    assert_eq!(status["state"], "ready");
}
