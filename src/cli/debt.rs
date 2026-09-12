//! `debt` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use camino::Utf8PathBuf;

/// Record, migrate, or tighten the inherited budget violations a project
/// carries in `.spec-driven-docs/debt.yaml`.
#[derive(Debug, clap::Args)]
pub struct DebtArgs {
    /// The verb.
    #[command(subcommand)]
    pub verb: DebtVerb,
}

/// The three ways a debt file changes, each previewing by default.
#[derive(Debug, clap::Subcommand)]
pub enum DebtVerb {
    /// Record every current violation as debt. Refuses where a debt file
    /// exists, because a baseline never widens one.
    Baseline(DebtVerbArgs),
    /// Convert the legacy chapter list into the dimensional file, then
    /// remove the list. Preserves every exemption and broadens nothing.
    Migrate(DebtVerbArgs),
    /// Lower every ceiling to its current measurement and clear every
    /// corrected exception. Never raises a ceiling.
    Tighten(DebtVerbArgs),
}

/// What every verb takes.
#[derive(Debug, clap::Args)]
pub struct DebtVerbArgs {
    /// The instance to act on; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Write the file. Without it the verb prints what it would write and
    /// changes nothing.
    #[arg(long)]
    pub apply: bool,
}
