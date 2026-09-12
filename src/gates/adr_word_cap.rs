//! Gate: a decision record's body stays within 350 words, or within the
//! ceiling its project recorded for it.
//!
//! Records are permanent, so their cost is paid on every read; the cap keeps
//! each one a decision rather than a chapter. The record set is read from
//! the documentation root — filename shape and heading structure belong to
//! other gates.

use camino::Utf8PathBuf;

use crate::domain::debt::Measurement;
use crate::domain::finding::Finding;
use crate::domain::gate_id::GateId;
use crate::domain::rule_id::RuleId;
use crate::gates::budget;
use crate::gates::paths::docs_root;
use crate::gates::{GateCtx, GateError, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::BodyStaysWithinWordCap];

const RULE: RuleId = RuleId::BodyStaysWithinWordCap;
const CAP: usize = 350;

/// Every decision record under the documentation root, unfiltered, or none
/// where the layout holds no record.
fn records(ctx: &GateCtx) -> Vec<Utf8PathBuf> {
    let decisions = docs_root(ctx).join("decisions");
    let mut names: Vec<String> = ctx
        .path(&decisions)
        .read_dir_utf8()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string())
                .filter(|name| {
                    name.strip_prefix("ADR-")
                        .and_then(|rest| rest.strip_suffix(".md"))
                        .is_some_and(|slug| !slug.is_empty())
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names.into_iter().map(|name| decisions.join(name)).collect()
}

/// Measure every judged record: one `words` count per record.
///
/// # Errors
///
/// [`GateError::Io`] when a matched record cannot be read.
pub fn measure(ctx: &GateCtx) -> Result<Vec<Measurement>, GateError> {
    let mut measurements = Vec::new();
    // The records are this gate's subjects, so a reserved one leaves the
    // list before it is read.
    for path in ctx.retained(records(ctx)) {
        let words = read_text(ctx, &path)?.split_whitespace().count();
        measurements.push(Measurement::count(
            GateId::AdrWordCap,
            path.as_str(),
            "words",
            words,
            CAP,
        ));
    }
    Ok(measurements)
}

/// Judge every decision record under the documentation root.
///
/// # Errors
///
/// [`GateError::Io`] when a matched record cannot be read, and
/// [`GateError::Debt`] when the debt file cannot be trusted.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    // The layout check reads the unfiltered set: a project that reserves
    // every record still has a layout.
    if records(ctx).is_empty() {
        return Ok(vec![Violation::Layout(
            "no decision records matched".to_string(),
        )]);
    }
    let debt = budget::read_debt(ctx)?;
    let measurements = measure(ctx)?;
    Ok(budget::judge(
        &debt,
        GateId::AdrWordCap,
        RULE,
        &measurements,
        |m| {
            let words = match m.value {
                crate::domain::debt::Measured::Count { value, .. } => value,
                crate::domain::debt::Measured::Flag(_) => 0,
            };
            Violation::Finding(Finding::on_file(RULE, m.path.as_str(), words.to_string()))
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(words: usize) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let decisions = dir.path().join("_docs/decisions");
        std::fs::create_dir_all(&decisions).unwrap();
        std::fs::write(decisions.join("ADR-choice.md"), "word ".repeat(words)).unwrap();
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

    #[test]
    fn accepts_a_record_at_the_cap() {
        assert!(run_in(&fixture(350)).is_empty());
    }

    #[test]
    fn rejects_a_record_over_the_cap() {
        let out = run_in(&fixture(351));
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("decision-records:body-stays-within-350-words"));
        assert!(out[0].ends_with(": 351"));
    }

    #[test]
    fn an_empty_record_set_is_a_layout_failure() {
        let dir = tempfile::tempdir().unwrap();
        let out = run_in(&dir);
        assert_eq!(out, vec!["FAIL no decision records matched".to_string()]);
    }

    #[test]
    fn a_recorded_ceiling_carries_an_oversize_record() {
        let dir = fixture(612);
        std::fs::create_dir_all(dir.path().join(".spec-driven-docs")).unwrap();
        std::fs::write(
            dir.path().join(".spec-driven-docs/debt.yaml"),
            "schema_version: 1\nadr-word-cap:\n  _docs/decisions/ADR-choice.md:\n    words:\n      ceiling: 612\n",
        )
        .unwrap();
        assert!(run_in(&dir).is_empty());
        std::fs::write(
            dir.path().join("_docs/decisions/ADR-choice.md"),
            "word ".repeat(613),
        )
        .unwrap();
        let out = run_in(&dir);
        assert_eq!(out.len(), 1);
        assert!(
            out[0].ends_with(": 613 words, recorded ceiling is 612"),
            "{}",
            out[0]
        );
    }
}
