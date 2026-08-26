//! `skill` subcommand: runtime-shape.
//!
//! Lists, prints, installs, and removes the embedded skills. Install and
//! uninstall semantics live in `services::skill_installer`; this handler
//! only resolves the home directory and the chosen roots.

use camino::Utf8PathBuf;

use crate::cli::skill::{Agent, SkillArgs, SkillCommand};
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::skill_installer;

fn roots(agent: Agent) -> Result<Vec<Utf8PathBuf>, AppError> {
    let home = std::env::var("HOME")
        .ok()
        .filter(|home| !home.is_empty())
        .ok_or_else(|| AppError::Usage("HOME is not set".to_string()))?;
    let home = Utf8PathBuf::from(home);
    let mut roots = Vec::new();
    if matches!(agent, Agent::Codex | Agent::All) {
        roots.push(home.join(".agents/skills"));
    }
    if matches!(agent, Agent::Claude | Agent::All) {
        roots.push(home.join(".claude/skills"));
    }
    Ok(roots)
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
            let roots = roots(install.agent)?;
            let lines = skill_installer::install(&roots, install.apply, install.force)?;
            for line in lines {
                output::line(line);
            }
            Ok(())
        }
        SkillCommand::Uninstall(uninstall) => {
            let roots = roots(uninstall.agent)?;
            let lines = skill_installer::uninstall(&roots, uninstall.apply)?;
            for line in lines {
                output::line(line);
            }
            Ok(())
        }
    }
}
