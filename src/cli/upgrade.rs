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

    /// Answer one decision the interval raises, as `<decision-id>=<answer>`.
    ///
    /// Repeatable. A release between here and the destination can ask
    /// something only a person can do, and this is how the answer travels
    /// through the short form. `sdd reconcile plan` presents each one with
    /// its body first.
    #[arg(long = "set", value_name = "DECISION=ANSWER")]
    pub set: Vec<String>,
}
