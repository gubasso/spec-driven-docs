//! `stage` subcommand: runtime-shape.
//!
//! Resolves the target and the stage root, asks the stage service to render
//! the candidate, and reports where it landed. Cleanup is its own verb, so
//! removing a stage is always something somebody asked for.

use camino::Utf8PathBuf;

use crate::cli::stage::{StageArgs, StageCleanArgs, StageCommand};
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
    if let Some(StageCommand::Clean(clean)) = args.command {
        return remove(&clean);
    }
    let target = resolved(ctx, args.target)?;
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
