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

    /// Land the choices a stage recorded for this target: its docs scratch,
    /// writing style, reserved paths, and acquisition decision. Every byte
    /// is still rendered from this binary; only the stage's receipt is
    /// read, and a receipt another release or another target wrote refuses.
    #[arg(long, value_name = "ROOT")]
    pub stage: Option<Utf8PathBuf>,

    /// Overwrite this conflict, as the run lists it; repeatable. Its bytes
    /// are saved first, and every conflict not named still refuses.
    #[arg(long, value_name = "PATH")]
    pub replace: Vec<String>,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}
