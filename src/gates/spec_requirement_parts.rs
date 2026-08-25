//! Gate: every requirement in a spec is a rule ID heading that owns exactly
//! one verification.
//!
//! The check is per block, not per file: counting rule IDs and `Verify:`
//! lines over a whole spec and comparing totals passes a spec whose second
//! requirement has no verification and whose first has two. Each `###`
//! heading opens a block that runs to the next one or to end of file. The
//! other three parts of a requirement are heading structure, held by the
//! markdownlint spec profile.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::RequirementCarriesFiveParts];

const RULE: RuleId = RuleId::RequirementCarriesFiveParts;

fn heading_is_rule_id(heading: &str) -> bool {
    let Some(rest) = heading.strip_prefix("### `") else {
        return false;
    };
    let Some((id, after)) = rest.split_once('`') else {
        return false;
    };
    let Some((domain, rule)) = id.split_once(':') else {
        return false;
    };
    let is_slug = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    if !is_slug(domain) || !is_slug(rule) {
        return false;
    }
    let Some(separator) = after.strip_prefix(' ') else {
        return false;
    };
    let mut tokens = separator.splitn(2, ' ');
    let dash = tokens.next().unwrap_or_default();
    !dash.is_empty() && tokens.next().is_some()
}

fn judge(file: &str, text: &str, violations: &mut Vec<Violation>) {
    let mut heading: Option<(usize, String)> = None;
    let mut verifies = 0usize;
    let close =
        |heading: &Option<(usize, String)>, verifies: usize, violations: &mut Vec<Violation>| {
            let Some((line, text)) = heading else { return };
            if !heading_is_rule_id(text) {
                violations.push(Violation::Finding(Finding::on_line(
                    RULE,
                    file,
                    *line,
                    "heading is not a rule id",
                )));
            }
            if verifies != 1 {
                violations.push(Violation::Finding(Finding::on_line(
                    RULE,
                    file,
                    *line,
                    format!("{verifies} verifications, expected 1"),
                )));
            }
        };
    for (number, line) in text.lines().enumerate() {
        if line.starts_with("### ") {
            close(&heading, verifies, violations);
            heading = Some((number + 1, line.to_string()));
            verifies = 0;
        } else if line.starts_with("Verify: ") && heading.is_some() {
            verifies += 1;
        }
    }
    close(&heading, verifies, violations);
}

/// Judge every file pre-commit passed.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a file cannot be read.
pub fn run(ctx: &GateCtx, files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for file in files {
        let text = read_text(ctx, file)?;
        judge(file, &text, &mut violations);
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = "# Sample Specification\n\n## Purpose\n\nRules.\n\n## Requirements\n\n### `sample:works` — Sample works\n\nThe sample MUST work.\n\n#### Scenario: Run\n\n- GIVEN input\n- WHEN run\n- THEN output\n\nVerify: `pre-commit run sample-hook --all-files`\n";

    fn run_on(text: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SPEC-sample.md"), text).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &["SPEC-sample.md".to_string()])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_a_conforming_requirement_block() {
        assert!(run_on(SPEC).is_empty());
    }

    #[test]
    fn rejects_a_block_missing_its_verification() {
        let without: String = SPEC
            .lines()
            .filter(|l| !l.starts_with("Verify:"))
            .collect::<Vec<_>>()
            .join("\n");
        let out = run_on(&without);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("docs-specs:requirement-carries-five-parts"));
        assert!(out[0].ends_with(": 0 verifications, expected 1"));
    }

    #[test]
    fn rejects_a_heading_that_is_not_a_rule_id() {
        let out = run_on("### Just a heading\n\nVerify: `true`\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": heading is not a rule id"));
    }

    #[test]
    fn each_block_owns_its_own_verification() {
        let two_blocks =
            "### `a:one` — One\n\nVerify: `x`\n\nVerify: `y`\n\n### `a:two` — Two\n\nProse.\n";
        let out = run_on(two_blocks);
        assert_eq!(out.len(), 2);
        assert!(out[0].contains(":1: 2 verifications, expected 1"));
        assert!(out[1].contains(":7: 0 verifications, expected 1"));
    }
}
