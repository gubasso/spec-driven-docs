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

/// VERIFIES distribution:initialization-preserves-project-content
///
/// The gates the landing wires are not all its own. It also wires three
/// markdown-linter hooks, and the defect this suite exists to prevent was
/// exactly one of those failing after verification passed. Running the
/// linter itself needs pre-commit's own network fetch, which this suite
/// does not do, so what it holds is everything short of that: each hook
/// reads a configuration the same landing wrote, and each configuration
/// parses.
#[test]
fn every_linter_hook_reads_a_configuration_the_landing_wrote() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");
    let block = fixture.read(".pre-commit-config.yaml");

    let mut configs = 0;
    for line in block.lines() {
        let Some(rest) = line.trim().strip_prefix("args: ['--config', '") else {
            continue;
        };
        let Some(relative) = rest.split('\'').next() else {
            continue;
        };
        configs += 1;
        let path = fixture.path().join(relative);
        assert!(
            path.is_file(),
            "a wired hook reads {relative}, which the landing did not write"
        );
        // Parsed, not grepped: a broken token in a configuration the
        // landing wrote is exactly the failure this suite exists to catch,
        // and a substring check would pass over it. The linter reads JSONC,
        // so line comments come off before the parse.
        let text = std::fs::read_to_string(&path).unwrap();
        let stripped: String = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let parsed: serde_json::Value = serde_json::from_str(&stripped)
            .unwrap_or_else(|error| panic!("{relative} does not parse: {error}"));
        assert!(
            parsed.get("config").is_some(),
            "{relative} carries no linter configuration"
        );
    }
    assert_eq!(
        configs, 3,
        "the landing wired {configs} linter configurations"
    );

    // Every file the three hooks judge is a file the landing wrote, so a
    // pattern that matched nothing would be a silent pass.
    for judged in [
        "_docs/specs/SPEC-docs-format.md",
        "_docs/specs/SPEC-instance.md",
    ] {
        assert!(
            fixture.path().join(judged).is_file(),
            "{judged} is not in the landed tree, so the spec hook judges nothing"
        );
    }
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

/// One wired markdownlint hook: the configuration it loads and the paths it
/// judges. An entry with no `files:` judges every markdown file.
struct LinterHook {
    config: String,
    files: Option<String>,
}

/// Every markdownlint hook the landing wired, minus the ones whose
/// configuration names a custom rule.
///
/// A custom rule is an npm module pre-commit installs into the hook's own
/// environment. This suite installs nothing, so a hook that needs one is
/// out of its reach and stays out of its claims.
fn linter_hooks(fixture: &Fixture) -> Vec<LinterHook> {
    let block = fixture.read(".pre-commit-config.yaml");
    let mut hooks = Vec::new();
    let mut config: Option<String> = None;
    let mut files: Option<String> = None;
    let mut close = |config: &mut Option<String>, files: &mut Option<String>| {
        if let Some(config) = config.take() {
            hooks.push(LinterHook {
                config,
                files: files.take(),
            });
        } else {
            *files = None;
        }
    };
    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- id: ") {
            close(&mut config, &mut files);
        } else if let Some(rest) = trimmed.strip_prefix("args: ['--config', '") {
            config = rest.split('\'').next().map(str::to_string);
        } else if let Some(rest) = trimmed.strip_prefix("files: '") {
            files = rest
                .rsplit_once('\'')
                .map(|(pattern, _)| pattern.to_string());
        }
    }
    close(&mut config, &mut files);
    hooks.retain(|hook| !fixture.read(&hook.config).contains("customRules"));
    hooks
}

/// Every markdown path in the landed tree the pattern selects, relative to
/// the target and sorted, as pre-commit would pass them.
fn judged_paths(fixture: &Fixture, pattern: Option<&str>) -> Vec<String> {
    // pre-commit matches with `re.search`, so the pattern is unanchored
    // unless it anchors itself, which is what `is_match` does here.
    let selector = pattern.map(|pattern| {
        regex::Regex::new(pattern)
            .unwrap_or_else(|error| panic!("a wired hook carries an unreadable pattern: {error}"))
    });
    let mut paths = Vec::new();
    for entry in walkdir::WalkDir::new(fixture.path())
        .into_iter()
        .filter_entry(|entry| entry.file_name() != ".git")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let Ok(relative) = entry.path().strip_prefix(fixture.path()) else {
            continue;
        };
        let Some(relative) = relative.to_str() else {
            continue;
        };
        if !relative.ends_with(".md") {
            continue;
        }
        if selector.as_ref().is_none_or(|rule| rule.is_match(relative)) {
            paths.push(relative.to_string());
        }
    }
    paths.sort();
    paths
}

/// Run the delivered linter over the given paths, from inside the target.
///
/// The binary comes from the devshell. Where it is absent the test fails
/// rather than passes: a proof that runs nowhere proves nothing.
fn markdownlint(fixture: &Fixture, config: &str, paths: &[String]) -> std::process::Output {
    std::process::Command::new("markdownlint-cli2")
        .current_dir(fixture.path())
        .arg("--config")
        .arg(config)
        .args(paths)
        .output()
        .unwrap_or_else(|error| {
            panic!("markdownlint-cli2 did not run: {error}; it comes from the devshell")
        })
}

/// VERIFIES distribution:a-delivered-configuration-serves-a-delivered-rule
///
/// The linter itself, over the tree the landing wrote, with no
/// configuration the adopter owns. `every_linter_hook_reads_a_configuration_the_landing_wrote`
/// proves each configuration exists and parses, which is a different claim
/// from the linter accepting the payload's own files under it. The defect
/// this holds against: a delivered configuration left MD013 on, and the
/// unwrapped prose `docs-format` requires failed the hook the same landing
/// wired.
#[test]
fn every_delivered_configuration_passes_over_the_seeds_the_landing_wrote() {
    for profile in ["codebase", "knowledge-base"] {
        let fixture = Fixture::new();
        fixture.install(profile);
        assert!(
            !fixture.path().join(".markdownlint-cli2.jsonc").is_file(),
            "the landing wrote a base configuration, so this run would judge the merge rather than what the payload delivers"
        );

        let mut judged = 0;
        for hook in linter_hooks(&fixture) {
            let paths = judged_paths(&fixture, hook.files.as_deref());
            if paths.is_empty() {
                continue;
            }
            judged += paths.len();
            let output = markdownlint(&fixture, &hook.config, &paths);
            assert!(
                output.status.success(),
                "a fresh {profile} landing fails the hook reading {}:\n{}{}",
                hook.config,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        // A pattern that matched nothing would report every hook clean.
        assert!(
            judged > 0,
            "no landed file matched any wired linter pattern, so the {profile} run judged nothing"
        );
    }
}

/// VERIFIES distribution:a-delivered-configuration-serves-a-delivered-rule
///
/// The gate the configuration exists for survives the change that silenced
/// the defaults. Without this, the phase above passes by disabling
/// everything.
#[test]
fn a_wrong_heading_shape_still_fails_the_delivered_configuration() {
    let fixture = Fixture::new();
    fixture.install("knowledge-base");

    let spec = "_docs/specs/SPEC-instance.md";
    let text = fixture.read(spec).replace("## Purpose", "## Wrong");
    fixture.write(spec, &text);

    let hooks = linter_hooks(&fixture);
    let hook = hooks
        .iter()
        .find(|hook| {
            judged_paths(&fixture, hook.files.as_deref())
                .iter()
                .any(|path| path == spec)
        })
        .expect("no wired hook judges the seeded specs");
    let output = markdownlint(&fixture, &hook.config, &[spec.to_string()]);
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success() && report.contains("MD043"),
        "a spec with the wrong heading shape passed the shape gate:\n{report}"
    );
}
