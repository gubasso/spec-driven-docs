//! `payload` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

/// Report what a release carries, through the release seam.
#[derive(Debug, clap::Args)]
pub struct PayloadArgs {
    /// A published version, or the release this binary carries.
    ///
    /// `latest` resolves the highest stable, non-yanked release at the
    /// registry. An exact version is fetched, verified against the index
    /// checksum, and cached.
    #[arg(long)]
    pub release: Option<String>,

    /// Forbid every network read.
    ///
    /// An exact version already cached still resolves. `latest` refuses,
    /// because only the index says which release is newest.
    #[arg(long)]
    pub offline: bool,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}
