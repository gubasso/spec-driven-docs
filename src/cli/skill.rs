//! `skill` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use clap::{Subcommand, ValueEnum};

/// Read the embedded skills, or install them for coding agents.
#[derive(Debug, clap::Args)]
pub struct SkillArgs {
    /// The verb to run.
    #[command(subcommand)]
    pub command: SkillCommand,
}

/// Every verb `sdd skill` offers.
#[derive(Debug, Subcommand)]
pub enum SkillCommand {
    /// List every embedded skill.
    List,
    /// Print one skill's `SKILL.md`.
    Show(ShowArgs),
    /// Install the skills into the agent skill directories.
    Install(InstallArgs),
}

/// Print one skill's `SKILL.md`.
#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    /// The skill's name.
    pub name: String,
}

/// Install the skills into the agent skill directories.
#[derive(Debug, clap::Args)]
pub struct InstallArgs {
    /// Which agent's skill directory to install into.
    #[arg(long, value_enum, default_value_t = Agent::All)]
    pub agent: Agent,

    /// Where the skills land; `user` is the home-directory scope.
    #[arg(long, value_enum, default_value_t = Scope::User)]
    pub scope: Scope,

    /// Write the files; without it the install previews.
    #[arg(long)]
    pub apply: bool,

    /// Overwrite a destination whose bytes differ from the payload.
    #[arg(long, requires = "apply")]
    pub force: bool,
}

/// Which skill directory family to install into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum Agent {
    /// `.claude/skills` — read by Claude Code.
    Claude,
    /// `.agents/skills` — read by Codex, Gemini CLI, and Copilot.
    Codex,
    /// Both directories.
    All,
}

/// Where an install lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum Scope {
    /// The home-directory skill roots, shared across projects.
    User,
}
