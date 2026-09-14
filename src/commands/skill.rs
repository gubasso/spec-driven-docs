//! `skill` subcommand: runtime-shape.
//!
//! Lists, prints, installs, and removes the embedded skills. Install and
//! uninstall semantics live in `services::skill_installer`; this handler
//! only resolves the roots the run touches and the receipt that vouches
//! for them.

use camino::Utf8PathBuf;

use crate::cli::skill::{Agent, SkillArgs, SkillCommand};
use crate::context::AppContext;
use crate::domain::paths::{
    AgentId, LEGACY_SHARED_ROOT, LEGACY_SKILL_RECEIPT_PATH, SKILL_RECEIPT_FILE, UserEnv,
};
use crate::error::AppError;
use crate::output;
use crate::services::skill_installer::{self, Layout, home};

/// The roots one run touches, and the receipt that vouches for them.
fn layout(agent: Agent) -> Result<Layout, AppError> {
    let home = home()?;
    let env = UserEnv::from_process();
    let state_root = env
        .state_root()
        .ok_or_else(|| AppError::Usage("no state root resolves".to_string()))?
        .path;
    Ok(Layout {
        roots: roots(&env, agent),
        receipt: state_root.join(SKILL_RECEIPT_FILE),
        legacy_receipt: home.join(LEGACY_SKILL_RECEIPT_PATH),
        legacy_shared: home.join(LEGACY_SHARED_ROOT),
        state_root,
    })
}

/// The selected roots, resolved through the table and deduplicated there.
fn roots(env: &UserEnv, agent: Agent) -> Vec<Utf8PathBuf> {
    let selected: &[AgentId] = match agent {
        Agent::Claude => &[AgentId::Claude],
        Agent::Agents => &[AgentId::Agents],
        Agent::All => &[AgentId::Claude, AgentId::Agents],
    };
    env.agent_roots(selected)
        .into_iter()
        .map(|entry| entry.path)
        .collect()
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
