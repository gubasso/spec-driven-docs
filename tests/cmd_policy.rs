//! Integration: `sdd policy reconcile` brings an adopted specification into
//! agreement with what the project declared, on request and never on
//! upgrade.

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

const SPEC: &str = "_docs/specs/SPEC-budget-debt.md";
const SENTINEL: &str = "budget-debt:a-recorded-dimension-only-shrinks";

/// An instance carrying debt whose own copy of the owning specification
/// predates the sentinel, with a rule the project wrote beside it.
fn needing_reconciliation() -> Fixture {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    fixture.write("method/legacy.md", &"line\n".repeat(250));
    fixture
        .cmd()
        .args(["debt", "baseline", "--target", &fixture.target(), "--apply"])
        .assert()
        .success();
    let own = "# Budget Debt Specification\n\n## Purpose\n\nOurs, from before.\n\n## Requirements\n\n### `budget-debt:our-own-rule` — Ours\n\nThe author MUST keep it.\n\n#### Scenario: Ours\n\n- GIVEN x\n- WHEN y\n- THEN z\n\nVerify: `true`\n\n## Unenforced\n\n| Rule | Reviewer confirms |\n| --- | --- |\n";
    fixture.write(SPEC, own);
    fixture
}

fn reconcile(fixture: &Fixture, apply: bool) -> assert_cmd::assert::Assert {
    let target = fixture.target();
    let mut args = vec!["policy", "reconcile", "--target", target.as_str()];
    if apply {
        args.push("--apply");
    }
    fixture.cmd().args(&args).assert()
}

#[test]
fn reconcile_previews_and_writes_nothing() {
    let fixture = needing_reconciliation();
    let digest = fixture.tree_digest();
    reconcile(&fixture, false)
        .success()
        .stdout(predicate::str::contains(format!(
            "{SPEC}: append `{SENTINEL}` to its Requirements section:"
        )))
        .stdout(predicate::str::contains(format!("  ### `{SENTINEL}`")))
        .stdout(predicate::str::contains("  Verify:"))
        .stdout(predicate::str::contains("DRY RUN: no files written"));
    assert_eq!(digest, fixture.tree_digest(), "the preview changed bytes");
}

#[test]
fn apply_appends_the_missing_rule_and_the_sentinel_resolves() {
    let fixture = needing_reconciliation();
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("run 'sdd policy reconcile'"));

    reconcile(&fixture, true)
        .success()
        .stdout(predicate::str::contains(format!("OK wrote {SPEC}")));

    let spec = fixture.read(SPEC);
    assert!(spec.contains(&format!("### `{SENTINEL}`")));
    // The project's own rule survives byte-identical, in place, and the
    // sections around it are untouched.
    assert!(spec.starts_with("# Budget Debt Specification\n\n## Purpose\n\nOurs, from before.\n"));
    assert!(spec.contains("### `budget-debt:our-own-rule` — Ours\n\nThe author MUST keep it.\n"));
    assert!(spec.contains("Verify: `true`\n\n### `budget-debt:a-recorded"));
    assert!(spec.ends_with("## Unenforced\n\n| Rule | Reviewer confirms |\n| --- | --- |\n"));

    // The record moved with the file: verify is green, not drifted, and
    // the note is gone.
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("no local specification defines").not())
        .stdout(predicate::str::contains("DRIFT").not());
    reconcile(&fixture, true)
        .success()
        .stdout(predicate::str::contains(
            "OK every active declaration is authorized by a local specification",
        ));
}

#[test]
fn an_unrecognized_shape_prints_a_checklist_and_writes_nothing() {
    let fixture = needing_reconciliation();
    fixture.write(
        SPEC,
        "# Budget Debt\n\nProse the project wrote, with no requirements section.\n",
    );
    let digest = fixture.tree_digest();
    reconcile(&fixture, false)
        .success()
        .stdout(predicate::str::contains(format!(
            "{SPEC}: not in a shape this command rewrites; add `{SENTINEL}` by hand:"
        )))
        .stdout(predicate::str::contains(format!("  ### `{SENTINEL}`")));
    reconcile(&fixture, true)
        .code(73)
        .stderr(predicate::str::contains("nothing written"))
        .stderr(predicate::str::contains(SPEC));
    assert_eq!(
        digest,
        fixture.tree_digest(),
        "a refused apply changed bytes"
    );
}

#[test]
fn an_absent_owning_specification_is_seeded_and_recorded() {
    let fixture = needing_reconciliation();
    std::fs::remove_file(fixture.path().join(SPEC)).unwrap();
    reconcile(&fixture, false)
        .success()
        .stdout(predicate::str::contains(format!(
            "{SPEC}: absent; write the seed"
        )));
    reconcile(&fixture, true).success();
    assert!(fixture.read(SPEC).contains(SENTINEL));
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("DRIFT").not());
}

#[test]
fn reconcile_is_a_no_op_where_every_sentinel_resolves() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let digest = fixture.tree_digest();
    reconcile(&fixture, true)
        .success()
        .stdout(predicate::str::contains(
            "OK every active declaration is authorized by a local specification",
        ));
    assert_eq!(digest, fixture.tree_digest());
}

#[test]
fn a_non_builtin_selection_reconciles_the_writing_policy() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let declaration = fixture.read(".spec-driven-docs/config.yaml").replace(
        "writing_style:\n  source: builtin\n  path: null\n",
        "writing_style:\n  source: none\n  path: null\n",
    );
    fixture.write(".spec-driven-docs/config.yaml", &declaration);
    fixture
        .cmd()
        .args(["hooks", "--target", &fixture.target(), "--apply"])
        .assert()
        .success();
    let spec = "_docs/specs/SPEC-writing-policy.md";
    let older = fixture.read(spec).replace(
        "writing-policy:the-project-selects-one-source",
        "writing-policy:an-older-sentence",
    );
    fixture.write(spec, &older);
    reconcile(&fixture, true)
        .success()
        .stdout(predicate::str::contains(format!("OK wrote {spec}")));
    assert!(
        fixture
            .read(spec)
            .contains("### `writing-policy:the-project-selects-one-source`")
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success()
        .stdout(predicate::str::contains("no local specification defines").not());
}

#[test]
fn a_relative_target_other_than_dot_is_refused() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args(["policy", "reconcile", "--target", "elsewhere"])
        .assert()
        .code(64);
}
