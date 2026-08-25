//! Gate: every verdict symbol carries the word the legend gives it.
//!
//! The match is per occurrence, not per line: filtering whole lines
//! exonerates a row for all of its cells as soon as one cell is annotated,
//! and a row with several subject columns is exactly where the bare verdict
//! hides. Which symbols exist is the legend's business; this gate holds each
//! occurrence to its word.

use std::sync::LazyLock;

use regex::Regex;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::VerdictCarriesItsWord];

static VERDICT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("(✅|⚠️|❌|➖|🧪|❓)( [a-z/]+)?").unwrap_or_else(|_| unreachable!())
});

const ANNOTATED: &[&str] = &[
    "✅ yes",
    "⚠️ partial",
    "❌ no",
    "➖ n/a",
    "🧪 unstable",
    "❓ untested",
];

/// Judge every file pre-commit passed.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a file cannot be read.
pub fn run(ctx: &GateCtx, files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for file in files {
        for (number, line) in read_text(ctx, file)?.lines().enumerate() {
            for hit in VERDICT.find_iter(line) {
                if !ANNOTATED.contains(&hit.as_str()) {
                    violations.push(Violation::Finding(Finding::on_line(
                        RuleId::VerdictCarriesItsWord,
                        file,
                        number + 1,
                        hit.as_str(),
                    )));
                }
            }
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_on(text: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("comparison.md"), text).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &["comparison.md".to_string()])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_every_annotated_verdict() {
        assert!(
            run_on("| ✅ yes | ⚠️ partial | ❌ no | ➖ n/a | 🧪 unstable | ❓ untested |\n")
                .is_empty()
        );
    }

    #[test]
    fn rejects_a_bare_verdict() {
        let out = run_on("| Runs | ✅ |\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("comparison-docs:a-verdict-carries-its-word"));
        assert!(out[0].ends_with(": ✅"));
    }

    #[test]
    fn rejects_a_bare_verdict_beside_an_annotated_one() {
        let out = run_on("| Runs | ✅ yes | ❌ |\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": ❌"));
    }

    #[test]
    fn rejects_a_wrong_word() {
        let out = run_on("| Runs | ✅ maybe |\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": ✅ maybe"));
    }
}
