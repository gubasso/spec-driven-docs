//! `ki` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use camino::Utf8PathBuf;
use clap::Subcommand;

/// Read the known-issue zone.
#[derive(Debug, clap::Args)]
pub struct KiArgs {
    /// The verb to run.
    #[command(subcommand)]
    pub command: KiCommand,
}

/// Every verb `sdd ki` offers.
#[derive(Debug, Subcommand)]
pub enum KiCommand {
    /// List every known-issue record and its two axes.
    List(ListArgs),
}

/// List every known-issue record and its two axes.
#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// The repository to read; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Print one JSON array instead of text.
    #[arg(long)]
    pub json: bool,
}
