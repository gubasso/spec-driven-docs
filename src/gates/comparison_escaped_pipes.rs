//! Gate: a pipe inside a comparison cell's code span is escaped.
//!
//! An unescaped `|` inside a code span splits the cell, and the table
//! silently gains a column. Only table lines are scanned, and only pipes
//! inside backtick spans are judged — a pipe between cells is the table.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::TablePipesAreEscaped];

fn unescaped_pipe_in_code(line: &str) -> bool {
    let mut code = false;
    let mut previous = '\0';
    for ch in line.chars() {
        if ch == '`' {
            code = !code;
        }
        if code && ch == '|' && previous != '\\' {
            return true;
        }
        previous = ch;
    }
    false
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
            if line.starts_with('|') && unescaped_pipe_in_code(line) {
                violations.push(Violation::Finding(Finding::on_line(
                    RuleId::TablePipesAreEscaped,
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
    fn accepts_an_escaped_pipe_in_a_code_span() {
        assert!(run_on("| Pipe | `a\\|b` |\n").is_empty());
    }

    #[test]
    fn rejects_an_unescaped_pipe_in_a_code_span() {
        let out = run_on("| Pipe | `a|b` |\n");
        assert_eq!(
            out,
            vec!["FAIL comparison-docs:table-pipes-are-escaped comparison.md:1".to_string()]
        );
    }

    #[test]
    fn pipes_between_cells_are_the_table() {
        assert!(run_on("| a | b |\n| --- | --- |\n").is_empty());
    }
}
