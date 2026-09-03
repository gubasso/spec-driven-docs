//! Arguments for `sdd assess`.

use camino::Utf8PathBuf;

/// Classify a target repository before anything lands.
///
/// Reporting only: every classification exits 0, and nothing is written.
#[derive(Debug, clap::Args)]
pub struct AssessArgs {
    /// The repository to assess; `.` is the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Emit one JSON object on stdout instead of the human report.
    #[arg(long)]
    pub json: bool,
}
