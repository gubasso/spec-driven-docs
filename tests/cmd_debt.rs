//! Integration: `sdd debt` records, migrates, and tightens the inherited
//! budget violations a project carries, and the budget gates judge them.

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

const DEBT: &str = ".spec-driven-docs/debt.yaml";
const LEGACY: &str = ".spec-driven-docs/chapter-size-debt.txt";
const BUDGET_GATES: &[&str] = &[
    "adr-word-cap",
    "agents-digest-size",
    "chapter-size-cap",
    "spec-size-cap",
];

/// An installed instance carrying one inherited violation per budget gate,
/// and one oversize spec that also lacks its table of contents.
fn inherited() -> Fixture {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("method/legacy.md", &"line\n".repeat(250));
    fixture.write("method/AGENTS.md", &"line\n".repeat(160));
    fixture.write("_docs/decisions/ADR-old-platform.md", &"word ".repeat(400));
    fixture.write("_docs/specs/SPEC-legacy.md", &"line\n".repeat(417));
    fixture
}

fn gate(fixture: &Fixture, id: &str) -> assert_cmd::assert::Assert {
    fixture
        .cmd()
        .args(["gate", id])
        .current_dir(fixture.path())
        .assert()
}

fn every_gate_is_green(fixture: &Fixture) {
    for id in BUDGET_GATES {
        gate(fixture, id).success();
    }
}

fn debt(fixture: &Fixture, verb: &str, apply: bool) -> assert_cmd::assert::Assert {
    let target = fixture.target();
    let mut args = vec!["debt", verb, "--target", target.as_str()];
    if apply {
        args.push("--apply");
    }
    fixture.cmd().args(&args).assert()
}

#[test]
fn baseline_records_every_current_violation() {
    let fixture = inherited();
    for id in BUDGET_GATES {
        gate(&fixture, id).code(1);
    }

    // The preview is the operator's first view of the corpus, and it writes
    // nothing.
    let digest = fixture.tree_digest();
    debt(&fixture, "baseline", false)
        .success()
        .stdout(predicate::str::contains("DRY RUN: no files written"))
        .stdout(predicate::str::contains("method/legacy.md"))
        .stdout(predicate::str::contains("ceiling: 250"));
    assert_eq!(digest, fixture.tree_digest(), "the preview changed bytes");

    debt(&fixture, "baseline", true)
        .success()
        .stdout(predicate::str::contains(format!("OK wrote {DEBT}")));
    let recorded = fixture.read(DEBT);
    for expected in [
        "chapter-size-cap:\n  'method/legacy.md':\n    lines:\n      ceiling: 250",
        "agents-digest-size:\n  'method/AGENTS.md':\n    lines:\n      ceiling: 160",
        "adr-word-cap:\n  '_docs/decisions/ADR-old-platform.md':\n    words:\n      ceiling: 400",
        "spec-size-cap:\n  '_docs/specs/SPEC-legacy.md':\n    authored_lines:\n      ceiling: 417\n    missing_toc: true",
    ] {
        assert!(
            recorded.contains(expected),
            "{expected:?} is missing from:\n{recorded}"
        );
    }

    // The first commit is green, and the instance verifies with no note:
    // the seeded specification defines the sentinel.
    every_gate_is_green(&fixture);
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("no local specification defines").not());
}

#[test]
fn baseline_refuses_where_a_debt_file_exists() {
    let fixture = inherited();
    debt(&fixture, "baseline", true).success();
    fixture.write("method/second-corpus.md", &"line\n".repeat(300));
    let digest = fixture.tree_digest();
    debt(&fixture, "baseline", true)
        .code(73)
        .stderr(predicate::str::contains("sdd debt tighten --apply"))
        .stderr(predicate::str::contains("fix the violation"));
    assert_eq!(
        digest,
        fixture.tree_digest(),
        "a refused baseline changed bytes"
    );
    gate(&fixture, "chapter-size-cap")
        .code(1)
        .stdout(predicate::str::contains("./method/second-corpus.md"));
}

#[test]
fn a_clean_corpus_needs_no_debt_file() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    debt(&fixture, "baseline", true)
        .success()
        .stdout(predicate::str::contains("no debt file is needed"));
    assert!(!fixture.path().join(DEBT).exists());
}

#[test]
fn migrate_preserves_every_exemption_and_fixes_each_ceiling() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("method/carried.md", &"line\n".repeat(280));
    fixture.write("method/fits.md", &"line\n".repeat(50));
    fixture.write("method/unlisted.md", &"line\n".repeat(230));
    fixture.write(
        LEGACY,
        "# exempt\nmethod/carried.md\n./method/fits.md\nmethod/gone.md\n",
    );

    // The legacy list skips a listed chapter; the unlisted one fails.
    gate(&fixture, "chapter-size-cap")
        .code(1)
        .stdout(predicate::str::contains("./method/unlisted.md"))
        .stdout(predicate::str::contains("./method/carried.md").not());

    debt(&fixture, "migrate", false)
        .success()
        .stdout(predicate::str::contains(
            "method/carried.md: ceiling fixed at 280 lines",
        ))
        .stdout(predicate::str::contains(
            "method/fits.md: now fits; dropped",
        ))
        .stdout(predicate::str::contains(
            "method/gone.md: the gate measures no such path; dropped",
        ))
        .stdout(predicate::str::contains("DRY RUN"));
    assert!(fixture.path().join(LEGACY).is_file());

    debt(&fixture, "migrate", true)
        .success()
        .stdout(predicate::str::contains(format!("OK wrote {DEBT}")))
        .stdout(predicate::str::contains(format!("OK removed {LEGACY}")));
    assert!(!fixture.path().join(LEGACY).exists());
    let recorded = fixture.read(DEBT);
    assert!(recorded.contains("'method/carried.md':\n    lines:\n      ceiling: 280"));
    assert!(!recorded.contains("fits.md"));
    assert!(!recorded.contains("gone.md"));

    // Migration never broadens: the violation outside the list still fails,
    // and the carried chapter is held to its ceiling.
    gate(&fixture, "chapter-size-cap")
        .code(1)
        .stdout(predicate::str::contains("./method/unlisted.md"))
        .stdout(predicate::str::contains("./method/carried.md").not());
    fixture.write("method/carried.md", &"line\n".repeat(281));
    gate(&fixture, "chapter-size-cap")
        .code(1)
        .stdout(predicate::str::contains(
            "./method/carried.md: 281 lines, recorded ceiling is 280",
        ));
}

#[test]
fn migrate_refuses_with_no_legacy_list() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    debt(&fixture, "migrate", true)
        .code(73)
        .stderr(predicate::str::contains("no legacy list to migrate"));
}

#[test]
fn both_formats_present_is_a_failure_naming_migrate() {
    // The state an interrupted migration leaves: the new file landed and the
    // old list did not leave. Neither wins; the gates and the verifier stop
    // and name the verb that finishes it.
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("_docs/decisions/ADR-choice.md", "# Choice\n");
    fixture.write("method/carried.md", &"line\n".repeat(280));
    fixture.write(LEGACY, "method/carried.md\n");
    fixture.write(
        DEBT,
        "schema_version: 1\nchapter-size-cap:\n  method/carried.md:\n    lines:\n      ceiling: 280\n",
    );
    for id in BUDGET_GATES {
        gate(&fixture, id)
            .code(65)
            .stderr(predicate::str::contains("sdd debt migrate --apply"));
    }
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("FAIL both"))
        .stdout(predicate::str::contains("sdd debt migrate --apply"));
    debt(&fixture, "baseline", true).code(73);

    // Re-running the migration finishes it, from the list that stayed
    // authoritative.
    debt(&fixture, "migrate", true).success();
    assert!(!fixture.path().join(LEGACY).exists());
    every_gate_is_green(&fixture);
}

#[test]
fn a_document_below_its_ceiling_fails_naming_tighten_and_tighten_lowers_it() {
    let fixture = inherited();
    debt(&fixture, "baseline", true).success();

    fixture.write("method/legacy.md", &"line\n".repeat(220));
    gate(&fixture, "chapter-size-cap")
        .code(1)
        .stdout(predicate::str::contains("./method/legacy.md"))
        .stdout(predicate::str::contains(
            "below the recorded ceiling of 250",
        ))
        .stdout(predicate::str::contains("sdd debt tighten --apply"));

    debt(&fixture, "tighten", false)
        .success()
        .stdout(predicate::str::contains(
            "chapter-size-cap: method/legacy.md: lines: ceiling 250 -> 220",
        ))
        .stdout(predicate::str::contains("DRY RUN"));
    assert!(fixture.read(DEBT).contains("ceiling: 250"));

    debt(&fixture, "tighten", true).success();
    assert!(
        fixture
            .read(DEBT)
            .contains("'method/legacy.md':\n    lines:\n      ceiling: 220")
    );
    every_gate_is_green(&fixture);

    // A ceiling never rises: growth is a failure, and tightening leaves it.
    fixture.write("method/legacy.md", &"line\n".repeat(250));
    gate(&fixture, "chapter-size-cap")
        .code(1)
        .stdout(predicate::str::contains(
            "250 lines, recorded ceiling is 220",
        ));
    debt(&fixture, "tighten", true)
        .success()
        .stdout(predicate::str::contains("above the ceiling of 220"))
        .stdout(predicate::str::contains("nothing to tighten"));
    assert!(fixture.read(DEBT).contains("ceiling: 220"));
}

#[test]
fn an_oversized_spec_without_a_toc_baselines_both_dimensions_and_improves_without_a_new_failure() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("_docs/decisions/ADR-choice.md", "# Choice\n");
    fixture.write("_docs/specs/SPEC-legacy.md", &"line\n".repeat(417));
    debt(&fixture, "baseline", true).success();
    let recorded = fixture.read(DEBT);
    assert!(recorded.contains("authored_lines:\n      ceiling: 417"));
    assert!(recorded.contains("missing_toc: true"));
    every_gate_is_green(&fixture);

    // Shrinking to the budget leaves a stale ceiling and nothing else: the
    // missing table of contents is still carried by its exception.
    fixture.write("_docs/specs/SPEC-legacy.md", &"line\n".repeat(300));
    gate(&fixture, "spec-size-cap")
        .code(1)
        .stdout(predicate::str::contains("within the budget"))
        .stdout(predicate::str::contains("no TOC").not());
    debt(&fixture, "tighten", true)
        .success()
        .stdout(predicate::str::contains(
            "authored_lines: removed, within the budget",
        ));
    let recorded = fixture.read(DEBT);
    assert!(!recorded.contains("authored_lines"));
    assert!(recorded.contains("missing_toc: true"));
    every_gate_is_green(&fixture);

    // Adding the table of contents corrects the last exception, and the
    // file leaves with it.
    fixture.write(
        "_docs/specs/SPEC-legacy.md",
        &format!("<!--TOC-->\n<!--TOC-->\n{}", "line\n".repeat(300)),
    );
    gate(&fixture, "spec-size-cap")
        .code(1)
        .stdout(predicate::str::contains(
            "exception for missing table of contents is corrected",
        ));
    debt(&fixture, "tighten", true)
        .success()
        .stdout(predicate::str::contains(format!("OK removed {DEBT}")));
    assert!(!fixture.path().join(DEBT).exists());
    every_gate_is_green(&fixture);
}

#[test]
fn a_malformed_debt_file_stops_every_budget_gate() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("_docs/decisions/ADR-choice.md", "# Choice\n");
    fixture.write(
        DEBT,
        "schema_version: 1\nchapter-size-cap:\n  method/a.md:\n    lines: 3\n",
    );
    for id in BUDGET_GATES {
        gate(&fixture, id)
            .code(65)
            .stderr(predicate::str::contains("chapter-size-cap: method/a.md"));
    }
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("FAIL .spec-driven-docs/debt.yaml"));
}

#[test]
fn tighten_writes_atomically_over_many_entries() {
    let fixture = inherited();
    debt(&fixture, "baseline", true).success();
    fixture.write("method/legacy.md", &"line\n".repeat(210));
    fixture.write("method/AGENTS.md", &"line\n".repeat(155));
    fixture.write("_docs/decisions/ADR-old-platform.md", &"word ".repeat(360));
    debt(&fixture, "tighten", true).success();
    let recorded = fixture.read(DEBT);
    assert!(recorded.contains("ceiling: 210"));
    assert!(recorded.contains("ceiling: 155"));
    assert!(recorded.contains("ceiling: 360"));
    let leftovers: Vec<String> = std::fs::read_dir(fixture.path().join(".spec-driven-docs"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name.contains("sdd-tmp"))
        .collect();
    assert!(leftovers.is_empty(), "scratch files remain: {leftovers:?}");
    every_gate_is_green(&fixture);
}

#[test]
fn a_relative_target_other_than_dot_is_refused() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["debt", "baseline", "--target", "elsewhere"])
        .assert()
        .code(64);
}

/// The migration reads the list with the gate's own parser. A line the gate
/// never honoured, because it carries whitespace, becomes no debt.
#[test]
fn migrate_never_broadens_a_line_the_gate_did_not_honour() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("method/carried.md", &"line\n".repeat(280));
    fixture.write(LEGACY, " method/carried.md \n");
    gate(&fixture, "chapter-size-cap")
        .code(1)
        .stdout(predicate::str::contains("./method/carried.md"));
    debt(&fixture, "migrate", true).success();
    assert!(
        !fixture.path().join(DEBT).exists(),
        "a non-exemption became debt"
    );
    gate(&fixture, "chapter-size-cap")
        .code(1)
        .stdout(predicate::str::contains("./method/carried.md"));
}

/// A symlinked instance directory is refused before a byte lands outside
/// the target.
#[test]
fn a_debt_write_never_follows_a_symlinked_instance_directory() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("method/legacy.md", &"line\n".repeat(250));
    let outside = tempfile::tempdir().unwrap();
    let instance = fixture.path().join(".spec-driven-docs");
    let moved = outside.path().join("instance");
    std::fs::rename(&instance, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &instance).unwrap();
    debt(&fixture, "baseline", true)
        .code(77)
        .stderr(predicate::str::contains("symlink"));
    assert!(
        !outside.path().join("debt.yaml").exists(),
        "the write escaped the target"
    );
}
