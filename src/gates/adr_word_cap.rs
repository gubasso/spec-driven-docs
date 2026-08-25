//! Gate: a decision record's body stays within 350 words.
//!
//! Records are permanent, so their cost is paid on every read; the cap keeps
//! each one a decision rather than a chapter. The record set is read from
//! the documentation root — filename shape and heading structure belong to
//! other gates.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::docs_root;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::BodyStaysWithinWordCap];

const CAP: usize = 350;

/// Judge every decision record under the documentation root.
///
/// # Errors
///
/// [`GateError::Io`] when a matched record cannot be read.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    let decisions = docs_root(ctx).join("decisions");
    let mut records: Vec<String> = ctx
        .path(&decisions)
        .read_dir_utf8()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string())
                .filter(|name| {
                    name.strip_prefix("ADR-")
                        .and_then(|rest| rest.strip_suffix(".md"))
                        .is_some_and(|slug| !slug.is_empty())
                })
                .collect()
        })
        .unwrap_or_default();
    records.sort();
    if records.is_empty() {
        return Ok(vec![Violation::Layout(
            "no decision records matched".to_string(),
        )]);
    }

    let mut violations = Vec::new();
    for name in records {
        let path = decisions.join(name);
        let words = read_text(ctx, &path)?.split_whitespace().count();
        if words > CAP {
            violations.push(Violation::Finding(Finding::on_file(
                RuleId::BodyStaysWithinWordCap,
                path,
                words.to_string(),
            )));
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(words: usize) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let decisions = dir.path().join("_docs/decisions");
        std::fs::create_dir_all(&decisions).unwrap();
        std::fs::write(decisions.join("ADR-choice.md"), "word ".repeat(words)).unwrap();
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
    fn accepts_a_record_at_the_cap() {
        assert!(run_in(&fixture(350)).is_empty());
    }

    #[test]
    fn rejects_a_record_over_the_cap() {
        let out = run_in(&fixture(351));
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("decision-records:body-stays-within-350-words"));
        assert!(out[0].ends_with(": 351"));
    }

    #[test]
    fn an_empty_record_set_is_a_layout_failure() {
        let dir = tempfile::tempdir().unwrap();
        let out = run_in(&dir);
        assert_eq!(out, vec!["FAIL no decision records matched".to_string()]);
    }
}
