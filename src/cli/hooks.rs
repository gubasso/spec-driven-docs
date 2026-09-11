//! `hooks` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

/// Render the delivered gate set as the managed pre-commit block.
#[derive(Debug, clap::Args)]
pub struct HooksArgs {
    /// What replaces `{docs_root}` in wiring patterns.
    ///
    /// Omitted, the target's recorded documentation root is used where the
    /// target is an instance, and `_docs` otherwise. A block rendered
    /// against the wrong root would disagree with the one the installer
    /// wrote.
    #[arg(long)]
    pub docs_root: Option<String>,

    /// The command prefix every entry invokes.
    #[arg(long, default_value = "sdd")]
    pub entry: String,

    /// The sequence-item indentation of the consumer's repos entries.
    #[arg(long, default_value = "  ")]
    pub indent: String,

    /// The instance whose declaration the block is rendered from.
    #[arg(long, default_value = ".")]
    pub target: String,

    /// Write the rendered block into the target's `.pre-commit-config.yaml`,
    /// replacing the managed region.
    ///
    /// Run this after editing the declaration. It works at the same version,
    /// which `sdd upgrade` does not.
    #[arg(long, conflicts_with = "check")]
    pub apply: bool,

    /// Exit non-zero when the target's managed region does not match what
    /// the declaration renders.
    #[arg(long)]
    pub check: bool,
}
