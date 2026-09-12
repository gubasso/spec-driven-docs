//! `debt` subcommand: runtime-shape.
//!
//! Three verbs, one engine. Measurement, the tightening function, and the
//! serialization are the same code, and each verb differs only in where its
//! input comes from: `baseline` mints new exceptions from what the corpus
//! measures today, `migrate` converts the exemptions a previous operator
//! already accepted and nothing else, and `tighten` lowers what is recorded
//! to what is measured. One verb switching between the first two on the
//! state of the filesystem would let a request for a format conversion
//! accept violations instead, so they stay two. Every write is atomic, and
//! the legacy list leaves only after the new file lands.

use camino::{Utf8Path, Utf8PathBuf};

use crate::adapters::fs::{remove_within, write_within};
use crate::cli::debt::{DebtArgs, DebtVerb, DebtVerbArgs};
use crate::context::AppContext;
use crate::domain::debt::{
    BUDGET_GATES, Change, DEBT_PATH, Debt, DebtError, LEGACY_DEBT_PATH, Measurement, Presence,
    Recorded,
};
use crate::domain::gate_id::GateId;
use crate::error::AppError;
use crate::gates::GateCtx;
use crate::output;

const DRY_RUN: &str = "DRY RUN: no files written";

fn resolve_target(ctx: &AppContext, target: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    if target.is_absolute() {
        Ok(target.to_path_buf())
    } else if target == "." {
        Ok(ctx.cwd.clone())
    } else {
        Err(AppError::Usage("target must be absolute or .".to_string()))
    }
}

/// Measure one budget gate at a root, under the project's declaration.
fn measure_gate(root: &Utf8Path, id: GateId) -> Result<Vec<Measurement>, AppError> {
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

/// Every measurement the four budget gates take at a root.
pub(crate) fn measure_all(root: &Utf8Path) -> Result<Vec<Measurement>, AppError> {
    let mut all = Vec::new();
    for id in BUDGET_GATES {
        all.extend(measure_gate(root, *id)?);
    }
    Ok(all)
}

fn print_debt(debt: &Debt) {
    for line in debt.render().lines() {
        output::line(line);
    }
}

/// Write the debt file, or remove it where the debt is empty: absence is
/// the empty debt, and an empty file would be a second spelling of it.
fn write_debt(root: &Utf8Path, debt: &Debt) -> Result<(), AppError> {
    let path = root.join(DEBT_PATH);
    if debt.is_empty() {
        if path.is_file() {
            remove_within(root, Utf8Path::new(DEBT_PATH))?;
            output::line(format!("OK removed {DEBT_PATH}; nothing remains recorded"));
        } else {
            output::line("OK nothing to record; no debt file is needed");
        }
        return Ok(());
    }
    write_within(root, Utf8Path::new(DEBT_PATH), debt.render().as_bytes())?;
    output::line(format!("OK wrote {DEBT_PATH}"));
    Ok(())
}

fn baseline(root: &Utf8Path, apply: bool) -> Result<(), AppError> {
    let presence = Presence::at(root);
    if presence.dimensional {
        return Err(DebtError::AlreadyBaselined.into());
    }
    if presence.legacy {
        return Err(DebtError::LegacyBlocksBaseline.into());
    }
    let debt = Debt::baseline(&measure_all(root)?);
    if debt.is_empty() {
        output::line("OK no budget violation to record; no debt file is needed");
        return Ok(());
    }
    print_debt(&debt);
    if !apply {
        output::line(DRY_RUN);
        return Ok(());
    }
    write_debt(root, &debt)
}

/// The legacy list's entries, read by the same parser the gate uses.
fn legacy_entries(root: &Utf8Path) -> Result<Vec<String>, AppError> {
    Ok(crate::domain::debt::legacy_list(&std::fs::read_to_string(
        root.join(LEGACY_DEBT_PATH),
    )?))
}

fn migrate(root: &Utf8Path, apply: bool) -> Result<(), AppError> {
    if !Presence::at(root).legacy {
        return Err(DebtError::NothingToMigrate.into());
    }
    let listed = legacy_entries(root)?;
    // The legacy list exempted chapters alone, so only the chapter gate's
    // measurements can become entries. A path the list names that the gate
    // does not measure, or that now fits, carried no live exemption and is
    // reported rather than converted.
    let measured = measure_gate(root, GateId::ChapterSizeCap)?;
    let mut debt = Debt::default();
    for path in &listed {
        match measured
            .iter()
            .find(|m| crate::domain::debt::normalize(&m.path) == *path)
        {
            Some(m) if m.violates() => {
                if let crate::domain::debt::Measured::Count { value, .. } = m.value {
                    debt.record(
                        GateId::ChapterSizeCap,
                        path,
                        "lines",
                        Recorded::Ceiling(value),
                    );
                    output::line(format!("{path}: ceiling fixed at {value} lines"));
                }
            }
            Some(_) => output::line(format!("{path}: now fits; dropped")),
            None => output::line(format!("{path}: the gate measures no such path; dropped")),
        }
    }
    if !apply {
        print_debt(&debt);
        output::line(DRY_RUN);
        return Ok(());
    }
    // The new file lands first, and the old list leaves only after it
    // has. An interruption between the two leaves both present, which is a
    // failure naming this verb, never a state where neither holds.
    write_debt(root, &debt)?;
    remove_within(root, Utf8Path::new(LEGACY_DEBT_PATH))?;
    output::line(format!("OK removed {LEGACY_DEBT_PATH}"));
    Ok(())
}

fn describe(change: &Change) -> String {
    match change {
        Change::Lowered {
            gate,
            path,
            dimension,
            from,
            to,
        } => format!("{gate}: {path}: {dimension}: ceiling {from} -> {to}"),
        Change::Removed {
            gate,
            path,
            dimension,
            reason,
        } => format!("{gate}: {path}: {dimension}: removed, {reason}"),
        Change::Grew {
            gate,
            path,
            dimension,
            ceiling,
            measured,
        } => format!(
            "{gate}: {path}: {dimension}: measured {measured} above the ceiling of {ceiling}; unchanged, the gate fails on it"
        ),
    }
}

fn tighten(root: &Utf8Path, apply: bool) -> Result<(), AppError> {
    let debt = Debt::read(root)?;
    if debt.is_empty() {
        output::line("OK nothing recorded; nothing to tighten");
        return Ok(());
    }
    let tightened = debt.tighten(&measure_all(root)?);
    for change in &tightened.changes {
        output::line(describe(change));
    }
    let moved = tightened
        .changes
        .iter()
        .any(|change| !matches!(change, Change::Grew { .. }));
    if !moved {
        output::line("OK every ceiling is at its measurement; nothing to tighten");
        return Ok(());
    }
    if !apply {
        print_debt(&tightened.debt);
        output::line(DRY_RUN);
        return Ok(());
    }
    write_debt(root, &tightened.debt)
}

/// Baseline, migrate, or tighten.
///
/// # Errors
///
/// [`AppError::Debt`] for a state the verb refuses or a file it cannot
/// trust, [`AppError::Usage`] for a target the arguments cannot mean, and
/// I/O errors when the tree cannot be read or written.
pub fn run(ctx: &AppContext, args: DebtArgs) -> Result<(), AppError> {
    let (verb, DebtVerbArgs { target, apply }) = match args.verb {
        DebtVerb::Baseline(args) => (baseline as fn(&Utf8Path, bool) -> _, args),
        DebtVerb::Migrate(args) => (migrate as fn(&Utf8Path, bool) -> _, args),
        DebtVerb::Tighten(args) => (tighten as fn(&Utf8Path, bool) -> _, args),
    };
    let root = resolve_target(ctx, &target)?;
    verb(&root, apply)
}
