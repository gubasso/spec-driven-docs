//! Gate: every rule whose verification runs a hook names a hook that is
//! still defined.
//!
//! A rule can be stated in a spec, given a `Verify:` line, and enforced by
//! nothing. This reads the hook name out of every verification that runs
//! a hook through prek or pre-commit and asserts the hook is defined, so
//! renaming or deleting one fails here instead of quietly turning a rule
//! back into a suggestion. Only a `Verify:` line binds: a command in prose
//! or in a scenario is an example.
//! Aliases count, because that is how a spec names a scoped variant.
//!
//! A configuration with a `repo: builtin` entry loads only in prek, so
//! there a verification that runs `pre-commit run` runs nothing, and the
//! gate refuses it.

use std::collections::BTreeSet;

use crate::domain::finding::Finding;
use crate::domain::paths::HOOKS_CONFIG_PATH;
use crate::domain::rule_id::RuleId;

use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::VerificationNamesALiveHook];

/// The invocations that run one hook, one for each runner the gates accept.
const RUNS: [&str; 2] = ["prek run ", "pre-commit run "];

/// The invocation of the runner that cannot load a `repo: builtin` entry.
const PRE_COMMIT_RUN: &str = "pre-commit run ";

/// The hook every runner invocation of every `Verify:` line names.
fn hook_names_in(text: &str) -> impl Iterator<Item = String> + '_ {
    text.lines()
        .filter_map(|line| line.trim_start().strip_prefix("Verify:"))
        .flat_map(hook_names_in_verification)
}

/// The hooks one verification runs, in the order its invocations appear.
fn hook_names_in_verification(verification: &str) -> Vec<String> {
    RUNS.into_iter()
        .flat_map(|run| verification.match_indices(run))
        .filter(|(index, _)| starts_a_word(verification, *index))
        .filter_map(|(index, needle)| hook_argument(&verification[index + needle.len()..]))
        .collect()
}

/// Whether a runner name at `index` is the runner itself, not the tail of a
/// longer word such as `xprek`.
fn starts_a_word(text: &str, index: usize) -> bool {
    text[..index]
        .chars()
        .next_back()
        .is_none_or(|c| !(c.is_alphanumeric() || matches!(c, '-' | '_' | '.')))
}

/// The whole first argument after the runner, or `None` where it is an
/// option, which names no hook.
fn hook_argument(after: &str) -> Option<String> {
    let name: String = after
        .chars()
        .take_while(|c| {
            !c.is_whitespace() && !matches!(c, '`' | ';' | '&' | '|' | ')' | '\'' | '"')
        })
        .collect();
    (!name.is_empty() && !name.starts_with('-')).then_some(name)
}

/// The 1-based number of every `Verify:` line that runs pre-commit.
fn pre_commit_verifications(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let verification = line.trim_start().strip_prefix("Verify:")?;
            verification
                .match_indices(PRE_COMMIT_RUN)
                .any(|(at, _)| starts_a_word(verification, at))
                .then_some(index + 1)
        })
        .collect()
}

/// Whether the configuration declares a prek built-in repository, which
/// pre-commit cannot load.
fn holds_a_builtin_repo(config: &str) -> bool {
    config.lines().any(|line| {
        let trimmed = line.trim_start();
        let entry = trimmed.strip_prefix("- ").unwrap_or(trimmed).trim_end();
        entry
            .strip_prefix("repo:")
            .is_some_and(|value| value.trim().trim_matches(['\'', '"']) == "builtin")
    })
}

fn defines_hook(config: &str, hook: &str) -> bool {
    config.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed
            .strip_prefix("- id: ")
            .or_else(|| trimmed.strip_prefix("alias: "))
            .is_some_and(|value| value == hook)
    })
}

/// Judge every hook the local specs cite.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a spec or the pre-commit
/// configuration cannot be read.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    // Both layout predicates read the unfiltered set. A project that
    // reserves every spec still has a layout and still names hooks; it has
    // asked this gate to judge none of them, which is an answer rather than
    // a moved layout.
    let Some(all_specs) = crate::gates::spec_rule_id_unique::spec_files(ctx) else {
        return Ok(vec![Violation::Layout(
            "no specs matched; the layout moved".to_string(),
        )]);
    };
    let mut declared: BTreeSet<String> = BTreeSet::new();
    for file in &all_specs {
        declared.extend(hook_names_in(&read_text(ctx, file)?));
    }
    if declared.is_empty() {
        return Ok(vec![Violation::Layout(
            "no spec names a hook; the Verify shape moved".to_string(),
        )]);
    }

    let config = read_text(ctx, HOOKS_CONFIG_PATH)?;
    let builtin = holds_a_builtin_repo(&config);
    let mut hooks: BTreeSet<String> = BTreeSet::new();
    let mut refused: Vec<Violation> = Vec::new();
    for file in ctx.retained(all_specs) {
        let text = read_text(ctx, &file)?;
        hooks.extend(hook_names_in(&text));
        if builtin {
            refused.extend(pre_commit_verifications(&text).into_iter().map(|line| {
                Violation::Finding(Finding::on_line(
                    RuleId::VerificationNamesALiveHook,
                    file.as_str(),
                    line,
                    "the hook configuration holds a `repo: builtin` entry, which \
                     pre-commit cannot load; run the hook as `prek run <hook>`",
                ))
            }));
        }
    }
    Ok(hooks
        .into_iter()
        .filter(|hook| !defines_hook(&config, hook))
        .map(|hook| Violation::Finding(Finding::global(RuleId::VerificationNamesALiveHook, hook)))
        .chain(refused)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(hook_id: &str) -> tempfile::TempDir {
        fixture_running(hook_id, "pre-commit run")
    }

    fn fixture_running(hook_id: &str, run: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let specs = dir.path().join("_docs/specs");
        std::fs::create_dir_all(&specs).unwrap();
        std::fs::write(
            specs.join("SPEC-sample.md"),
            format!("### `sample:works` — Works\n\nVerify: `{run} sample-hook --all-files`\n"),
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".pre-commit-config.yaml"),
            format!(
                "repos:\n  - repo: local\n    hooks:\n      - id: {hook_id}\n        entry: true\n"
            ),
        )
        .unwrap();
        dir
    }

    fn run_in(dir: &tempfile::TempDir) -> Vec<String> {
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_a_cited_hook_that_exists() {
        assert!(run_in(&fixture("sample-hook")).is_empty());
    }

    /// Both runners name a hook the same way, so the gate reads both forms.
    #[test]
    fn reads_a_hook_named_through_either_runner() {
        for run in ["prek run", "pre-commit run"] {
            assert!(
                run_in(&fixture_running("sample-hook", run)).is_empty(),
                "{run}"
            );
            assert_eq!(
                run_in(&fixture_running("renamed-hook", run)),
                vec!["FAIL docs-specs:verification-names-a-live-hook: sample-hook".to_string()],
                "{run}"
            );
        }
    }

    #[test]
    fn rejects_a_cited_hook_that_was_renamed() {
        let out = run_in(&fixture("renamed-hook"));
        assert_eq!(
            out,
            vec!["FAIL docs-specs:verification-names-a-live-hook: sample-hook".to_string()]
        );
    }

    #[test]
    fn a_suffixed_rename_does_not_satisfy_the_citation() {
        assert_eq!(run_in(&fixture("sample-hook-v2")).len(), 1);
    }

    /// One spec and one configuration, written verbatim.
    fn fixture_with(spec: &str, hook_ids: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let specs = dir.path().join("_docs/specs");
        std::fs::create_dir_all(&specs).unwrap();
        std::fs::write(specs.join("SPEC-sample.md"), spec).unwrap();
        let mut hooks = String::new();
        for id in hook_ids {
            hooks.push_str("      - id: ");
            hooks.push_str(id);
            hooks.push_str("\n        entry: true\n");
        }
        std::fs::write(
            dir.path().join(".pre-commit-config.yaml"),
            format!("repos:\n  - repo: local\n    hooks:\n{hooks}"),
        )
        .unwrap();
        dir
    }

    /// A command in prose is an example, not a verification: only a
    /// `Verify:` line binds a hook name.
    #[test]
    fn a_command_in_prose_is_not_a_verification() {
        for run in ["prek run", "pre-commit run"] {
            let spec = format!(
                "### `sample:works` — Works\n\nA retired command is `{run} retired-example --all-files`.\n\n- GIVEN a rule whose `Verify:` line runs `{run} retired-scenario`\n\nVerify: `{run} sample-hook --all-files`\n"
            );
            assert!(
                run_in(&fixture_with(&spec, &["sample-hook"])).is_empty(),
                "{run}"
            );
        }
    }

    /// A runner name inside a longer word is not the runner.
    #[test]
    fn a_runner_name_inside_a_word_is_not_the_runner() {
        let spec = "### `sample:works` — Works\n\nVerify: `rg -q xprek run-missing && sh -c 'notpre-commit run other-missing' && prek run sample-hook`\n";
        assert!(run_in(&fixture_with(spec, &["sample-hook"])).is_empty());
    }

    /// The whole hook argument is the name: an underscore or a capital is
    /// part of it, so a prefix that happens to be defined satisfies nothing.
    #[test]
    fn the_whole_hook_argument_is_the_name() {
        for run in ["prek run", "pre-commit run"] {
            let defined =
                format!("### `sample:works` — Works\n\nVerify: `{run} Sample_hook --all-files`\n");
            assert!(
                run_in(&fixture_with(&defined, &["Sample_hook"])).is_empty(),
                "{run}"
            );
            let missing = format!(
                "### `sample:works` — Works\n\nVerify: `{run} sample-hook_missing --all-files`\n"
            );
            assert_eq!(
                run_in(&fixture_with(&missing, &["sample-hook"])),
                vec![
                    "FAIL docs-specs:verification-names-a-live-hook: sample-hook_missing"
                        .to_string()
                ],
                "{run}"
            );
        }
    }

    /// Every runner invocation of one verification names a hook, and an
    /// invocation whose first argument is an option names none.
    #[test]
    fn every_invocation_of_a_verification_is_judged() {
        let spec = "### `sample:works` — Works\n\nVerify: `prek run sample-hook && pre-commit run second-missing && prek run --all-files`\n";
        assert_eq!(
            run_in(&fixture_with(spec, &["sample-hook"])),
            vec!["FAIL docs-specs:verification-names-a-live-hook: second-missing".to_string()]
        );
    }

    /// Append a prek built-in repository to a fixture's configuration.
    fn with_builtin_repo(dir: &tempfile::TempDir, repo_line: &str) {
        let config = dir.path().join(".pre-commit-config.yaml");
        let text = std::fs::read_to_string(&config).unwrap()
            + &format!("  {repo_line}\n    hooks:\n      - id: check-yaml\n");
        std::fs::write(&config, text).unwrap();
    }

    /// pre-commit cannot load a `repo: builtin` entry, so there a
    /// verification that runs pre-commit runs nothing.
    #[test]
    fn a_builtin_repo_refuses_a_pre_commit_verification() {
        for repo_line in [
            "- repo: builtin",
            "- repo: 'builtin'",
            "- repo: \"builtin\"",
        ] {
            let dir = fixture("sample-hook");
            with_builtin_repo(&dir, repo_line);
            assert_eq!(
                run_in(&dir),
                vec![
                    "FAIL docs-specs:verification-names-a-live-hook _docs/specs/SPEC-sample.md:3: \
                     the hook configuration holds a `repo: builtin` entry, which pre-commit \
                     cannot load; run the hook as `prek run <hook>`"
                        .to_string()
                ],
                "{repo_line}"
            );
        }
    }

    #[test]
    fn a_builtin_repo_accepts_a_prek_verification() {
        let dir = fixture_running("sample-hook", "prek run");
        with_builtin_repo(&dir, "- repo: builtin");
        assert!(run_in(&dir).is_empty());
    }

    /// Without a built-in repository both runners load the configuration.
    #[test]
    fn a_pre_commit_verification_passes_without_a_builtin_repo() {
        let dir = fixture("sample-hook");
        with_builtin_repo(&dir, "- repo: builtin-hooks");
        assert!(run_in(&dir).is_empty());
    }

    /// Only a `Verify:` line binds, and a runner name inside a word is not
    /// the runner, under a built-in repository as everywhere else.
    #[test]
    fn a_builtin_repo_judges_only_the_pre_commit_runner_of_a_verify_line() {
        let spec = "### `sample:works` — Works\n\nThe old form was `pre-commit run sample-hook`.\n\nVerify: `sh -c 'notpre-commit run x' && prek run sample-hook`\n";
        let dir = fixture_with(spec, &["sample-hook"]);
        with_builtin_repo(&dir, "- repo: builtin");
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn an_alias_satisfies_the_citation() {
        let dir = fixture("other");
        let config = dir.path().join(".pre-commit-config.yaml");
        let text = std::fs::read_to_string(&config).unwrap() + "        alias: sample-hook\n";
        std::fs::write(&config, text).unwrap();
        assert!(run_in(&dir).is_empty());
    }
}
