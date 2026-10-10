//! `init` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

use camino::Utf8PathBuf;

use crate::domain::profile::ProfileId;

/// Install the payload into a target repository.
#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// The target repository; must be an absolute path. With `--stage`, it
    /// defaults to the working directory and must be the stage's target.
    #[arg(long, required_unless_present = "stage")]
    pub target: Option<Utf8PathBuf>,

    /// The profile to install.
    #[arg(long, value_enum, required_unless_present = "stage")]
    pub profile: Option<ProfileId>,

    /// Land the choices a stage recorded: its profile, docs scratch,
    /// writing style, reserved paths, and acquisition decision. The landing
    /// still renders every byte from this binary; only the stage's receipt
    /// is read, and a receipt another release wrote refuses.
    #[arg(
        long,
        value_name = "ROOT",
        conflicts_with_all = ["profile", "reserve", "docs_scratch", "writing_style"]
    )]
    pub stage: Option<Utf8PathBuf>,

    /// Land beside a corpus that stays, on a target classified `migration`,
    /// with no archive, and record each budget finding of the corpus as
    /// debt. The operator names this scope: `sdd migrate` is the default
    /// for such a target.
    #[arg(long)]
    pub incremental: bool,

    /// Write into a non-empty target that has no instance yet.
    #[arg(long, conflicts_with = "dry_run")]
    pub apply: bool,

    /// Preview only; write nothing.
    #[arg(long)]
    pub dry_run: bool,

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

    /// Where the writing style comes from: `builtin` routes authors to
    /// `sdd method writing-style`, `project:<PATH>` routes them to the
    /// project's own document, and `none` installs no route and imposes no
    /// conversion obligation. Recorded in the instance's declaration,
    /// unless the declaration already holds that selection; a missing key
    /// holds `builtin`. Omitted, the recorded value stays.
    #[arg(long, value_name = "SELECTION")]
    pub writing_style: Option<String>,
}
