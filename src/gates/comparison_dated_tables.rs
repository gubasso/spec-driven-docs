//! Gate: a comparison document carrying tables carries a verification date.
//!
//! A comparison is a set of claims about someone else's software, so its
//! tables decay; the `Verified:` line is what tells a reader how stale.
//! Only presence is judged — freshness policy is the comparison spec's
//! business.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::EveryTableIsDated];

fn has_verified_line(text: &str) -> bool {
    text.lines().any(|line| {
        let Some(rest) = line.strip_prefix("Verified: ") else {
            return false;
        };
        if rest.starts_with("`<YYYY-MM-DD>`") {
            return true;
        }
        let bytes = rest.as_bytes();
        bytes.len() >= 10
            && bytes[..4].iter().all(u8::is_ascii_digit)
            && bytes[4] == b'-'
            && bytes[5..7].iter().all(u8::is_ascii_digit)
            && bytes[7] == b'-'
            && bytes[8..10].iter().all(u8::is_ascii_digit)
    })
}

pub(crate) fn has_table(text: &str) -> bool {
    text.lines()
        .any(|line| line.len() >= 2 && line.starts_with('|') && line.ends_with('|'))
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
        if has_table(&text) && !has_verified_line(&text) {
            violations.push(Violation::Finding(Finding::on_file(
                RuleId::EveryTableIsDated,
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

    const DATED: &str = "Legend: ✅ yes.\n\nVerified: 2026-08-24 — Subject 1.0.\n\n| [Case](#case) | Subject |\n| --- | --- |\n| Runs | ✅ yes |\n";

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
    fn accepts_a_dated_comparison_and_the_template_placeholder() {
        assert!(run_on(DATED).is_empty());
        assert!(run_on("Verified: `<YYYY-MM-DD>` — Subject.\n\n| a |\n").is_empty());
    }

    #[test]
    fn rejects_tables_with_no_date() {
        let undated: String = DATED
            .lines()
            .filter(|l| !l.starts_with("Verified:"))
            .collect::<Vec<_>>()
            .join("\n");
        let out = run_on(&undated);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("comparison-docs:every-table-is-dated"));
    }

    #[test]
    fn a_tableless_document_needs_no_date() {
        assert!(run_on("# Notes\n\nProse only.\n").is_empty());
    }
}
