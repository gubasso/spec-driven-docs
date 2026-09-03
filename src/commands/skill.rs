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
use crate::services::skill_installer::{self, AGENTS_ROOT, CLAUDE_ROOT, Layout, SHARED_ROOT, home};

/// The roots one run touches, and the record that vouches for them.
fn layout(agent: Agent) -> Result<Layout, AppError> {
    let home = home()?;
    Ok(Layout {
        roots: roots(&home, agent),
        every_root: roots(&home, Agent::All),
        shared: home.join(SHARED_ROOT),
        record: home.join(RECORD_PATH),
    })
}

fn roots(home: &Utf8Path, agent: Agent) -> Vec<Utf8PathBuf> {
    let mut roots = Vec::new();
    if matches!(agent, Agent::Codex | Agent::All) {
        roots.push(home.join(AGENTS_ROOT));
    }
    if matches!(agent, Agent::Claude | Agent::All) {
        roots.push(home.join(CLAUDE_ROOT));
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
            let layout = layout(install.agent)?;
            let lines = skill_installer::install(&layout, install.apply, install.force)?;
            for line in lines {
                output::line(line);
            }
            Ok(())
        }
        SkillCommand::Uninstall(uninstall) => {
            let layout = layout(uninstall.agent)?;
            let lines = skill_installer::uninstall(&layout, uninstall.apply)?;
            for line in lines {
                output::line(line);
            }
            Ok(())
        }
    }
}
