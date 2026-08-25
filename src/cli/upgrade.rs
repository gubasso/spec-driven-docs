//! `upgrade` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

use camino::Utf8PathBuf;

/// Upgrade an installed instance to this binary's version.
#[derive(Debug, clap::Args)]
pub struct UpgradeArgs {
    /// The instance to upgrade; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Report the plan and change nothing.
    #[arg(long)]
    pub dry_run: bool,
}
