//! Integration: `sdd gate` runs one gate and lists the registry.

// Integration tests: assertion style is the point, so the production
// restrictions on unwrap/panic and string building do not apply here.
// sdd: permanent the panic is this suite's failure signal, not control flow
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
fn list_names_every_gate() {
    let fixture = Fixture::new();
    let assert = fixture.cmd().args(["gate", "--list"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    for gate in spec_driven_docs::gates::GATES {
        assert!(
            stdout.contains(&format!("{}: {}", gate.id, gate.name)),
            "{} unlisted",
            gate.id
        );
    }
}

#[test]
fn a_files_scoped_gate_accepts_and_rejects() {
    let fixture = Fixture::new();
    fixture.write("_docs/decisions/ADR-use-slugs.md", "# Record\n");
    fixture
        .cmd()
        .args([
            "gate",
            "adr-filename-shape",
            "_docs/decisions/ADR-use-slugs.md",
        ])
        .current_dir(fixture.path())
        .assert()
        .success();

    fixture.write("_docs/decisions/ADR-use-v2.md", "# Record\n");
    fixture
        .cmd()
        .args([
            "gate",
            "adr-filename-shape",
            "_docs/decisions/ADR-use-v2.md",
        ])
        .current_dir(fixture.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "decision-records:filename-carries-no-digit",
        ));
}

#[test]
fn an_always_run_gate_reads_the_repository() {
    let fixture = Fixture::new();
    fixture.write("AGENTS.md", &"line\n".repeat(101));
    fixture
        .cmd()
        .args(["gate", "agents-digest-size"])
        .current_dir(fixture.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL docs-format:author-instructions-stay-within-budget ./AGENTS.md",
        ));
}

#[test]
fn an_unknown_gate_is_a_clap_error() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["gate", "nonexistent-gate"])
        .assert()
        .code(2);
}

/// The plan-zone gate reads a declared location, so the variable is the one
/// path a unit test cannot reach: this crate forbids unsafe code, and
/// setting a variable is unsafe from the 2024 edition on.
#[test]
fn the_plan_zone_variable_selects_what_the_typed_clause_gate_reads() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        "elsewhere/work.md",
        "- `_docs/specs/SPEC-auth.md` — ADDED auth:token-expiry\n",
    );

    // Unset, the instance declares no zone and the gate reports nothing.
    fixture
        .cmd()
        .args(["gate", "spec-change-is-typed"])
        .current_dir(fixture.path())
        .assert()
        .success();

    fixture
        .cmd()
        .env("SDD_PLAN_ZONE", "elsewhere")
        .args(["gate", "spec-change-is-typed"])
        .current_dir(fixture.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "FAIL spec-to-code:a-spec-change-is-typed elsewhere/work.md:1",
        ));

    // A variable pointing at nothing is a stale declaration, not a pass.
    fixture
        .cmd()
        .env("SDD_PLAN_ZONE", "gone")
        .args(["gate", "spec-change-is-typed"])
        .current_dir(fixture.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("SDD_PLAN_ZONE names gone"));
}

// ---------------------------------------------------------------------
// Subject filters. A gate judges what its row declares and what the
// command line adds, and it judges nothing its filter excludes, by either
// route.
// ---------------------------------------------------------------------

/// The trap ruff carries `--force-exclude` for: pre-commit passes changed
/// files explicitly, so an exclude that only applied to a walk would be an
/// exclude that never applied at all.
///
/// SATISFIES release:a-delivered-gate-reads-what-the-convention-owns
#[test]
fn an_excluded_path_passed_on_the_command_line_is_not_judged() {
    let fixture = Fixture::new();
    fixture.write("_docs/decisions/ADR-bad name.md", "# Record\n");

    // Without the exclude, the gate judges it and fails.
    fixture
        .cmd()
        .args([
            "gate",
            "adr-filename-shape",
            "_docs/decisions/ADR-bad name.md",
        ])
        .current_dir(fixture.path())
        .assert()
        .code(1);

    // Excluded, the same path passed the same way reports nothing.
    fixture
        .cmd()
        .args([
            "gate",
            "adr-filename-shape",
            "--exclude",
            "_docs/decisions/ADR-bad name.md",
            "_docs/decisions/ADR-bad name.md",
        ])
        .current_dir(fixture.path())
        .assert()
        .success();
}

/// The same guarantee by the other route, where the gate finds the path
/// itself rather than being handed it.
///
/// SATISFIES release:a-delivered-gate-reads-what-the-convention-owns
#[test]
fn an_excluded_path_a_gate_walks_to_is_not_judged() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    // A digest far over its budget, which the walking gate would report.
    fixture.write("deep/AGENTS.md", &"line\n".repeat(200));

    fixture
        .cmd()
        .args(["gate", "agents-digest-size"])
        .current_dir(fixture.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("deep/AGENTS.md"));

    fixture
        .cmd()
        .args(["gate", "agents-digest-size", "--exclude", "deep/**"])
        .current_dir(fixture.path())
        .assert()
        .success();
}

/// A filter bounds what a gate judges, never what it reads to know what to
/// judge. Excluding the records would otherwise be an off switch.
#[test]
fn a_gate_still_reads_its_support_files_under_a_filter() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        "_docs/reference/known-issues/KI-a-case.md",
        "---\nupstream: https://example.invalid/i\nstate: open\nfiling: gathering\n---\n# A case\n",
    );
    // Exclude the records themselves. The gate that resolves a suppression
    // against them still resolves it, because they are support.
    fixture
        .cmd()
        .args([
            "gate",
            "suppression-names-its-case",
            "--exclude",
            "_docs/reference/known-issues/**",
        ])
        .current_dir(fixture.path())
        .assert()
        .success();
}

#[test]
fn a_path_outside_the_repository_is_refused() {
    let fixture = Fixture::new();
    for path in ["/etc/passwd", "../outside.md"] {
        fixture
            .cmd()
            .args(["gate", "adr-filename-shape", path])
            .current_dir(fixture.path())
            .assert()
            .code(64);
    }
}

#[test]
fn a_malformed_pattern_is_a_usage_error() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["gate", "adr-filename-shape", "--exclude", "!negated"])
        .current_dir(fixture.path())
        .assert()
        .code(64);
}

#[test]
fn a_repeated_flag_adds_each_pattern() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("a/AGENTS.md", &"line\n".repeat(200));
    fixture.write("b/AGENTS.md", &"line\n".repeat(200));

    fixture
        .cmd()
        .args([
            "gate",
            "agents-digest-size",
            "--exclude",
            "a/**",
            "--exclude",
            "b/**",
        ])
        .current_dir(fixture.path())
        .assert()
        .success();
}

#[test]
fn explain_names_the_deciding_pattern_and_the_types_selector() {
    let fixture = Fixture::new();
    let assert = fixture
        .cmd()
        .args(["gate", "--explain", "_docs/decisions/ADR-a-choice.md"])
        .current_dir(fixture.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.contains("judges       adr-filename-shape"),
        "the row that judges it is missing:\n{stdout}"
    );
    assert!(
        stdout.contains("skipped      no-self-narration"),
        "the excluded row does not name its exclude:\n{stdout}"
    );
    assert!(
        stdout.contains("not included comparison-legend"),
        "an include miss is not reported as such:\n{stdout}"
    );
    assert!(
        stdout.contains("types: [markdown]"),
        "the second selector pre-commit applies is not shown:\n{stdout}"
    );
    assert!(
        stdout.contains("note: pre-commit also applies"),
        "the answer does not say what it is not:\n{stdout}"
    );
}

#[test]
fn explain_needs_no_gate_id_and_conflicts_with_list() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["gate", "--explain", "README.md"])
        .current_dir(fixture.path())
        .assert()
        .success();
    fixture
        .cmd()
        .args(["gate", "--explain", "README.md", "--list"])
        .current_dir(fixture.path())
        .assert()
        .failure();
}
