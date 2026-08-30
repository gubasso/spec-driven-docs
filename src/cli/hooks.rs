//! `hooks` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

/// Render the delivered gate set as the managed pre-commit block.
#[derive(Debug, clap::Args)]
pub struct HooksArgs {
    /// What replaces `{docs_root}` in wiring patterns.
    #[arg(long, default_value = "_docs")]
    pub docs_root: String,

    /// The command prefix every entry invokes.
    #[arg(long, default_value = "sdd")]
    pub entry: String,

    /// The sequence-item indentation of the consumer's repos entries.
    #[arg(long, default_value = "  ")]
    pub indent: String,
}
