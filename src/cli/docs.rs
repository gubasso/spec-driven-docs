//! `docs` subcommand: parse-shape.
//!
//! One positional taking any number of words, so `sdd docs context budget`
//! and `sdd docs "context budget"` reach the same alias. No `--index`
//! flag: the bare verb is the index, which keeps the always-loaded route
//! one clause and gives an agent no flag to half-remember.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

/// Read this binary's own corpus: the index, or one topic.
#[derive(Debug, clap::Args)]
pub struct DocsArgs {
    /// The topic, as one or more words. With none, print the index.
    #[arg(value_name = "TOPIC")]
    pub topic: Vec<String>,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}
