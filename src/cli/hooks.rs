//! `hooks` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

use crate::services::hooks_render::Style;

/// Render the delivered gate set as pre-commit hook entries.
#[derive(Debug, clap::Args)]
pub struct HooksArgs {
    /// The output shape: a managed block for an instance's configuration,
    /// or the top-level manifest published to remote-hook consumers.
    #[arg(long, value_enum, default_value = "block")]
    pub style: Style,

    /// What replaces `{docs_root}` in wiring patterns — a literal root for an
    /// instance, a pattern for the published manifest.
    #[arg(long, default_value = "_docs")]
    pub docs_root: String,

    /// The command prefix every entry invokes.
    #[arg(long, default_value = "sdd")]
    pub entry: String,

    /// The pre-commit language the entries declare.
    #[arg(long, default_value = "system")]
    pub language: String,

    /// The sequence-item indentation of the consumer's repos entries.
    #[arg(long, default_value = "  ")]
    pub indent: String,
}
