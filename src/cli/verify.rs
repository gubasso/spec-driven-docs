//! `verify` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

use camino::Utf8PathBuf;

/// Verify an installed instance offline.
#[derive(Debug, clap::Args)]
pub struct VerifyArgs {
    /// The instance to verify; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,
}
