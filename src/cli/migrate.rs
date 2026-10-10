//! `migrate` subcommand: parse-shape.
//!
//! Holds the clap derive structs only. No I/O, no business logic.

use camino::Utf8PathBuf;

use crate::domain::migration::Status;

/// Archive a corpus, track its rebuild, and close the migration.
#[derive(Debug, clap::Args)]
pub struct MigrateArgs {
    /// What to do.
    #[command(subcommand)]
    pub command: MigrateCommand,
}

/// The verbs `sdd migrate` offers.
#[derive(Debug, clap::Subcommand)]
pub enum MigrateCommand {
    /// List every document the target holds, with its kind and checksum.
    Inventory(TargetArgs),
    /// Move every document but the root metadata into the archive, and open
    /// the migration; without --apply nothing is written.
    Archive(ArchiveArgs),
    /// Move a source, or every source of a group, to a status.
    Mark(MarkArgs),
    /// Report where every source of the open migration stands.
    Status(TargetArgs),
    /// End the open migration once every source is verified or dropped;
    /// without --apply nothing is written.
    Close(CloseArgs),
}

/// The repository a read-only verb reads.
#[derive(Debug, clap::Args)]
pub struct TargetArgs {
    /// The repository to read; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// Move the corpus into the archive.
#[derive(Debug, clap::Args)]
pub struct ArchiveArgs {
    /// The repository to migrate; absolute, or the working directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Write: move the documents, write the registry, and record the open
    /// migration in the declaration.
    #[arg(long)]
    pub apply: bool,

    /// Recreate the open migration's archive from Git at its base commit,
    /// on this clone or another one.
    #[arg(long)]
    pub rebuild: bool,

    /// The trunk every archived document must match byte for byte: a
    /// branch or a commit. By default, the local branch the remote's HEAD
    /// names. A rebuild reads its base from the registry or from Git.
    #[arg(long, value_name = "REF", conflicts_with = "rebuild")]
    pub trunk: Option<String>,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// Move a source or a group to a status.
#[derive(Debug, clap::Args)]
pub struct MarkArgs {
    /// A source id or path, or a group id.
    pub name: String,

    /// The status it takes.
    #[arg(value_enum)]
    pub status: Status,

    /// The repository of the open migration; absolute, or the working
    /// directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// File the source into this group, created where it is new.
    #[arg(long, value_name = "GROUP")]
    pub group: Option<String>,

    /// Tag the source with this topic. Repeatable.
    #[arg(long, value_name = "TAG")]
    pub tag: Vec<String>,

    /// Give the group this destination. Repeatable.
    #[arg(long, value_name = "PATH")]
    pub destination: Vec<Utf8PathBuf>,

    /// Why the source is dropped; required with `dropped`.
    #[arg(long)]
    pub reason: Option<String>,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}

/// End the open migration.
#[derive(Debug, clap::Args)]
pub struct CloseArgs {
    /// The repository of the open migration; absolute, or the working
    /// directory.
    #[arg(long, default_value = ".")]
    pub target: Utf8PathBuf,

    /// Write: remove the migration from the declaration.
    #[arg(long)]
    pub apply: bool,

    /// Print one JSON object instead of text.
    #[arg(long)]
    pub json: bool,
}
