//! Gate: a comparison cell carries at most one reference.
//!
//! A cell with two links is two claims sharing one verdict, and the reader
//! cannot tell which reference backs which half. Cells are the fields
//! between the table's pipes; what a reference points at is review's
//! business.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::CellCarriesOneReference];

fn cell_has_extra_references(line: &str) -> bool {
    let fields: Vec<&str> = line.split('|').collect();
    if fields.len() < 3 {
        return false;
    }
    fields[1..fields.len() - 1]
        .iter()
        .any(|cell| cell.matches("](").count() > 1)
}

/// Judge every file pre-commit passed.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a file cannot be read.
pub fn run(ctx: &GateCtx, files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for file in files {
        for (number, line) in read_text(ctx, file)?.lines().enumerate() {
            if line.starts_with('|') && cell_has_extra_references(line) {
                violations.push(Violation::Finding(Finding::on_line(
                    RuleId::CellCarriesOneReference,
                    file,
                    number + 1,
                    "",
                )));
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
    fn accepts_one_reference_per_cell() {
        assert!(run_on("| [Case](#case) | Subject |\n| Runs | ✅ yes |\n").is_empty());
    }

    #[test]
    fn rejects_two_references_in_one_cell() {
        let out = run_on("| Runs [a](#a) [b](#b) | ✅ yes |\n");
        assert_eq!(
            out,
            vec!["FAIL comparison-docs:a-cell-carries-one-reference comparison.md:1".to_string()]
        );
    }
}
