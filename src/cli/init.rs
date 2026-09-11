//! `init` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

use camino::Utf8PathBuf;

use crate::domain::profile::ProfileId;

/// Install the payload into a target repository.
#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// The target repository; must be an absolute path.
    #[arg(long)]
    pub target: Utf8PathBuf,

    /// The profile to install.
    #[arg(long, value_enum)]
    pub profile: ProfileId,

    /// Write into a non-empty target that has no instance yet.
    #[arg(long, conflicts_with = "dry_run")]
    pub apply: bool,

    /// Preview only; write nothing.
    #[arg(long)]
    pub dry_run: bool,

    /// Where the planning tool writes entry documents: a repository-relative
    /// path, `untracked:<PATH>`, `env`, or `none`. Write `./none` or `./env`
    /// for a directory carrying either name. Omitted, the recorded value
    /// stays.
    #[arg(long, value_name = "VALUE")]
    pub plan_zone: Option<String>,

    /// A path no delivered gate judges, recorded under `reserved:` in the
    /// instance's declaration. Repeatable.
    ///
    /// Use it for a region another tool owns. Omitted, a recorded value
    /// stays.
    #[arg(long, value_name = "PATH")]
    pub reserve: Vec<String>,

    /// Where material that is not a statement yet is kept, as a path that may
    /// leave the repository, or `none` to clear a recorded value. Omitted,
    /// the recorded value stays.
    #[arg(long, value_name = "PATH")]
    pub docs_scratch: Option<String>,
}
