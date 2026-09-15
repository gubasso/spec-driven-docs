//! `self-depend` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use camino::Utf8PathBuf;
use clap::{Subcommand, ValueEnum};

use crate::self_depend::manager::Manager;
use crate::self_depend::venue::Venue;

/// Wire this tool as a consumer's dependency and keep its pin fresh.
#[derive(Debug, clap::Args)]
pub struct SelfDependArgs {
    /// The verb to run.
    #[command(subcommand)]
    pub command: SelfDependCommand,
}

/// Every verb `sdd self-depend` offers.
#[derive(Debug, Subcommand)]
pub enum SelfDependCommand {
    /// Report what a target carries, offline: each manager's pin, the shell-entry line, and any leftover.
    Status(StatusArgs),
    /// Serve the fragments for one manager and venue pair and the shell-entry line; seed the manager file where the target has none.
    Add(AddArgs),
    /// Move the pin to the latest release through the wired manager; the flake pair moves both files or neither.
    Sync(SyncArgs),
    /// Remove what a predecessor bump mechanism left, and name what a line scan must not touch.
    Clean(CleanArgs),
}

/// Report what a target carries.
#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    /// The project to read; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Report one manager's entry alone; every manager by default, absent ones included.
    #[arg(long, value_enum)]
    pub manager: Option<Manager>,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// Serve the fragments for one pair.
#[derive(Debug, clap::Args)]
pub struct AddArgs {
    /// The project to wire; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// The release tag to pin: v0.2.16, 0.2.16, or the release URL; this binary's version by default.
    #[arg(long)]
    pub tag: Option<String>,

    /// The target's tool manager; required when the target carries several, the flake when it carries none.
    #[arg(long, value_enum)]
    pub manager: Option<Manager>,

    /// Where this tool is fetched from; the first venue the manager renders a fragment for by default.
    #[arg(long, value_enum)]
    pub venue: Option<Venue>,

    /// Write the seed files; without it the fragments are printed and nothing is written.
    #[arg(long)]
    pub apply: bool,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// Who is calling the sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Caller {
    /// The shell-entry line on directory entry: gated by the stamp, silent, exit 0.
    Envrc,
    /// A person or an agent at a prompt: every outcome reported, the exit-code matrix.
    Operator,
}

/// Move the pin.
#[derive(Debug, clap::Args)]
pub struct SyncArgs {
    /// The project to sync; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// The release tag to pin, in either direction, making no network request; the latest release by default.
    #[arg(long)]
    pub tag: Option<String>,

    /// The manager whose pin moves; the one manager naming this tool by default.
    #[arg(long, value_enum)]
    pub manager: Option<Manager>,

    /// Who is calling: envrc stays silent and exits 0 on every outcome, operator reports and fails loudly.
    #[arg(long, value_enum, default_value_t = Caller::Envrc)]
    pub caller: Caller,

    /// Rewrite the pin and refresh the lock; without it the bump is reported and nothing runs.
    #[arg(long)]
    pub apply: bool,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// Remove what a predecessor left.
#[derive(Debug, clap::Args)]
pub struct CleanArgs {
    /// The project to clean; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// One extra file to remove, relative to the target, for a predecessor the catalog does not know; repeatable.
    #[arg(long)]
    pub also: Vec<Utf8PathBuf>,

    /// Remove the files; without it every leftover is listed.
    #[arg(long)]
    pub apply: bool,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}
