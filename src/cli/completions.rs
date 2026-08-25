//! `completions` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

/// Generate shell completions.
#[derive(Debug, Clone, Copy, clap::Args)]
pub struct CompletionsArgs {
    /// The shell to generate for.
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}
