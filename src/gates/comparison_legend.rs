//! Gate: a comparison document carrying tables carries a legend.
//!
//! The verdict symbols mean nothing without the line that defines them.
//! Only presence is judged; which symbols the legend defines is the verdict
//! gate's business.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::comparison_dated_tables::has_table;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::ComparisonCarriesALegend];

/// Judge every file pre-commit passed.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a file cannot be read.
pub fn run(ctx: &GateCtx, files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for file in files {
        let text = read_text(ctx, file)?;
        if has_table(&text) && !text.lines().any(|line| line.starts_with("Legend: ")) {
            violations.push(Violation::Finding(Finding::on_file(
                RuleId::ComparisonCarriesALegend,
                file,
                "",
            )));
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
    fn accepts_a_table_with_a_legend() {
        assert!(run_on("Legend: ✅ yes.\n\n| a |\n").is_empty());
    }

    #[test]
    fn rejects_a_table_without_a_legend() {
        let out = run_on("| a |\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("comparison-docs:a-comparison-carries-a-legend"));
    }

    #[test]
    fn a_tableless_document_needs_no_legend() {
        assert!(run_on("# Notes\n").is_empty());
    }
}
