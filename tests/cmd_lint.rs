//! Integration: `sdd lint` reports what the Markdown linter reads, and
//! `sdd lint prove` holds the link rule to a broken and a valid link.

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

fn lint_json(fixture: &Fixture, extra: &[&str]) -> serde_json::Value {
    let output = fixture
        .cmd()
        .args(["lint", "--target", &fixture.target(), "--json"])
        .args(extra)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

fn edits(report: &serde_json::Value) -> Vec<(String, String, String)> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                finding["path"].as_str().unwrap().to_string(),
                finding["rule"].as_str().unwrap().to_string(),
                finding["edit"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn has(report: &serde_json::Value, path: &str, rule: &str, edit: &str) -> bool {
    edits(report)
        .iter()
        .any(|(p, r, e)| p == path && r == rule && e == edit)
}

/// VERIFIES setup:lint-reports-each-finding-with-its-rule
#[test]
fn a_fresh_landing_has_no_finding_and_a_stray_configuration_has_one() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let report = lint_json(&fixture, &[]);
    assert_eq!(report["schema"], "sdd.lint/1");
    assert_eq!(report["profile_source"], "recorded");
    assert_eq!(edits(&report), []);

    fixture.write("docs/specs/.markdownlint.yaml", "MD013: false\n");
    let report = lint_json(&fixture, &[]);
    assert!(
        has(
            &report,
            "docs/specs/.markdownlint.yaml",
            "instance:the-lint-configuration-composes",
            "fold-into-root"
        ),
        "{report}"
    );
}

#[test]
fn a_root_configuration_without_the_rule_needs_it_loaded_and_enabled() {
    let fixture = Fixture::new();
    fixture.write(
        ".markdownlint-cli2.jsonc",
        "// the project's own\n{\"config\": {}}\n",
    );
    let report = lint_json(&fixture, &[]);
    assert_eq!(report["profile_source"], "default");
    let rule = "instance:the-root-lint-configuration-loads-relative-links";
    assert!(has(
        &report,
        ".markdownlint-cli2.jsonc",
        rule,
        "load-relative-links"
    ));
    assert!(has(
        &report,
        ".markdownlint-cli2.jsonc",
        rule,
        "enable-relative-links"
    ));

    let library = Fixture::new();
    library.write(".markdownlint.yaml", "MD013: false\n");
    let report = lint_json(&library, &["--profile", "knowledge-base"]);
    assert_eq!(report["profile"], "knowledge-base");
    assert!(has(
        &report,
        ".markdownlint-cli2.jsonc",
        rule,
        "add-companion-cli2"
    ));
    assert!(has(
        &report,
        ".markdownlint.yaml",
        rule,
        "enable-relative-links"
    ));

    let module = Fixture::new();
    module.write(".markdownlint-cli2.cjs", "module.exports = {};\n");
    let report = lint_json(&module, &[]);
    assert!(
        has(&report, ".markdownlint-cli2.cjs", rule, "unknown"),
        "{report}"
    );
}

#[test]
fn a_reserved_markdown_path_the_linter_reads_needs_an_ignore() {
    let fixture = Fixture::new();
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "codebase",
            "--reserve",
            "generated/**",
            "--apply",
        ])
        .assert()
        .success();
    fixture.write("generated/api.md", "# Api\n");
    let report = lint_json(&fixture, &[]);
    assert!(
        has(
            &report,
            "generated/**",
            "instance:the-project-declares-what-its-gates-judge",
            "add-ignore"
        ),
        "{report}"
    );

    // An ignore in a CLI2 file the linter passes over ignores nothing.
    fixture.write(
        ".markdownlint-cli2.yaml",
        "ignores:\n  - \"generated/**\"\n",
    );
    let report = lint_json(&fixture, &[]);
    assert!(
        has(
            &report,
            "generated/**",
            "instance:the-project-declares-what-its-gates-judge",
            "add-ignore"
        ),
        "{report}"
    );
}

#[test]
fn only_the_cli2_file_the_linter_reads_counts() {
    let fixture = Fixture::new();
    fixture.write(
        ".markdownlint-cli2.jsonc",
        "{\"config\": {\"relative-links\": true}}\n",
    );
    fixture.write(
        ".markdownlint-cli2.yaml",
        "customRules:\n  - markdownlint-rule-relative-links\n",
    );
    let report = lint_json(&fixture, &[]);
    let rule = "instance:the-root-lint-configuration-loads-relative-links";
    assert!(
        has(
            &report,
            ".markdownlint-cli2.jsonc",
            rule,
            "load-relative-links"
        ),
        "{report}"
    );
    assert!(
        !edits(&report)
            .iter()
            .any(|(path, _, _)| path == ".markdownlint-cli2.yaml"),
        "{report}"
    );
}

/// A runner that fails a file naming a missing target the way the link
/// rule does, and passes anything else.
fn fake_runner(fixture: &Fixture, fires: bool) -> String {
    let script = fixture.state_root().parent().unwrap().join("runner.sh");
    let body = if fires {
        "grep -q missing \"$1\" && { echo \"$1: relative-links broken link\"; exit 1; }; exit 0\n"
    } else {
        "exit 0\n"
    };
    std::fs::write(&script, body).unwrap();
    format!("sh {}", script.display())
}

/// VERIFIES instance:the-root-lint-configuration-loads-relative-links
#[test]
fn the_proof_passes_where_the_rule_fires_and_fails_where_it_does_not() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let before = fixture.tree_digest();

    let output = fixture
        .cmd()
        .env("SDD_LINT_PROVE_RUNNER", fake_runner(&fixture, true))
        .args(["lint", "prove", "--target", &fixture.target(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let proof: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(proof["ok"], true, "{proof}");
    assert_eq!(proof["broken"]["cites_rule"], true);
    assert_eq!(proof["valid"]["exit"], 0);
    assert_eq!(fixture.tree_digest(), before, "the scratch file stayed");

    fixture
        .cmd()
        .env("SDD_LINT_PROVE_RUNNER", fake_runner(&fixture, false))
        .args(["lint", "prove", "--target", &fixture.target()])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("FAIL relative-links"));
    assert_eq!(fixture.tree_digest(), before, "the scratch file stayed");

    fixture
        .cmd()
        .env("SDD_LINT_PROVE_RUNNER", "no-such-program-for-the-proof")
        .args(["lint", "prove", "--target", &fixture.target()])
        .assert()
        .code(69);
    assert_eq!(fixture.tree_digest(), before, "the scratch file stayed");
}
