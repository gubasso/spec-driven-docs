//! Gate: a case id is `KI-<slug>.md`, and the slug is not a counter.
//!
//! The id a suppression cites has to survive a merge, so it names the bug
//! rather than its position in a queue. A digit inside the slug is ordinary
//! — an upstream issue number belongs to the story it tells — but a slug
//! that opens with one is the counter this rejects.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::CaseIdIsASlug];

const RULE: RuleId = RuleId::CaseIdIsASlug;

/// Judge every file pre-commit passed.
///
/// # Errors
///
/// None; the gate reads only the argument paths themselves.
pub fn run(_ctx: &GateCtx, files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for file in files {
        let basename = file.rsplit('/').next().unwrap_or(file);
        let Some(after_prefix) = basename.strip_prefix("KI-") else {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                file,
                "no KI- prefix",
            )));
            continue;
        };
        let Some(slug) = after_prefix.strip_suffix(".md") else {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                file,
                "not a markdown file",
            )));
            continue;
        };
        if slug.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                file,
                "a slug, not a counter",
            )));
        } else if slug.is_empty()
            || !slug
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                file,
                "the slug is lowercase, digits and hyphens",
            )));
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_on(files: &[&str]) -> Vec<String> {
        let ctx = GateCtx::new(".");
        let files: Vec<String> = files.iter().map(ToString::to_string).collect();
        run(&ctx, &files)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_a_slug_with_an_inner_digit() {
        assert!(run_on(&["_docs/reference/known-issues/KI-vendor-500.md"]).is_empty());
    }

    #[test]
    fn rejects_a_counter() {
        let out = run_on(&["_docs/reference/known-issues/KI-001-vendor.md"]);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("known-issues:case-id-is-a-slug"));
        assert!(out[0].ends_with(": a slug, not a counter"));
    }

    #[test]
    fn rejects_missing_prefix_wrong_extension_and_bad_charset() {
        assert!(run_on(&["x/ISSUE-a.md"])[0].ends_with(": no KI- prefix"));
        assert!(run_on(&["x/KI-a.txt"])[0].ends_with(": not a markdown file"));
        assert!(
            run_on(&["x/KI-Weird.md"])[0].ends_with(": the slug is lowercase, digits and hyphens")
        );
    }
}
