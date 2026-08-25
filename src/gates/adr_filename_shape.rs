//! Gate: a record filename is `ADR-<slug>.md`, and the slug carries no digit.
//!
//! The slug is the identifier: a counter lets two branches each allocate the
//! next number, and the merge leaves one identity claimed twice. Only the
//! filenames pre-commit hands over are judged; record content is other
//! gates' business.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::FilenameCarriesNoDigit];

const RULE: RuleId = RuleId::FilenameCarriesNoDigit;

/// Judge every file pre-commit passed.
///
/// # Errors
///
/// None; the gate reads only the argument paths themselves.
pub fn run(_ctx: &GateCtx, files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for file in files {
        let basename = file.rsplit('/').next().unwrap_or(file);
        if basename == "TEMPLATE-adr.md" {
            continue;
        }
        let Some(after_prefix) = basename.strip_prefix("ADR-") else {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                file,
                "no ADR- prefix",
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
        if slug.is_empty() || !slug.bytes().all(|b| b.is_ascii_lowercase() || b == b'-') {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                file,
                "the slug is lowercase and hyphens, with no digit",
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
    fn accepts_a_slugged_record_and_the_template() {
        assert!(run_on(&["_docs/decisions/ADR-use-slugs.md"]).is_empty());
        assert!(run_on(&["_docs/decisions/TEMPLATE-adr.md"]).is_empty());
    }

    #[test]
    fn rejects_a_digit_in_the_slug() {
        let out = run_on(&["_docs/decisions/ADR-use-v2.md"]);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("decision-records:filename-carries-no-digit"));
        assert!(out[0].contains("the slug is lowercase and hyphens, with no digit"));
    }

    #[test]
    fn rejects_missing_prefix_and_wrong_extension() {
        assert!(run_on(&["_docs/decisions/RECORD-x.md"])[0].contains("no ADR- prefix"));
        assert!(run_on(&["_docs/decisions/ADR-x.txt"])[0].contains("not a markdown file"));
    }
}
