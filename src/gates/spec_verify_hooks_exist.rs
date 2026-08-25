//! Gate: every rule whose verification runs a hook names a hook that is
//! still defined.
//!
//! A rule can be stated in a spec, given a `Verify:` line, and enforced by
//! nothing. This reads the hook name out of every verification that runs
//! pre-commit and asserts the hook is defined, so renaming or deleting one
//! fails here instead of quietly turning a rule back into a suggestion.
//! Aliases count, because that is how a spec names a scoped variant.

use std::collections::BTreeSet;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::spec_rule_id_unique::spec_files;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::VerificationNamesALiveHook];

fn hook_names_in(text: &str) -> impl Iterator<Item = String> + '_ {
    text.match_indices("pre-commit run ")
        .filter_map(|(index, needle)| {
            let after = &text[index + needle.len()..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                .collect();
            (!name.is_empty()).then_some(name)
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
    let Some(files) = spec_files(ctx) else {
        return Ok(vec![Violation::Layout(
            "no specs matched; the layout moved".to_string(),
        )]);
    };
    let mut hooks: BTreeSet<String> = BTreeSet::new();
    for file in files {
        hooks.extend(hook_names_in(&read_text(ctx, &file)?));
    }
    if hooks.is_empty() {
        return Ok(vec![Violation::Layout(
            "no spec names a hook; the Verify shape moved".to_string(),
        )]);
    }
    let config = read_text(ctx, ".pre-commit-config.yaml")?;
    Ok(hooks
        .into_iter()
        .filter(|hook| !defines_hook(&config, hook))
        .map(|hook| Violation::Finding(Finding::global(RuleId::VerificationNamesALiveHook, hook)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(hook_id: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let specs = dir.path().join("_docs/specs");
        std::fs::create_dir_all(&specs).unwrap();
        std::fs::write(
            specs.join("SPEC-sample.md"),
            "### `sample:works` — Works\n\nVerify: `pre-commit run sample-hook --all-files`\n",
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

    #[test]
    fn an_alias_satisfies_the_citation() {
        let dir = fixture("other");
        let config = dir.path().join(".pre-commit-config.yaml");
        let text = std::fs::read_to_string(&config).unwrap() + "        alias: sample-hook\n";
        std::fs::write(&config, text).unwrap();
        assert!(run_in(&dir).is_empty());
    }
}
