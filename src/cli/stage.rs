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

    /// The profile to project: the recorded one where an instance exists,
    /// which a different value refuses; codebase where nothing is recorded.
    #[arg(long, value_enum)]
    pub profile: Option<ProfileId>,

    /// Where to write the stage; absolute.
    #[arg(long)]
    pub output: Option<Utf8PathBuf>,

    /// The documentation scratch the candidate would record, or `none`.
    #[arg(long)]
    pub docs_scratch: Option<String>,

    /// A path to record under `reserved:` in the declaration. Repeatable.
    #[arg(long)]
    pub reserve: Vec<String>,

    /// The writing-style selection the candidate would record, unless the
    /// declaration already holds it; a missing key holds `builtin`.
    #[arg(long)]
    pub writing_style: Option<String>,

    /// Record that every host installs sdd itself, at this release or newer,
    /// where no manager file pins it.
    #[arg(long, value_enum)]
    pub acquisition: Option<AcquisitionChoice>,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// How the project obtains `sdd`, where no manager file records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum AcquisitionChoice {
    /// Each host installs `sdd` itself.
    Host,
}

/// The verbs `sdd stage` offers beyond rendering one.
#[derive(Debug, clap::Subcommand)]
pub enum StageCommand {
    /// Remove one stage this tool wrote.
    Clean(StageCleanArgs),
    /// Compare one stage against its target, one row per destination.
    Compare(StageCompareArgs),
}

/// Compare one stage against the target it was rendered for.
#[derive(Debug, clap::Args)]
pub struct StageCompareArgs {
    /// The stage directory to read; absolute.
    pub root: Utf8PathBuf,

    /// The repository the stage was rendered for; absolute, or the working
    /// directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
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
