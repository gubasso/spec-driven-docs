//! `skill` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use clap::{Subcommand, ValueEnum};

/// Read the embedded skills, and remove the copies an earlier release
/// installed under the home.
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
    /// Print one skill's `SKILL.md`, or one file of its package.
    Show(ShowArgs),
    /// Retired: the skills land into each instance through `sdd init` and
    /// `sdd upgrade`.
    Install(InstallArgs),
    /// Remove the copies an earlier release installed under the home.
    Uninstall(UninstallArgs),
}

/// Print one skill's `SKILL.md`, or one file of its package.
#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    /// The skill's name.
    pub name: String,

    /// One file of the package, by its path relative to the package root,
    /// such as `references/pre-flight-gate.md`.
    #[arg(long, value_name = "PATH")]
    pub file: Option<String>,
}

/// The retired install: it states where the skills land now.
#[derive(Debug, clap::Args)]
pub struct InstallArgs {
    /// Which agent's skill directory an earlier release installed into.
    #[arg(long, value_enum, default_value_t = Agent::All)]
    pub agent: Agent,

    /// Refuse, naming the landing; without it the same text prints.
    #[arg(long)]
    pub apply: bool,
}

/// Remove the installed skills from the agent skill directories.
#[derive(Debug, clap::Args)]
pub struct UninstallArgs {
    /// Which agent's skill directory to remove from.
    #[arg(long, value_enum, default_value_t = Agent::All)]
    pub agent: Agent,

    /// Remove the files; without it the uninstall previews.
    #[arg(long)]
    pub apply: bool,
}

/// Which skill directory family to install into.
///
/// The values name the root rather than one host that reads it. `sdd status
/// --json` reports where each one resolves, including the variable that
/// moved it, so no help text here has to spell a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum Agent {
    /// The root Claude Code reads.
    Claude,
    /// The shared root every other agent reads. `codex` is its old name.
    #[value(alias = "codex")]
    Agents,
    /// Both roots.
    All,
}
