//! `migrate` subcommand: runtime-shape.
//!
//! Resolves the target, asks the migration service for one verb, and
//! renders text or JSON. A refusal is the service's, with its reason.

use camino::Utf8PathBuf;

use crate::cli::migrate::{MigrateArgs, MigrateCommand};
use crate::context::AppContext;
use crate::domain::migration::Marking;
use crate::error::AppError;
use crate::output;
use crate::services::migration::{ArchiveReport, archive, close, inventory, mark, rebuild, status};

/// Run one `sdd migrate` verb.
///
/// # Errors
///
/// [`AppError::Usage`] for a target the arguments cannot mean, and whatever
/// the verb refuses.
pub fn run(ctx: &AppContext, args: MigrateArgs) -> Result<(), AppError> {
    match args.command {
        MigrateCommand::Inventory(read) => {
            let report = inventory(&resolved(ctx, read.target)?)?;
            if read.json {
                return output::json(&report);
            }
            for entry in &report.documents {
                let kind = word(&entry.kind);
                let placement = word(&entry.placement);
                let home = word(&entry.home);
                output::line(format!("{kind} {placement} {home} {}", entry.path));
            }
            Ok(())
        }
        MigrateCommand::Archive(archiving) => {
            let target = resolved(ctx, archiving.target)?;
            let report = if archiving.rebuild {
                rebuild(&target, archiving.apply)?
            } else {
                archive(&target, archiving.trunk.as_deref(), archiving.apply)?
            };
            if archiving.json {
                return output::json(&report);
            }
            archived(&report, archiving.rebuild);
            Ok(())
        }
        MigrateCommand::Mark(marking) => {
            let target = resolved(ctx, marking.target)?;
            let report = mark(
                &target,
                &marking.name,
                marking.status,
                &Marking {
                    group: marking.group,
                    tags: marking.tag,
                    destinations: marking.destination,
                    reason: marking.reason,
                },
            )?;
            if marking.json {
                return output::json(&report);
            }
            for id in &report.marked {
                output::line(format!("{id}: {}", report.status.as_str()));
            }
            Ok(())
        }
        MigrateCommand::Status(read) => {
            let report = status(&resolved(ctx, read.target)?)?;
            if read.json {
                return output::json(&report);
            }
            output::line(format!("migration {} at {}", report.id, report.workspace));
            output::line(format!("base commit {}", report.base_commit));
            for source in &report.sources {
                let group = source.group.as_deref().unwrap_or("-");
                let changed = if source.changed { " (changed)" } else { "" };
                output::line(format!(
                    "{} {group} {}{changed}",
                    source.status.as_str(),
                    source.path
                ));
            }
            if report.unfinished.is_empty() {
                output::line(
                    "every source is verified or dropped: 'sdd migrate close --apply' ends it",
                );
            }
            Ok(())
        }
        MigrateCommand::Close(closing) => {
            let report = close(&resolved(ctx, closing.target)?, closing.apply)?;
            if closing.json {
                return output::json(&report);
            }
            if report.applied {
                output::line(format!("closed {}", report.id));
            } else {
                output::line(format!(
                    "{} can close: 'sdd migrate close --apply' removes the declaration key",
                    report.id
                ));
            }
            output::line(format!(
                "the archive stays at {}; removing it is the operator's",
                report.workspace
            ));
            Ok(())
        }
    }
}

/// The text form of an archive or a rebuild.
fn archived(report: &ArchiveReport, rebuilt: bool) {
    let verb = match (rebuilt, report.applied) {
        (false, true) => "archived",
        (false, false) => "would archive",
        (true, true) => "recreated",
        (true, false) => "would recreate",
    };
    for path in &report.archived {
        output::line(format!("{verb} {path}"));
    }
    for path in &report.in_place {
        output::line(format!("in place {path}"));
    }
    output::line(format!(
        "migration {} at {} from {}",
        report.id, report.workspace, report.base_commit
    ));
    if !report.applied {
        output::line("DRY RUN: no files written");
    }
}

/// The kebab-case word a serialized enum spells.
fn word(value: &impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(String::from))
        .unwrap_or_default()
}

/// The target the arguments name: absolute, or the working directory.
fn resolved(ctx: &AppContext, target: Utf8PathBuf) -> Result<Utf8PathBuf, AppError> {
    let target = if target.is_absolute() {
        target
    } else if target == "." {
        ctx.cwd.clone()
    } else {
        return Err(AppError::Usage("target must be absolute or .".to_string()));
    };
    if !target.is_dir() {
        return Err(AppError::Usage(format!("unresolved target: {target}")));
    }
    Ok(target)
}
