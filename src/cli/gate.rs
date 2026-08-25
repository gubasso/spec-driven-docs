//! `gate` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

use crate::domain::gate_id::GateId;

/// Run one delivered gate against the working directory.
#[derive(Debug, clap::Args)]
pub struct GateArgs {
    /// The gate to run.
    #[arg(value_enum, required_unless_present = "list")]
    pub id: Option<GateId>,

    /// Files to judge, or extra record roots — whatever the gate's hook
    /// wiring passes.
    pub files: Vec<String>,

    /// List every delivered gate instead of running one.
    #[arg(long, conflicts_with = "id")]
    pub list: bool,
}
