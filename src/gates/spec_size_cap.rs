//! Gate: a spec stays within 300 authored lines and carries a TOC above 100.
//!
//! The generated TOC is excluded from the count: it grows with the
//! requirement list and would otherwise spend the author's budget on
//! navigation. Excluding it means trusting its delimiters, so the pair is
//! checked first — a spec carrying one marker and nothing to close it would
//! have every line after that marker deleted from the count, which is the
//! over-budget file the cap exists to reject.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::spec_rule_id_unique::spec_files;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::SpecStaysWithinLineCap];

const RULE: RuleId = RuleId::SpecStaysWithinLineCap;
const MARKER: &str = "<!--TOC-->";

/// Judge every spec under the documentation root.
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
    let mut violations = Vec::new();
    for file in files {
        let text = read_text(ctx, &file)?;
        let markers = text.lines().filter(|line| *line == MARKER).count();
        if markers != 0 && markers != 2 {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                &file,
                format!("{markers} TOC markers, expected 0 or 2"),
            )));
            continue;
        }
        let mut inside_toc = false;
        let authored = text
            .lines()
            .filter(|line| {
                if *line == MARKER {
                    inside_toc = !inside_toc;
                    return false;
                }
                !inside_toc
            })
            .count();
        if authored > 300 {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                &file,
                format!("{authored} authored lines, cap is 300"),
            )));
        } else if authored > 100 && markers == 0 {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                &file,
                "over 100 lines with no TOC",
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
        let specs = dir.path().join("_docs/specs");
        std::fs::create_dir_all(&specs).unwrap();
        std::fs::write(specs.join("SPEC-sample.md"), text).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_a_short_spec_with_no_toc() {
        assert!(run_on(&"line\n".repeat(100)).is_empty());
    }

    #[test]
    fn rejects_a_spec_over_the_authored_cap() {
        let out = run_on(&"line\n".repeat(301));
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("docs-specs:spec-stays-within-300-lines"));
        assert!(out[0].ends_with(": 301 authored lines, cap is 300"));
    }

    #[test]
    fn the_toc_region_does_not_spend_the_budget() {
        let text = format!(
            "<!--TOC-->\n{}<!--TOC-->\n{}",
            "toc\n".repeat(250),
            "line\n".repeat(90)
        );
        assert!(run_on(&text).is_empty());
    }

    #[test]
    fn a_long_spec_without_a_toc_is_rejected() {
        let out = run_on(&"line\n".repeat(101));
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": over 100 lines with no TOC"));
    }

    #[test]
    fn a_lone_marker_is_rejected_before_counting() {
        let text = format!("<!--TOC-->\n{}", "line\n".repeat(400));
        let out = run_on(&text);
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": 1 TOC markers, expected 0 or 2"));
    }
}
