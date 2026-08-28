//! `skill` subcommand: runtime-shape.
//!
//! Lists, prints, installs, and removes the embedded skills. Install and
//! uninstall semantics live in `services::skill_installer`; this handler
//! only resolves the home directory, the chosen roots, and the user-scope
//! record that sits beside them.

use camino::{Utf8Path, Utf8PathBuf};

use crate::cli::skill::{Agent, SkillArgs, SkillCommand};
use crate::context::AppContext;
use crate::domain::skill_record::RECORD_PATH;
use crate::error::AppError;
use crate::output;
use crate::services::skill_installer;

fn home() -> Result<Utf8PathBuf, AppError> {
    std::env::var("HOME")
        .ok()
        .filter(|home| !home.is_empty())
        .map(Utf8PathBuf::from)
        .ok_or_else(|| AppError::Usage("HOME is not set".to_string()))
}

fn roots(home: &Utf8Path, agent: Agent) -> Vec<Utf8PathBuf> {
    let mut roots = Vec::new();
    if matches!(agent, Agent::Codex | Agent::All) {
        roots.push(home.join(".agents/skills"));
    }
    if matches!(agent, Agent::Claude | Agent::All) {
        roots.push(home.join(".claude/skills"));
    }
    roots
}

/// List, print, install, or uninstall the embedded skills.
///
/// # Errors
///
/// [`AppError::Usage`] for an unknown skill or an unset home, and the
/// installer's refusals and I/O errors.
pub fn run(_ctx: &AppContext, args: SkillArgs) -> Result<(), AppError> {
    match args.command {
        SkillCommand::List => {
            for name in crate::embedded::skill_names() {
                output::line(name);
            }
            Ok(())
        }
        SkillCommand::Show(show) => {
            let Some(text) = crate::embedded::skill(&show.name) else {
                return Err(AppError::Usage(format!("no such skill: {}", show.name)));
            };
            output::line(text.trim_end_matches('\n'));
            Ok(())
        }
        SkillCommand::Install(install) => {
            let home = home()?;
            let roots = roots(&home, install.agent);
            let record = home.join(RECORD_PATH);
            let lines = skill_installer::install(&roots, &record, install.apply, install.force)?;
            for line in lines {
                output::line(line);
            }
            Ok(())
        }
        SkillCommand::Uninstall(uninstall) => {
            let home = home()?;
            let roots = roots(&home, uninstall.agent);
            let record = home.join(RECORD_PATH);
            let lines = skill_installer::uninstall(&roots, &record, uninstall.apply)?;
            for line in lines {
                output::line(line);
            }
            Ok(())
        }
    }
}
