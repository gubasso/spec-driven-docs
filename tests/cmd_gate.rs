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
fn a_path_climbing_out_of_the_repository_is_refused() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["gate", "adr-filename-shape", "../outside.md"])
        .current_dir(fixture.path())
        .assert()
        .code(64);
}

/// Pre-commit hands the message file by absolute path at the `commit-msg`
/// stage, and it sits outside the working tree by design. Refusing every
/// absolute path broke that wiring once; this holds the repair.
#[test]
fn an_absolute_path_is_accepted_because_pre_commit_passes_one() {
    let fixture = Fixture::new();
    let message = fixture.path().join("COMMIT_EDITMSG");
    std::fs::write(&message, "chore: a subject\n").unwrap();
    fixture
        .cmd()
        .args(["gate", "no-personal-path", message.to_str().unwrap()])
        .current_dir(fixture.path())
        .assert()
        .success();
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

/// The leak check judges the whole project again, which is what v0.6.5 took
/// and what the declaration makes safe to give back.
///
/// SATISFIES docs-foundations:a-document-carries-no-personal-path
#[test]
fn the_leak_check_reports_a_home_directory_in_a_root_readme() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    // Assembled rather than written out, so this source file carries no
    // personal path of its own. The gate judges it now.
    fixture.write(
        "README.md",
        &format!("Run it from {}/ada/projects.\n", "/ho".to_owned() + "me"),
    );

    // v0.6.5 anchored this row to the documentation root, so the path
    // below was outside what the gate would judge however it was reached.
    fixture
        .cmd()
        .args(["gate", "no-personal-path", "README.md"])
        .current_dir(fixture.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("README.md"));

    // And pre-commit reaches it: the rendered row carries no `files:`, so
    // `types: [text]` alone selects, which is the breadth being restored.
    let assert = fixture
        .cmd()
        .args(["gate", "--explain", "README.md"])
        .current_dir(fixture.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("judges       no-personal-path"),
        "the leak check does not judge a root README:\n{stdout}"
    );
}

#[test]
fn a_reserved_path_still_stops_the_leak_check() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    // Assembled rather than written out, so this source file carries no
    // personal path of its own. The gate judges it now.
    fixture.write(
        "README.md",
        &format!("Run it from {}/ada/projects.\n", "/ho".to_owned() + "me"),
    );
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - README.md\ngates: {}\n",
    );

    fixture
        .cmd()
        .args(["gate", "no-personal-path", "README.md"])
        .current_dir(fixture.path())
        .assert()
        .success();
}

/// What `suppression-rule-without-a-parser` rests on. Its rewritten gate
/// sweeps every `KI-` token and resolves it against the records, and this
/// convention cannot tell a citation from a test fixture that builds a fake
/// record. The project can, and this proves the three things that item
/// needs: a reserved fixture leaves the subject set, a non-reserved source
/// path stays in it, and the records stay readable as support.
#[test]
fn a_reserved_fixture_is_absent_from_the_suppression_subject_set() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        "_docs/reference/known-issues/KI-a-real-case.md",
        "---\nupstream: https://example.invalid/i\nstate: open\nfiling: gathering\n---\n# A case\n",
    );
    // A fixture naming a record that does not exist, and a source file
    // whose suppression names the record that does.
    fixture.write("tests/fixtures/build.rs", "// KI-a-fabricated-case\n");
    fixture.write("src/real.rs", "// sdd: permanent KI-a-real-case\n");
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - tests/fixtures/**\ngates: {}\n",
    );

    let assert = fixture
        .cmd()
        .args(["gate", "--explain", "tests/fixtures/build.rs"])
        .current_dir(fixture.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("skipped      suppression-names-its-case"),
        "the reserved fixture is still in the gate's subject set:\n{stdout}"
    );

    let assert = fixture
        .cmd()
        .args(["gate", "--explain", "src/real.rs"])
        .current_dir(fixture.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("judges       suppression-names-its-case"),
        "a non-reserved source path left the subject set too:\n{stdout}"
    );

    // The records are support, so reserving a subject path does not stop the
    // gate resolving a case against them.
    fixture
        .cmd()
        .args(["gate", "suppression-names-its-case"])
        .current_dir(fixture.path())
        .assert()
        .success();
}

/// Every gate that finds its own subjects by reading a directory honours the
/// declaration too.
///
/// The source-text inventory in `tests/canon.rs` cannot see a
/// `read_dir_utf8` route, and four gates took one. This asserts the
/// behaviour instead of the shape of the code.
///
/// SATISFIES instance:the-project-declares-what-its-gates-judge
#[test]
fn a_reserved_path_leaves_the_subject_set_of_a_self_discovering_gate() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");

    // A spec far past the size cap, a decision record past the word cap, and
    // a known-issue record with no state. Each is found by its gate reading
    // a directory, not by a path anyone passes.
    fixture.write(
        "_docs/specs/SPEC-oversized.md",
        &format!(
            "# SPEC oversized\n\n{}",
            "A line about a rule.\n".repeat(400)
        ),
    );
    fixture.write(
        "_docs/decisions/ADR-a-long-record.md",
        &format!("# A long record\n\n{}", "word ".repeat(600)),
    );
    fixture.write(
        "_docs/reference/known-issues/KI-a-shapeless-case.md",
        "# no front matter at all\n",
    );

    let cases = [
        ("spec-size-cap", "_docs/specs/SPEC-oversized.md"),
        ("adr-word-cap", "_docs/decisions/ADR-a-long-record.md"),
        (
            "ki-state",
            "_docs/reference/known-issues/KI-a-shapeless-case.md",
        ),
    ];

    for (gate, path) in cases {
        fixture.write(".spec-driven-docs/config.yaml", "reserved: []\ngates: {}\n");
        fixture
            .cmd()
            .args(["gate", gate])
            .current_dir(fixture.path())
            .assert()
            .code(1);

        fixture.write(
            ".spec-driven-docs/config.yaml",
            &format!("reserved:\n  - '{path}'\ngates: {{}}\n"),
        );
        fixture
            .cmd()
            .args(["gate", gate])
            .current_dir(fixture.path())
            .assert()
            .success();
    }
}

/// A `codebase` instance keeps its records under `docs/`, so a filter built
/// against a fixed `_docs` would drop every path pre-commit passes and the
/// gate would silently judge nothing.
#[test]
fn a_filename_selected_gate_judges_under_the_recorded_docs_root() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    fixture.write("docs/decisions/ADR-bad name.md", "# Record\n");

    fixture
        .cmd()
        .args([
            "gate",
            "adr-filename-shape",
            "docs/decisions/ADR-bad name.md",
        ])
        .current_dir(fixture.path())
        .assert()
        .code(1);
}

/// A positional value is a file to judge or a record root to resolve.
/// Filtering the root drops it, and the gate then falls back to the default
/// location and reports nothing about the records it was pointed at.
#[test]
fn an_extra_record_root_reaches_the_gate_unfiltered() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    // A record outside the docs root, in a directory the gate's include
    // pattern does not match.
    fixture.write(
        "fixtures/KI-a-shapeless-case.md",
        "# no front matter at all\n",
    );

    fixture
        .cmd()
        .args(["gate", "ki-state", "fixtures"])
        .current_dir(fixture.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("fixtures/KI-a-shapeless-case.md"));
}

/// Reserving every spec is an answer, not a moved layout. Both layout
/// predicates read the unfiltered set.
#[test]
fn reserving_every_spec_is_not_a_layout_failure() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - '_docs/specs/**'\ngates: {}\n",
    );

    for gate in [
        "spec-verify-hooks-exist",
        "spec-size-cap",
        "spec-rule-id-unique",
    ] {
        fixture
            .cmd()
            .args(["gate", gate])
            .current_dir(fixture.path())
            .assert()
            .success();
    }
}

/// A nested brace alternation renders as something matching nothing the
/// matcher judges, so the grammar refuses it rather than break the superset
/// contract.
#[test]
fn a_nested_brace_alternation_is_refused() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - '{{a,b},{c,d}}.md'\ngates: {}\n",
    );

    fixture
        .cmd()
        .args(["gate", "ki-state"])
        .current_dir(fixture.path())
        .assert()
        .code(64);
}

/// An absolute name for a file inside the repository is the same file, and
/// a declaration only ever writes the relative one.
#[test]
fn a_reserved_path_stays_reserved_under_its_absolute_name() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("_docs/private.md", "Nothing here.\n");
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - '_docs/private.md'\ngates: {}\n",
    );
    let absolute = fixture.path().join("_docs/private.md");

    let assert = fixture
        .cmd()
        .args(["gate", "--explain", absolute.to_str().unwrap()])
        .current_dir(fixture.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("(reserved)"),
        "the absolute name walked past the reservation:\n{stdout}"
    );
}

/// A project's own include is a whitelist it wrote against this gate
/// knowingly, so it binds even where the gate discovered the set itself.
#[test]
fn a_project_include_narrows_a_self_discovering_gate() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        "_docs/specs/SPEC-oversized.md",
        &format!(
            "# SPEC oversized\n\n{}",
            "A line about a rule.\n".repeat(400)
        ),
    );

    // Undeclared, the gate discovers and reports it.
    fixture
        .cmd()
        .args(["gate", "spec-size-cap"])
        .current_dir(fixture.path())
        .assert()
        .code(1);

    // Narrowed to another spec, it does not.
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved: []\ngates:\n  spec-size-cap:\n    include:\n      - '_docs/specs/SPEC-instance.md'\n",
    );
    fixture
        .cmd()
        .args(["gate", "spec-size-cap"])
        .current_dir(fixture.path())
        .assert()
        .success();
}

/// A reservation is written against a name, so it binds that name however
/// it is spelled. Resolving the path instead would rewrite a link to its
/// target and let the absolute spelling walk past.
#[test]
fn a_reserved_symlink_stays_reserved_under_its_absolute_name() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("real.md", "Nothing here.\n");
    std::os::unix::fs::symlink("real.md", fixture.path().join("alias.md")).unwrap();
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - 'alias.md'\ngates: {}\n",
    );

    for spelling in [
        "alias.md".to_string(),
        fixture
            .path()
            .join("alias.md")
            .to_str()
            .unwrap()
            .to_string(),
    ] {
        let assert = fixture
            .cmd()
            .args(["gate", "--explain", &spelling])
            .current_dir(fixture.path())
            .assert()
            .success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
        assert!(
            stdout.contains("(reserved)"),
            "the reservation did not bind {spelling}:\n{stdout}"
        );
    }
}

/// `--explain` answers with the rule the gate actually applies. An
/// always-run gate discovers its own subjects, so the registry include does
/// not narrow them.
#[test]
fn explain_answers_with_the_rule_a_self_discovering_gate_applies() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write(
        "fixtures/KI-a-shapeless-case.md",
        "# no front matter at all\n",
    );

    // The gate judges it when pointed at the root.
    fixture
        .cmd()
        .args(["gate", "ki-state", "fixtures"])
        .current_dir(fixture.path())
        .assert()
        .code(1);

    let assert = fixture
        .cmd()
        .args(["gate", "--explain", "fixtures/KI-a-shapeless-case.md"])
        .current_dir(fixture.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("not discovered ki-state"),
        "explain does not say the gate reaches this path only when pointed at it:\n{stdout}"
    );
}

/// An always-run gate is not necessarily one that discovers its own
/// subjects. `agents-digest-size` runs always and still judges what the
/// walk hands it, which the registry include narrows, so `--explain` must
/// not answer for it with the retained rule.
#[test]
fn explain_answers_with_the_rule_a_walking_gate_applies() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");

    let assert = fixture
        .cmd()
        .args(["gate", "--explain", "README.md"])
        .current_dir(fixture.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("not included agents-digest-size"),
        "explain claims a walking gate ignores its include:\n{stdout}"
    );
}

/// An operator who entered through a symlinked checkout types that
/// spelling, and a reservation must still bind.
#[test]
fn a_reservation_binds_through_a_symlinked_checkout_root() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("_docs/private.md", "Nothing here.\n");
    fixture.write(
        ".spec-driven-docs/config.yaml",
        "reserved:\n  - '_docs/private.md'\ngates: {}\n",
    );

    let link = std::env::temp_dir().join(format!("sdd-link-{}", std::process::id()));
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(fixture.path(), &link).unwrap();

    let through_link = link.join("_docs/private.md");
    let assert = fixture
        .cmd()
        .args(["gate", "--explain", through_link.to_str().unwrap()])
        .current_dir(&link)
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let _ = std::fs::remove_file(&link);
    assert!(
        stdout.contains("(reserved)"),
        "the symlinked checkout spelling walked past the reservation:\n{stdout}"
    );
}

/// A gate whose subject set is one fixed file must not claim every path it
/// does not exclude. The answer for a discovering gate is three-valued:
/// what it discovers, what it excludes, and what it reaches only when
/// pointed at it.
#[test]
fn explain_does_not_overclaim_for_a_fixed_subject_gate() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");

    let assert = fixture
        .cmd()
        .args(["gate", "--explain", "README.md"])
        .current_dir(fixture.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    for gate in ["instance-manifest", "spec-size-cap", "tracking-registry"] {
        assert!(
            !stdout.contains(&format!("judges       {gate}")),
            "{gate} cannot judge README.md and explain says it does:\n{stdout}"
        );
        assert!(
            stdout.contains(&format!("not discovered {gate}")),
            "{gate} is missing its three-valued answer:\n{stdout}"
        );
    }

    // And the answer stays right where the gate really does judge.
    let assert = fixture
        .cmd()
        .args(["gate", "--explain", ".spec-driven-docs/manifest.json"])
        .current_dir(fixture.path())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("judges       instance-manifest"),
        "explain lost the true answer:\n{stdout}"
    );
    assert!(
        stdout.contains("`not discovered` means outside the gate's own set"),
        "the answer does not say what `not discovered` means:\n{stdout}"
    );
    assert!(
        stdout.contains("others read one fixed location and never will"),
        "the answer claims every discovering gate can be redirected:\n{stdout}"
    );
}
