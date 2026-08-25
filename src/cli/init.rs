//! `init` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

use camino::Utf8PathBuf;

use crate::domain::profile::ProfileId;

/// Install the payload into a target repository.
#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// The target repository; must be an absolute path.
    #[arg(long)]
    pub target: Utf8PathBuf,

    /// The profile to install.
    #[arg(long, value_enum)]
    pub profile: ProfileId,

    /// Write into a non-empty target that has no instance yet.
    #[arg(long, conflicts_with = "dry_run")]
    pub apply: bool,

    /// Preview only; write nothing.
    #[arg(long)]
    pub dry_run: bool,
}
