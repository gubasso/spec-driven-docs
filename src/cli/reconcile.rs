//! `reconcile` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use camino::Utf8PathBuf;
use clap::Subcommand;

/// Compute, read, and apply one plan.
#[derive(Debug, clap::Args)]
pub struct ReconcileArgs {
    /// The verb to run.
    #[command(subcommand)]
    pub command: ReconcileCommand,
}

/// Every verb `sdd reconcile` offers.
#[derive(Debug, Subcommand)]
pub enum ReconcileCommand {
    /// Compute one plan and print it. Writes nothing into the target.
    Plan(PlanArgs),
}

/// Compute one plan and print it.
#[derive(Debug, clap::Args)]
pub struct PlanArgs {
    /// The repository to plan for; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// The destination release: `embedded`, `latest`, or a version.
    ///
    /// The default is the release this binary carries, which keeps the
    /// default offline.
    #[arg(long, default_value = "embedded")]
    pub to: String,

    /// Forbid every network read.
    #[arg(long)]
    pub offline: bool,

    /// Answer one decision the plan offers, as `<decision-id>=<answer>`.
    ///
    /// Repeatable. Selecting a decision is what re-plans with it, so an
    /// approval binds to the answers it was given with.
    #[arg(long = "set", value_name = "DECISION=ANSWER")]
    pub set: Vec<String>,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}
