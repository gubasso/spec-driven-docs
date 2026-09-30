//! Integration: what a landing writes passes the gates that landing delivered.
//!
//! The symptom this proves against: a landing reported success, `sdd verify`
//! printed OK, and the project's first hook run then failed on the payload's
//! own files. An operator who applies a landing has to be able to commit it.

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

use support::Fixture;

/// Every delivered gate, run against the tree the landing just wrote.
///
/// The gates run through `sdd gate` rather than through pre-commit: the
/// subject is the landed tree, and pre-commit would add its own network
/// fetch and its own environment to a question about bytes on disk.
fn every_gate_over(fixture: &Fixture) -> Vec<String> {
    let mut failures = Vec::new();
    for gate in spec_driven_docs::gates::GATES {
        let id = gate.id.as_str();
        // A gate judges the working directory, so the run happens in the
        // landed tree rather than wherever the suite started.
        let assert = fixture
            .cmd()
            .current_dir(fixture.path())
            .args(["gate", id])
            .assert();
        let output = assert.get_output();
        if !output.status.success() {
            failures.push(format!(
                "{id}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    failures
}

/// VERIFIES distribution:initialization-preserves-project-content
#[test]
fn a_landing_into_an_empty_repository_passes_every_gate_it_delivered() {
    for profile in ["codebase", "knowledge-base"] {
        let fixture = Fixture::new();
        fixture.install(profile);

        let failures = every_gate_over(&fixture);
        assert!(
            failures.is_empty(),
            "a fresh {profile} landing cannot be committed:\n{}",
            failures.join("\n")
        );
    }
}

/// The commented example in the landed registry, uncommented into its
/// `tracked:` list, as an adopter following the comment would write it.
fn uncommented_example(registry: &str) -> String {
    let entry: String = registry
        .lines()
        .filter(|line| line.starts_with("#   "))
        .map(|line| format!("{}\n", &line[1..]))
        .collect();
    assert!(!entry.is_empty(), "the landed registry carries no example");
    registry.replace("tracked: []\n", &format!("tracked:\n{entry}"))
}

/// Every delivered tracking example passes the schema, the parser, and the
/// gate, in the instance it landed in.
///
/// VERIFIES staging:a-seed-lands-rendered-for-its-root
#[test]
fn every_landed_tracking_example_passes_the_gate() {
    for (profile, root) in [("codebase", "docs"), ("knowledge-base", "_docs")] {
        let fixture = Fixture::new();
        fixture.install(profile);
        let relative = format!("{root}/reference/tracking.yaml");
        let registry = uncommented_example(&fixture.read(&relative));
        assert!(
            !registry.contains("{docs_root}"),
            "{profile}: a placeholder survived the landing:\n{registry}"
        );
        fixture.write(&relative, &registry);

        let parsed = spec_driven_docs::domain::tracking::parse(&registry)
            .unwrap_or_else(|error| panic!("{profile}: the example does not parse: {error}"));
        assert_eq!(parsed.tracked.len(), 1, "{profile}: one example entry");
        let entry = &parsed.tracked[0];
        for path in std::iter::once(&entry.path).chain(&entry.dependents) {
            assert!(
                path.starts_with(&format!("{root}/")),
                "{profile}: {path} does not start at the documentation root {root}/"
            );
            fixture.write(path, "# Tracked\n");
        }

        let schema = fixture
            .path()
            .join(format!("{root}/specs/SPEC-tracking/tracking.schema.json"));
        let output = std::process::Command::new("check-jsonschema")
            .arg("--schemafile")
            .arg(&schema)
            .arg(fixture.path().join(&relative))
            .output()
            .unwrap_or_else(|error| {
                panic!("check-jsonschema did not run: {error}; it comes from the devshell")
            });
        assert!(
            output.status.success(),
            "{profile}: the example fails the schema:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let as_of: jiff::civil::Date = entry.last_checked.parse().unwrap();
        let report = spec_driven_docs::services::tracking::evaluate(
            camino::Utf8Path::from_path(fixture.path()).unwrap(),
            camino::Utf8Path::new(root),
            as_of,
        )
        .unwrap();
        assert!(
            !report.has_failures(),
            "{profile}: the gate refuses the example: {:?} {:?}",
            report.fatal,
            report
                .entries
                .iter()
                .flat_map(|assessed| &assessed.problems)
                .collect::<Vec<_>>()
        );
    }
}

/// VERIFIES distribution:initialization-preserves-project-content
#[test]
fn a_fresh_landing_verifies_and_reports_no_drift() {
    let fixture = Fixture::new();
    fixture.install("codebase");

    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
}

/// VERIFIES docs-specs:verification-names-a-live-hook
///
/// Every `Verify:` line a seeded spec carries must name something the
/// adopter can run. A hook the landing does not wire is work the project
/// cannot do, whatever the sentence above it says.
#[test]
fn every_seeded_verification_names_a_hook_the_landing_wires() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");

    let block = fixture.read(".pre-commit-config.yaml");
    let wired: Vec<String> = block
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- id: ").map(str::to_string))
        .chain(
            block
                .lines()
                .filter_map(|line| line.trim().strip_prefix("alias: ").map(str::to_string)),
        )
        .collect();

    let specs = std::fs::read_dir(fixture.path().join("_docs/specs")).unwrap();
    let mut missing = Vec::new();
    for entry in specs.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_none_or(|kind| kind != "md") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("Verify: `pre-commit run ") else {
                continue;
            };
            let Some(hook) = rest.split_whitespace().next() else {
                continue;
            };
            if !wired.iter().any(|id| id == hook) {
                missing.push(format!("{}: {hook}", path.display()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "a seeded verification names a hook the landing does not wire:\n{}",
        missing.join("\n")
    );
}

/// A Markdown file whose headings follow the spec shape.
const SPEC_SHAPE: &str =
    "# SPEC a\n\n## Purpose\n\nText.\n\n## Requirements\n\n### Requirement: a\n\nText.\n";

/// A spec whose second section is not the one the shape requires.
const SPEC_WRONG: &str = "# SPEC a\n\n## Purpose\n\nText.\n\n## Wrong\n\nText.\n";

/// A record carrying two of the five sections.
const RECORD_WRONG: &str =
    "# ADR a\n\n## Context and Problem Statement\n\nText.\n\n## Status\n\nAccepted.\n";

/// What one linter run printed, and whether it passed.
struct Lint {
    passed: bool,
    report: String,
}

/// Run the delivered linter the way a developer or an editor does: from the
/// target root, reading configuration through its own discovery, with no
/// `--config` and no module path.
///
/// The binary comes from the devshell, which bundles the relative-links
/// rule. Where it is absent the test fails rather than passes: a proof that
/// runs nowhere proves nothing.
fn lint(fixture: &Fixture, args: &[&str]) -> Lint {
    let output = std::process::Command::new("markdownlint-cli2")
        .current_dir(fixture.path())
        .env_remove("NODE_PATH")
        .args(args)
        .output()
        .unwrap_or_else(|error| {
            panic!("markdownlint-cli2 did not run: {error}; it comes from the devshell")
        });
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !report.contains("Unable to import module"),
        "the linter cannot load the relative-links rule, so the bundled derivation lost it:\n{report}"
    );
    Lint {
        passed: output.status.success(),
        report,
    }
}

/// A landed configuration, parsed. The linter reads JSONC, and the landed
/// files carry comments on their own lines only.
fn jsonc(fixture: &Fixture, relative: &str) -> serde_json::Value {
    let text = fixture.read(relative);
    let stripped: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    serde_json::from_str(&stripped)
        .unwrap_or_else(|error| panic!("{relative} does not parse: {error}"))
}

/// VERIFIES distribution:a-delivered-configuration-serves-a-delivered-rule
#[test]
fn both_landed_lint_configurations_parse_and_merge() {
    for (profile, root) in [("codebase", "docs"), ("knowledge-base", "_docs")] {
        let fixture = Fixture::new();
        fixture.install(profile);

        let seed = jsonc(&fixture, ".markdownlint-cli2.jsonc");
        assert_eq!(
            seed["customRules"],
            serde_json::json!(["markdownlint-rule-relative-links"])
        );
        assert_eq!(seed["config"]["relative-links"], true);

        let managed = jsonc(&fixture, &format!("{root}/.markdownlint-cli2.jsonc"));
        assert_eq!(managed["config"]["MD013"], false);
        assert_eq!(managed["config"]["MD040"], true);
        let overrides = managed["overrides"].as_array().unwrap();
        let filters: Vec<&serde_json::Value> =
            overrides.iter().map(|entry| &entry["filter"]).collect();
        assert_eq!(
            filters,
            [
                &serde_json::json!(["specs/SPEC-*.md"]),
                &serde_json::json!(["decisions/ADR-*.md"])
            ]
        );
        for entry in overrides {
            assert_eq!(entry["combine"], "merge", "{entry}");
            assert_eq!(entry["config"]["MD043"]["match_case"], true, "{entry}");
        }
        assert_eq!(overrides[1]["config"]["relative-links"], false);
    }
}

/// VERIFIES distribution:a-delivered-configuration-serves-a-delivered-rule
///
/// The linter itself, over every Markdown file the landing wrote, the root
/// `AGENTS.md` included, with no file passed over and no rule disabled by
/// the test. The defect this holds against: a landing verified and the
/// project's first hook run failed on the payload's own files.
#[test]
fn a_fresh_landing_passes_the_native_lint_run() {
    for (profile, root) in [("codebase", "docs"), ("knowledge-base", "_docs")] {
        let fixture = Fixture::new();
        fixture.install(profile);
        assert!(
            fixture
                .read("AGENTS.md")
                .starts_with("# AGENTS\n\n<!-- BEGIN"),
            "{profile}: a created AGENTS.md opens with its title"
        );

        let run = lint(&fixture, &["**/*.md"]);
        assert!(
            run.passed,
            "a fresh {profile} landing fails the linter:\n{}",
            run.report
        );
        let version = run
            .report
            .split_whitespace()
            .find_map(|word| word.strip_prefix("v0."))
            .and_then(|rest| rest.split('.').next())
            .and_then(|minor| minor.parse::<u32>().ok())
            .expect("the linter prints its version");
        assert!(
            version >= 23,
            "the linter is older than 0.23.0:\n{}",
            run.report
        );
        let linted: usize = run
            .report
            .lines()
            .find_map(|line| line.strip_prefix("Linting: "))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|count| count.parse().ok())
            .expect("the linter reports what it linted");
        assert!(
            linted >= 15,
            "the run judged only {linted} files:\n{}",
            run.report
        );

        // The shapes still judge: without this the run above passes by
        // disabling everything.
        fixture.write(&format!("{root}/specs/SPEC-right.md"), SPEC_SHAPE);
        fixture.write(&format!("{root}/specs/SPEC-wrong.md"), SPEC_WRONG);
        fixture.write(&format!("{root}/decisions/ADR-wrong.md"), RECORD_WRONG);
        for held in ["specs", "decisions"] {
            fixture.write(&format!("{root}/{held}/README.md"), "# Readme\n\nText.\n");
        }
        let run = lint(&fixture, &["**/*.md"]);
        assert!(!run.passed);
        let shape_failures = run
            .report
            .lines()
            .filter(|line| line.contains("MD043"))
            .count();
        assert_eq!(shape_failures, 2, "{}", run.report);
        assert!(run.report.contains(&format!("{root}/specs/SPEC-wrong.md")));
        assert!(
            run.report
                .contains(&format!("{root}/decisions/ADR-wrong.md"))
        );
        for passing in [
            "SPEC-right.md",
            "TEMPLATE-spec.md",
            "TEMPLATE-adr.md",
            "README.md",
        ] {
            assert!(
                !run.report.lines().any(|line| line.contains(passing)),
                "{passing} was judged:\n{}",
                run.report
            );
        }
    }
}

/// VERIFIES distribution:a-delivered-configuration-serves-a-delivered-rule
///
/// A broken link fails citing the rule, not a module-import error, which
/// would mean the bundled linter lost the rule. The fixture sits outside the
/// linter's own closure and the run unsets `NODE_PATH`, so the rule resolves
/// from the linter's location or nowhere. Records are frozen history, so the
/// check does not judge their links.
#[test]
fn the_relative_links_rule_judges_links_outside_the_records() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    fixture.write("valid.md", "# Valid\n\nSee [the agents file](AGENTS.md).\n");
    fixture.write("broken.md", "# Broken\n\nSee [nothing](missing.md).\n");
    fixture.write(
        "docs/decisions/ADR-frozen.md",
        &RECORD_WRONG.replace("Text.", "See [gone](gone.md).").replace(
            "## Status",
            "## Considered Options\n\nText.\n\n## Decision Outcome\n\nText.\n\n## Consequences\n\nText.\n\n## Status",
        ),
    );

    let run = lint(&fixture, &["valid.md", "docs/decisions/ADR-frozen.md"]);
    assert!(run.passed, "{}", run.report);
    let run = lint(&fixture, &["broken.md"]);
    assert!(!run.passed);
    assert!(
        run.report.contains("broken.md:3 error relative-links"),
        "{}",
        run.report
    );
}

/// A host the project already keeps is the project's: the landing adds its
/// block and no title, and the tree still lints clean.
#[test]
fn a_landing_into_an_existing_agents_file_adds_no_title() {
    let fixture = Fixture::new();
    fixture.write("AGENTS.md", "# Project agents\n\nShort project text.\n");
    fixture.install("codebase");
    let host = fixture.read("AGENTS.md");
    assert!(
        host.starts_with("# Project agents\n\nShort project text.\n\n<!-- BEGIN"),
        "{host}"
    );
    assert!(!host.contains("# AGENTS\n"), "{host}");

    let run = lint(&fixture, &["**/*.md"]);
    assert!(run.passed, "{}", run.report);
}

/// The block's suppression is line-local: project prose before and after
/// the markers is still held to the project's line length.
#[test]
fn the_block_suppression_reaches_the_block_alone() {
    let long = "project prose ".repeat(8);
    let fixture = Fixture::new();
    fixture.write(
        "AGENTS.md",
        &format!("# Project agents\n\n{}\n", long.trim_end()),
    );
    fixture.install("codebase");
    let host = fixture.read("AGENTS.md");
    fixture.write("AGENTS.md", &format!("{host}\n{}\n", long.trim_end()));

    let run = lint(&fixture, &["AGENTS.md"]);
    let length: Vec<&str> = run
        .report
        .lines()
        .filter(|line| line.contains("MD013"))
        .collect();
    let lines = fixture.read("AGENTS.md").lines().count();
    assert_eq!(
        length,
        [
            "AGENTS.md:3:81 error MD013/line-length Line length [Expected: 80; Actual: 111]",
            format!("AGENTS.md:{lines}:81 error MD013/line-length Line length [Expected: 80; Actual: 111]").as_str(),
        ],
        "{}",
        run.report
    );
}

/// VERIFIES instance:the-lint-configuration-composes
///
/// Every root configuration a project may keep composes with the managed
/// one: a root that turns every default on, a complete root override for
/// the same spec, and a library-family file that tries to turn the shapes
/// off all leave them firing.
#[test]
fn a_project_root_configuration_cannot_turn_the_shapes_off() {
    let roots: [(&str, &str); 5] = [
        (
            ".markdownlint-cli2.jsonc",
            r#"{ "customRules": ["markdownlint-rule-relative-links"], "config": { "default": true, "MD013": true, "relative-links": true } }"#,
        ),
        (
            ".markdownlint-cli2.jsonc",
            r#"{ "customRules": ["markdownlint-rule-relative-links"], "config": { "relative-links": true }, "overrides": [ { "filter": ["docs/specs/SPEC-wrong.md"], "config": { "MD043": false }, "combine": "merge" } ] }"#,
        ),
        (
            ".markdownlint-cli2.jsonc",
            r#"{ "config": { "MD043": false } }"#,
        ),
        (".markdownlint.jsonc", r#"{ "MD043": false }"#),
        (".markdownlint.jsonc", r#"{ "default": false }"#),
    ];
    for (name, body) in roots {
        let fixture = Fixture::new();
        fixture.install("codebase");
        fixture.write(name, &format!("{body}\n"));
        fixture.write("docs/specs/SPEC-wrong.md", SPEC_WRONG);
        let run = lint(&fixture, &["docs/specs/SPEC-wrong.md"]);
        assert!(
            !run.passed && run.report.contains("MD043"),
            "a root {name} holding {body} turned the spec shape off:\n{}",
            run.report
        );
    }
}

/// The linter's scope lives in its own configuration: a file the root
/// `ignores` names is skipped even when a caller passes it by name, which is
/// what pre-commit does.
#[test]
fn a_path_the_root_ignores_is_skipped_when_passed_by_name() {
    let fixture = Fixture::new();
    fixture.install("codebase");
    let seed = fixture
        .read(".markdownlint-cli2.jsonc")
        .replace("\"ignores\": []", "\"ignores\": [\"vendor/**\"]");
    fixture.write(".markdownlint-cli2.jsonc", &seed);
    fixture.write(
        "vendor/notes.md",
        "no heading\n\n\n\nSee [x](missing.md).\n",
    );
    let run = lint(&fixture, &["vendor/notes.md"]);
    assert!(run.passed, "{}", run.report);
    assert!(run.report.contains("Linting: 0 files"), "{}", run.report);
}

/// One proposed root: its name, the files it writes, and whether the rule
/// loads under it.
type ProposedRoot = (&'static str, &'static [(&'static str, &'static str)], bool);

/// The configurations the setup skill proposes where the project's own root
/// configuration made the seed yield. `customRules` belongs to the CLI2
/// family alone, so a library-family file keeps its rules and a companion
/// CLI2 file carries the loader. The last case is the negative control: the
/// loader written into the library file is ignored, and the broken link
/// passes, which is why the companion is necessary. These test the proposed
/// configuration, not an edit `sdd` makes.
#[test]
fn the_proposed_relative_links_configurations_load_the_rule() {
    let cases: [ProposedRoot; 4] = [
        (
            "CLI2 only, loader merged",
            &[(
                ".markdownlint-cli2.jsonc",
                r#"{ "customRules": ["./rules/noop.cjs", "markdownlint-rule-relative-links"], "config": { "MD041": false, "relative-links": true } }"#,
            )],
            true,
        ),
        (
            "library only, companion loader",
            &[
                (".markdownlint.yaml", "MD041: false\nrelative-links: true\n"),
                (
                    ".markdownlint-cli2.jsonc",
                    r#"{ "customRules": ["markdownlint-rule-relative-links"] }"#,
                ),
            ],
            true,
        ),
        (
            "both families",
            &[
                (".markdownlint.yaml", "MD041: false\nrelative-links: true\n"),
                (
                    ".markdownlint-cli2.jsonc",
                    r#"{ "customRules": ["./rules/noop.cjs", "markdownlint-rule-relative-links"], "config": { "MD012": false } }"#,
                ),
            ],
            true,
        ),
        (
            "library only, loader in the wrong family",
            &[(
                ".markdownlint.yaml",
                "customRules:\n  - markdownlint-rule-relative-links\nMD041: false\nrelative-links: true\n",
            )],
            false,
        ),
    ];
    for (case, files, loads) in cases {
        let fixture = Fixture::new();
        fixture.write(
            "rules/noop.cjs",
            "module.exports = { names: [\"noop-rule\"], description: \"noop\", tags: [\"x\"], parser: \"none\", function: () => {} };\n",
        );
        for (name, body) in files {
            fixture.write(name, &format!("{body}\n"));
        }
        fixture.install("codebase");
        assert!(
            files
                .iter()
                .all(|(name, body)| fixture.read(name) == format!("{body}\n")),
            "{case}: the landing touched the project's configuration"
        );
        fixture.write(
            "valid.md",
            "No heading, which MD041 would refuse.\n\nSee [the agents file](AGENTS.md).\n",
        );
        fixture.write("broken.md", "# Broken\n\nSee [nothing](missing.md).\n");

        let run = lint(&fixture, &["valid.md"]);
        assert!(
            run.passed,
            "{case}: an unrelated rule or a valid link failed:\n{}",
            run.report
        );
        let run = lint(&fixture, &["broken.md"]);
        if loads {
            assert!(
                !run.passed && run.report.contains("broken.md:3 error relative-links"),
                "{case}: the broken link was not judged:\n{}",
                run.report
            );
        } else {
            assert!(
                run.passed,
                "{case}: the negative control judged the link:\n{}",
                run.report
            );
        }
    }
}
