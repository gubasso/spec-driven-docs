//! What the budget gates measure, read without judging.
//!
//! The gates decide pass or fail. This takes the same measurements and
//! hands them to whoever needs the number: `sdd debt` to record a ceiling,
//! and the planner to report a finding about a corpus written before the
//! convention arrived. One measurement, two readers, no second definition
//! of what a budget is.

use camino::Utf8Path;

use crate::domain::debt::Measurement;
use crate::domain::gate_id::GateId;
use crate::error::AppError;
use crate::gates::GateCtx;

use crate::domain::debt::BUDGET_GATES;

/// What one budget gate measures at a root.
///
/// # Errors
///
/// Whatever the gate refuses while reading the tree.
pub fn measure_gate(root: &Utf8Path, id: GateId) -> Result<Vec<Measurement>, AppError> {
    let ctx: GateCtx = crate::commands::gate::context_at(root, id)?;
    let measured = match id {
        GateId::AdrWordCap => crate::gates::adr_word_cap::measure(&ctx),
        GateId::AgentsDigestSize => crate::gates::agents_digest_size::measure(&ctx),
        GateId::ChapterSizeCap => crate::gates::chapter_size_cap::measure(&ctx),
        GateId::SpecSizeCap => crate::gates::spec_size_cap::measure(&ctx),
        _ => Ok(Vec::new()),
    };
    Ok(measured?)
}

/// Every measurement the budget gates take at a root.
///
/// # Errors
///
/// Whatever a gate refuses while reading the tree.
pub fn measure_all(root: &Utf8Path) -> Result<Vec<Measurement>, AppError> {
    let mut all = Vec::new();
    for id in BUDGET_GATES {
        all.extend(measure_gate(root, *id)?);
    }
    Ok(all)
}
