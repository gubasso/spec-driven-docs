//! `stage` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use camino::Utf8PathBuf;

use crate::domain::profile::ProfileId;

/// Render this binary's candidate into a stage an agent reads.
#[derive(Debug, clap::Args)]
pub struct StageArgs {
    /// What to do. Rendering a stage is the default.
    #[command(subcommand)]
    pub command: Option<StageCommand>,

    /// The repository to render a candidate for; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// The profile to project.
    #[arg(long, value_enum, default_value_t = ProfileId::Codebase)]
    pub profile: ProfileId,

    /// Where to write the stage; absolute.
    #[arg(long)]
    pub output: Option<Utf8PathBuf>,

    /// The documentation scratch the candidate would record, or `none`.
    #[arg(long)]
    pub docs_scratch: Option<String>,

    /// A path to record under `reserved:` in the declaration. Repeatable.
    #[arg(long)]
    pub reserve: Vec<String>,

    /// The writing-style selection the candidate would record.
    #[arg(long)]
    pub writing_style: Option<String>,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// The verbs `sdd stage` offers beyond rendering one.
#[derive(Debug, clap::Subcommand)]
pub enum StageCommand {
    /// Remove one stage this tool wrote.
    Clean(StageCleanArgs),
}

/// Remove one stage this tool wrote.
#[derive(Debug, clap::Args)]
pub struct StageCleanArgs {
    /// The stage directory to remove; absolute.
    pub path: Utf8PathBuf,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}
