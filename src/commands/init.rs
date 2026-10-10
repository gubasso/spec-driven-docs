//! `init` subcommand: runtime-shape.
//!
//! A front over the engine. It classifies the target first and serves one
//! classification: a first landing. A settled corpus or an installed
//! instance is somebody else's verb, and this one says which rather than
//! landing seeds beside a convention that is already there.
//!
//! Projects the flags into the installer's options and prints its report.
//! Install semantics live in `services::installer`.

use crate::cli::init::InitArgs;
use crate::context::AppContext;
use crate::domain::manifest::parse_docs_scratch;
use crate::error::AppError;
use crate::landing::classify::Intent;
use crate::output;
use crate::services::installer::{InitOptions, init};

/// Install the payload into a target repository.
///
/// # Errors
///
/// Whatever the installer refuses; see [`init`].
pub fn run(ctx: &AppContext, args: InitArgs) -> Result<(), AppError> {
    let incremental = args.incremental;
    let options = match args.stage.as_deref() {
        Some(root) => staged(ctx, root, &args)?,
        None => flagged(args)?,
    };
    let (intent, inherited) = if incremental {
        (Intent::Incremental, Some(inherited(&options)?))
    } else {
        (Intent::Init, None)
    };
    let outcome = init(&options, intent)?;
    for line in &outcome.lines {
        output::line(line);
    }
    if let Some(debt) = inherited {
        crate::commands::debt::record_inherited(&options.target, &debt, outcome.applied)?;
    }
    Ok(())
}

/// The debt an incremental landing inherits, measured before it writes.
///
/// The target is judged first, so a target the landing refuses is never
/// measured. The measurement reads the landing's own candidate: the
/// declaration it will write and the documentation root its record will
/// name, so what is measured is what lands.
fn inherited(options: &InitOptions) -> Result<crate::domain::debt::Debt, AppError> {
    crate::commands::front::serves(Intent::Incremental, &options.target)?;
    crate::commands::debt::refuse_recorded(&options.target)?;
    let target = crate::services::installer::resolved_target(&options.target)?;
    let candidate = crate::services::installer::candidate_for(&target, options)?;
    crate::commands::debt::inherited(
        &target,
        &candidate.declaration,
        candidate.manifest.docs_root.as_str(),
    )
}

/// The options the flags spell.
fn flagged(args: InitArgs) -> Result<InitOptions, AppError> {
    let (Some(target), Some(profile)) = (args.target, args.profile) else {
        return Err(AppError::Usage(
            "--target and --profile are required without --stage".to_string(),
        ));
    };
    let docs_scratch = args
        .docs_scratch
        .as_deref()
        .map(parse_docs_scratch)
        .transpose()
        .map_err(|error| AppError::Usage(format!("--docs-scratch: {error}")))?;
    let writing_style = args
        .writing_style
        .as_deref()
        .map(crate::domain::instance_config::WritingStyle::parse_flag)
        .transpose()
        .map_err(|error| AppError::Usage(format!("--writing-style: {error}")))?;
    Ok(InitOptions {
        target,
        profile,
        apply: args.apply,
        dry_run: args.dry_run,
        docs_scratch,
        reserve: args.reserve,
        writing_style,
        acquisition: None,
        replace: Vec::new(),
    })
}

/// The options a stage receipt recorded.
///
/// SATISFIES staging:a-landing-takes-the-choices-its-stage-recorded
fn staged(
    ctx: &AppContext,
    root: &camino::Utf8Path,
    args: &InitArgs,
) -> Result<InitOptions, AppError> {
    let target = match args.target.as_ref() {
        Some(target) if target.is_absolute() => target.clone(),
        Some(target) if target == "." => ctx.cwd.clone(),
        Some(_) => return Err(AppError::Usage("target must be absolute or .".to_string())),
        None => ctx.cwd.clone(),
    };
    let choices = crate::stage::read_choices(root, &target)?;
    // A manager can come to pin this tool between the stage and the landing.
    if choices.acquisition.is_some() {
        crate::commands::self_depend::refuse_host_where_pinned(&choices.target)?;
    }
    Ok(InitOptions {
        target: choices.target,
        profile: choices.profile,
        apply: args.apply,
        dry_run: args.dry_run,
        docs_scratch: Some(choices.docs_scratch),
        reserve: choices.reserve,
        writing_style: Some(choices.writing_style),
        acquisition: choices.acquisition,
        replace: Vec::new(),
    })
}
