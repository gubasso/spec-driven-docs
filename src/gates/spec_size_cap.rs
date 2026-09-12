//! Gate: a spec stays within 300 authored lines and carries a TOC above 100,
//! or within the ceiling and exception its project recorded for it.
//!
//! The generated TOC is excluded from the count: it grows with the
//! requirement list and would otherwise spend the author's budget on
//! navigation. Excluding it means trusting its delimiters, so the pair is
//! checked first — a spec carrying one marker and nothing to close it would
//! have every line after that marker deleted from the count, which is the
//! over-budget file the cap exists to reject. A malformed pair takes no
//! measurement and no debt entry: it stays strict.
//!
//! The two dimensions are measured independently. Reporting the missing
//! table of contents only once the line count fits would let a recorded
//! ceiling silence the one active finding and let the second appear later,
//! as a reward for shrinking the file.

use crate::domain::debt::Measurement;
use crate::domain::finding::Finding;
use crate::domain::gate_id::GateId;
use crate::domain::rule_id::RuleId;
use crate::gates::budget;
use crate::gates::spec_rule_id_unique::spec_files_judged;
use crate::gates::{GateCtx, GateError, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::SpecStaysWithinLineCap];

const RULE: RuleId = RuleId::SpecStaysWithinLineCap;
const MARKER: &str = "<!--TOC-->";
const CAP: usize = 300;
const TOC_FROM: usize = 100;

/// What one spec measured, or why it could not be measured.
struct Counted {
    path: String,
    markers: usize,
    authored: usize,
}

fn count(ctx: &GateCtx, file: &camino::Utf8Path) -> Result<Counted, GateError> {
    let text = read_text(ctx, file)?;
    let markers = text.lines().filter(|line| *line == MARKER).count();
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
    Ok(Counted {
        path: file.to_string(),
        markers,
        authored,
    })
}

fn measurements_of(counted: &Counted) -> [Measurement; 2] {
    [
        Measurement::count(
            GateId::SpecSizeCap,
            counted.path.as_str(),
            "authored_lines",
            counted.authored,
            CAP,
        ),
        Measurement::flag(
            GateId::SpecSizeCap,
            counted.path.as_str(),
            "missing_toc",
            counted.authored > TOC_FROM && counted.markers == 0,
        ),
    ]
}

/// Measure every spec under the documentation root on both dimensions. A
/// spec with a malformed marker pair is skipped, as the gate skips it.
///
/// # Errors
///
/// [`GateError::Io`] when a spec cannot be read.
pub fn measure(ctx: &GateCtx) -> Result<Vec<Measurement>, GateError> {
    let mut measurements = Vec::new();
    for file in spec_files_judged(ctx).unwrap_or_default() {
        let counted = count(ctx, &file)?;
        if counted.markers == 0 || counted.markers == 2 {
            measurements.extend(measurements_of(&counted));
        }
    }
    Ok(measurements)
}

/// Judge every spec under the documentation root.
///
/// # Errors
///
/// [`GateError::Io`] when a spec cannot be read, and [`GateError::Debt`]
/// when the debt file cannot be trusted.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    let Some(files) = spec_files_judged(ctx) else {
        return Ok(vec![Violation::Layout(
            "no specs matched; the layout moved".to_string(),
        )]);
    };
    let debt = budget::read_debt(ctx)?;
    let mut violations = Vec::new();
    let mut measurements = Vec::new();
    for file in files {
        let counted = count(ctx, &file)?;
        if counted.markers != 0 && counted.markers != 2 {
            violations.push(Violation::Finding(Finding::on_file(
                RULE,
                &file,
                format!("{} TOC markers, expected 0 or 2", counted.markers),
            )));
            continue;
        }
        measurements.extend(measurements_of(&counted));
    }
    violations.extend(budget::judge(
        &debt,
        GateId::SpecSizeCap,
        RULE,
        &measurements,
        |m| {
            let detail = match m.value {
                crate::domain::debt::Measured::Count { value, .. } => {
                    format!("{value} authored lines, cap is {CAP}")
                }
                crate::domain::debt::Measured::Flag(_) => {
                    format!("over {TOC_FROM} lines with no TOC")
                }
            };
            Violation::Finding(Finding::on_file(RULE, m.path.as_str(), detail))
        },
    ));
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(text: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let specs = dir.path().join("_docs/specs");
        std::fs::create_dir_all(&specs).unwrap();
        std::fs::write(specs.join("SPEC-sample.md"), text).unwrap();
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

    fn run_on(text: &str) -> Vec<String> {
        run_in(&fixture(text))
    }

    /// A spec of `lines` authored lines carrying an empty, well-formed TOC.
    fn with_toc(lines: usize) -> String {
        format!("<!--TOC-->\n<!--TOC-->\n{}", "line\n".repeat(lines))
    }

    #[test]
    fn accepts_a_short_spec_with_no_toc() {
        assert!(run_on(&"line\n".repeat(100)).is_empty());
    }

    #[test]
    fn rejects_a_spec_over_the_authored_cap() {
        let out = run_on(&with_toc(301));
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
    fn both_dimensions_are_reported_independently() {
        let out = run_on(&"line\n".repeat(301));
        assert_eq!(out.len(), 2, "{out:?}");
        assert!(out[0].ends_with(": 301 authored lines, cap is 300"));
        assert!(out[1].ends_with(": over 100 lines with no TOC"));
    }

    #[test]
    fn a_lone_marker_is_rejected_before_counting() {
        let text = format!("<!--TOC-->\n{}", "line\n".repeat(400));
        let out = run_on(&text);
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": 1 TOC markers, expected 0 or 2"));
    }

    #[test]
    fn a_malformed_marker_stays_strict_under_debt() {
        let dir = fixture(&format!("<!--TOC-->\n{}", "line\n".repeat(400)));
        std::fs::create_dir_all(dir.path().join(".spec-driven-docs")).unwrap();
        std::fs::write(
            dir.path().join(".spec-driven-docs/debt.yaml"),
            "schema_version: 1\nspec-size-cap:\n  _docs/specs/SPEC-sample.md:\n    authored_lines:\n      ceiling: 400\n    missing_toc: true\n",
        )
        .unwrap();
        let out = run_in(&dir);
        assert!(
            out.iter().any(|line| line.contains("1 TOC markers")),
            "{out:?}"
        );
    }

    #[test]
    fn a_recorded_ceiling_and_exception_carry_an_oversize_spec_without_a_toc() {
        let dir = fixture(&"line\n".repeat(417));
        std::fs::create_dir_all(dir.path().join(".spec-driven-docs")).unwrap();
        std::fs::write(
            dir.path().join(".spec-driven-docs/debt.yaml"),
            "schema_version: 1\nspec-size-cap:\n  _docs/specs/SPEC-sample.md:\n    authored_lines:\n      ceiling: 417\n    missing_toc: true\n",
        )
        .unwrap();
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn measure_reports_both_dimensions_of_an_oversize_spec_without_a_toc() {
        let dir = fixture(&"line\n".repeat(417));
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        let measured = measure(&ctx).unwrap();
        assert_eq!(measured.len(), 2);
        assert!(measured.iter().all(Measurement::violates));
    }
}
