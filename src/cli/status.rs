//! `status` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

use camino::Utf8PathBuf;

/// Report an instance's state without gating on it.
#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    /// The repository to inspect; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}
