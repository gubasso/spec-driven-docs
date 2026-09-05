//! `track` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use camino::Utf8PathBuf;

/// Report or check the tracking registry.
#[derive(Debug, clap::Args)]
pub struct TrackArgs {
    /// The operation to run.
    #[command(subcommand)]
    pub command: TrackCommand,
}

/// The two tracking operations.
#[derive(Debug, clap::Subcommand)]
pub enum TrackCommand {
    /// Report registry freshness offline.
    Status(StatusArgs),
    /// Compare each pinned Git revision to its upstream over the network.
    Check(CheckArgs),
}

/// `sdd track status`: an offline freshness report.
#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    /// The instance to read; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Judge freshness as of this ISO date rather than today.
    #[arg(long)]
    pub as_of: Option<String>,

    /// Emit one JSON object rather than lines.
    #[arg(long)]
    pub json: bool,
}

/// `sdd track check`: the one network-enabled operation.
#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    /// The instance to read; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Check only the entry with this id.
    #[arg(long)]
    pub id: Option<String>,

    /// Exit 1 when a completed lookup finds a different revision.
    #[arg(long)]
    pub fail_on_update: bool,

    /// Emit one JSON object rather than lines.
    #[arg(long)]
    pub json: bool,
}
