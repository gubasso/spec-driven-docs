//! Shared parse-shape for the embedded-document readers.
//!
//! `method`, `spec`, and `template` all take one optional name and a
//! `--list` flag; the shelf they read is the subcommand's identity.
//! No I/O, no business logic.

/// Read one embedded document, or list the shelf.
#[derive(Debug, clap::Args)]
pub struct ReadArgs {
    /// The document's short name.
    #[arg(required_unless_present = "list")]
    pub name: Option<String>,

    /// List every document on this shelf instead.
    #[arg(long, conflicts_with = "name")]
    pub list: bool,
}
