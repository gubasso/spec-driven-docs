//! `skill` subcommand: runtime-shape.
//!
//! Lists, prints, and installs the embedded skills. Install semantics live
//! in `services::skill_installer`; this handler only resolves the home
//! directory and the chosen roots.

use camino::Utf8PathBuf;

use crate::cli::skill::{Agent, InstallArgs, SkillArgs, SkillCommand};
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::services::skill_installer;

fn roots(args: &InstallArgs) -> Result<Vec<Utf8PathBuf>, AppError> {
    let home = std::env::var("HOME")
        .ok()
        .filter(|home| !home.is_empty())
        .ok_or_else(|| AppError::Usage("HOME is not set".to_string()))?;
    let home = Utf8PathBuf::from(home);
    let mut roots = Vec::new();
    if matches!(args.agent, Agent::Codex | Agent::All) {
        roots.push(home.join(".agents/skills"));
    }
    if matches!(args.agent, Agent::Claude | Agent::All) {
        roots.push(home.join(".claude/skills"));
    }
    Ok(roots)
}

/// List, print, or install the embedded skills.
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
            let roots = roots(&install)?;
            let lines = skill_installer::install(&roots, install.apply, install.force)?;
            for line in lines {
                output::line(line);
            }
            Ok(())
        }
    }
}
