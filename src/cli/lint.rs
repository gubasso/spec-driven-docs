//! `lint` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use camino::Utf8PathBuf;

use crate::domain::profile::ProfileId;

/// Report what the Markdown linter will read, or prove its link rule runs.
#[derive(Debug, clap::Args)]
pub struct LintArgs {
    /// What to do. The report is the default.
    #[command(subcommand)]
    pub command: Option<LintCommand>,

    /// The repository to read; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// The candidate profile to read before a landing; an instance reads
    /// its recorded profile, which a different value refuses.
    #[arg(long, value_enum)]
    pub profile: Option<ProfileId>,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// The verbs `sdd lint` offers beyond the report.
#[derive(Debug, clap::Subcommand)]
pub enum LintCommand {
    /// Prove the relative-links rule fires: lint a broken link, then a valid
    /// one, through the delivered hook, and remove the scratch file.
    Prove(LintProveArgs),
}

/// Prove the relative-links rule fires.
#[derive(Debug, clap::Args)]
pub struct LintProveArgs {
    /// The repository to prove; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}
