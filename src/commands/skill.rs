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
use crate::services::skill_installer::{self, Layout};

fn home() -> Result<Utf8PathBuf, AppError> {
    std::env::var("HOME")
        .ok()
        .filter(|home| !home.is_empty())
        .map(Utf8PathBuf::from)
        .ok_or_else(|| AppError::Usage("HOME is not set".to_string()))
}

/// The root holding what the skills share, relative to the home directory.
///
/// Home-relative rather than `XDG_STATE_HOME`-relative for the reason the
/// record states: the skills naming these artifacts live under
/// `$HOME/.agents` and `$HOME/.claude`, which no XDG variable moves, and a
/// shared file reachable under a different home than the skills reading it
/// would be worse than no shared file at all.
const SHARED_ROOT: &str = ".local/state/spec-driven-docs/skills/shared";

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
