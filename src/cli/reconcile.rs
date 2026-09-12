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
    /// Compute one plan, store it, and print it.
    Plan(PlanArgs),
    /// Render one stored plan, or the latest result for its id.
    Show(ShowArgs),
    /// Execute one stored plan, or refuse because its inputs moved.
    Apply(ApplyArgs),
}

/// Render one stored plan, or the latest result for its id.
#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    /// The plan's id, which is its fingerprint.
    pub plan_id: String,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// Execute one stored plan.
#[derive(Debug, clap::Args)]
pub struct ApplyArgs {
    /// The plan's id, which is its fingerprint.
    pub plan_id: String,

    /// The repository to apply to; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// Compute one plan, store it, and print it.
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
