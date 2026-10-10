//! `env` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

use camino::Utf8PathBuf;

use crate::services::env::Item;

/// Report the shell environment the gates read, or write one item of it.
#[derive(Debug, clap::Args)]
pub struct EnvArgs {
    /// The repository to read; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Write this item through the loader the repository records, with its
    /// ignore entry; without it the report is printed and nothing is written.
    #[arg(long, value_enum, value_name = "ITEM")]
    pub apply: Option<Item>,

    /// The docs scratch location the variable names, for
    /// `--apply docs-scratch-variable`.
    #[arg(long, value_name = "PATH", requires = "apply")]
    pub value: Option<String>,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}
