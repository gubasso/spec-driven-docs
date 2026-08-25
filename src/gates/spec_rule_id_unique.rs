//! Gate: a rule ID resolves to exactly one requirement across the project.
//!
//! A commit citing a duplicated ID names two rules at once, so the citation
//! stops being an address. Corpus-wide by construction: the duplicate is
//! only visible when every spec is read together.

use std::collections::BTreeMap;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::docs_root;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::RuleIdIsUniqueAndSlugged];

/// List the spec documents under the documentation root, or `None` when the
/// layout has moved.
pub(crate) fn spec_files(ctx: &GateCtx) -> Option<Vec<camino::Utf8PathBuf>> {
    let specs = docs_root(ctx).join("specs");
    let mut names: Vec<String> = ctx
        .path(&specs)
        .read_dir_utf8()
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string())
        .filter(|name| {
            name.strip_prefix("SPEC-")
                .and_then(|rest| rest.strip_suffix(".md"))
                .is_some_and(|slug| !slug.is_empty())
        })
        .collect();
    if names.is_empty() {
        return None;
    }
    names.sort();
    Some(names.into_iter().map(|name| specs.join(name)).collect())
}

/// Judge the whole spec corpus.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a spec cannot be read.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    let Some(files) = spec_files(ctx) else {
        return Ok(vec![Violation::Layout(
            "no specs matched; the layout moved".to_string(),
        )]);
    };
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for file in files {
        for id in crate::embedded::rule_ids_in(&read_text(ctx, &file)?) {
            *counts.entry(id).or_default() += 1;
        }
    }
    let duplicated: Vec<String> = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(id, _)| format!("### `{id}`"))
        .collect();
    if duplicated.is_empty() {
        return Ok(vec![]);
    }
    let mut violations = vec![Violation::Finding(Finding::global(
        RuleId::RuleIdIsUniqueAndSlugged,
        "",
    ))];
    violations.extend(duplicated.into_iter().map(Violation::Note));
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &tempfile::TempDir, name: &str, text: &str) {
        let specs = dir.path().join("_docs/specs");
        std::fs::create_dir_all(&specs).unwrap();
        std::fs::write(specs.join(name), text).unwrap();
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
    fn accepts_unique_ids_across_the_corpus() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "SPEC-a.md", "### `a:one` — One\n");
        write(&dir, "SPEC-b.md", "### `b:one` — One\n");
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn rejects_a_duplicate_and_names_it() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "SPEC-a.md", "### `sample:works` — Works\n");
        write(&dir, "SPEC-b.md", "### `sample:works` — Works again\n");
        let out = run_in(&dir);
        assert_eq!(
            out,
            vec![
                "FAIL docs-specs:rule-id-is-unique-and-slugged".to_string(),
                "### `sample:works`".to_string(),
            ]
        );
    }

    #[test]
    fn a_missing_corpus_is_a_layout_failure() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            run_in(&dir),
            vec!["FAIL no specs matched; the layout moved".to_string()]
        );
    }
}
