//! `policy` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use camino::Utf8PathBuf;

/// Bring an adopted specification into agreement with what the project
/// declared, on request and never on upgrade.
#[derive(Debug, clap::Args)]
pub struct PolicyArgs {
    /// The verb.
    #[command(subcommand)]
    pub verb: PolicyVerb,
}

/// The one verb, previewing by default.
#[derive(Debug, clap::Subcommand)]
pub enum PolicyVerb {
    /// Add the rule that authorizes an active declaration to the adopted
    /// specification that owns it, where the local specifications lack it.
    Reconcile(ReconcileArgs),
}

/// What the verb takes.
#[derive(Debug, clap::Args)]
pub struct ReconcileArgs {
    /// The instance to act on; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Write the change. Without it the verb prints every file, the rule
    /// it would add, and the exact text, and changes nothing.
    #[arg(long)]
    pub apply: bool,
}
