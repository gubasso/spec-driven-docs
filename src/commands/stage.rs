//! `stage` subcommand: runtime-shape.
//!
//! Resolves the target and the stage root, asks the stage service to render
//! the candidate, and reports where it landed. Cleanup is its own verb, so
//! removing a stage is always something somebody asked for.

use camino::Utf8PathBuf;

use crate::cli::stage::{StageArgs, StageCleanArgs, StageCommand, StageCompareArgs};
use crate::context::AppContext;
use crate::domain::paths::UserEnv;
use crate::error::AppError;
use crate::output;
use crate::stage::{self, Request};

/// Render a stage, or remove one.
///
/// # Errors
///
/// [`AppError::Usage`] for a target or stage path the arguments cannot
/// mean, [`AppError::Refused`] where a stage is already there or a cleanup
/// target is not a stage, and I/O errors writing the stage.
pub fn run(ctx: &AppContext, args: StageArgs) -> Result<(), AppError> {
    match args.command {
        Some(StageCommand::Clean(clean)) => return remove(&clean),
        Some(StageCommand::Compare(compare)) => return compared(ctx, compare),
        None => {}
    }
    let target = resolved(ctx, args.target)?;
    if args.acquisition.is_some() {
        crate::commands::self_depend::refuse_host_where_pinned(&target)?;
    }
    let state_root = UserEnv::from_process()
        .state_root()
        .ok_or_else(|| AppError::Usage("no state root resolves".to_string()))?
        .path;
    let docs_scratch = args
        .docs_scratch
        .as_deref()
        .map(crate::domain::manifest::parse_docs_scratch)
        .transpose()
        .map_err(|error| AppError::Usage(format!("--docs-scratch: {error}")))?;
    let writing_style = args
        .writing_style
        .as_deref()
        .map(crate::domain::instance_config::WritingStyle::parse_flag)
        .transpose()
        .map_err(|error| AppError::Usage(format!("--writing-style: {error}")))?;
    let receipt = stage::create(
        &Request {
            target,
            profile: args.profile,
            output: args.output,
            docs_scratch,
            reserve: args.reserve,
            writing_style,
            acquisition: args.acquisition.map(|choice| match choice {
                crate::cli::stage::AcquisitionChoice::Host => {
                    crate::domain::instance_config::Acquisition::host_at_this_release()
                }
            }),
        },
        &state_root,
    )?;

    if args.json {
        return output::json(&receipt);
    }
    output::line(format!(
        "staged spec-driven-docs {} ({}) for {}",
        receipt.version, receipt.profile, receipt.target
    ));
    output::line(format!("stage: {}", receipt.root));
    output::line(format!(
        "{} artifacts under {}/, reference material under {}/",
        receipt.artifacts.len(),
        stage::ARTIFACTS_DIR,
        stage::REFERENCE_DIR
    ));
    for note in &receipt.notes {
        output::line(note.clone());
    }
    output::line(format!(
        "the stage stays until 'sdd stage clean {}' removes it",
        receipt.root
    ));
    Ok(())
}

/// Compare one stage against its target.
fn compared(ctx: &AppContext, args: StageCompareArgs) -> Result<(), AppError> {
    use crate::stage::compare::{Provenance, State};

    let target = resolved(ctx, args.target)?;
    let report = crate::stage::compare::compare(&args.root, &target)?;
    if args.json {
        return output::json(&report);
    }
    let word = |state: State| match state {
        State::Equal => "equal",
        State::Absent => "absent",
        State::Differs => "differs",
        State::Retired => "retired",
    };
    let mut equal = 0usize;
    for row in &report.rows {
        if row.state == State::Equal {
            equal += 1;
            continue;
        }
        let provenance = row.provenance.map_or("", |held| match held {
            Provenance::Attributed => " (attributed)",
            Provenance::Reduced => " (reduced)",
            Provenance::Unknown => " (unknown)",
        });
        output::line(format!(
            "{} {} {}{provenance}",
            word(row.state),
            row.ownership.as_str(),
            row.path
        ));
    }
    output::line(format!(
        "{equal} of {} destinations equal the stage",
        report.rows.len()
    ));
    if let Some(reason) = &report.history.reason {
        output::line(format!("history: not read, because {reason}"));
    }
    Ok(())
}

/// Remove one stage this tool wrote.
fn remove(args: &StageCleanArgs) -> Result<(), AppError> {
    let lines = stage::clean(&args.path)?;
    if args.json {
        return output::json(&serde_json::json!({
            "schema": "sdd.stage-clean/1",
            "removed": args.path,
        }));
    }
    for line in lines {
        output::line(line);
    }
    Ok(())
}

/// The target, absolute or the working directory.
fn resolved(ctx: &AppContext, target: Utf8PathBuf) -> Result<Utf8PathBuf, AppError> {
    if target.is_absolute() {
        Ok(target)
    } else if target == "." {
        Ok(ctx.cwd.clone())
    } else {
        Err(AppError::Usage("target must be absolute or .".to_string()))
    }
}
