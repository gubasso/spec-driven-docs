//! `gate` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

use crate::domain::gate_id::GateId;

/// Run one delivered gate against the working directory.
#[derive(Debug, clap::Args)]
pub struct GateArgs {
    /// The gate to run.
    #[arg(value_enum, required_unless_present_any = ["list", "explain"])]
    pub id: Option<GateId>,

    /// Files to judge, or extra record roots — whatever the gate's hook
    /// wiring passes.
    ///
    /// A path a gate judges is filtered; a record root it resolves is not.
    #[arg(conflicts_with = "explain")]
    pub files: Vec<String>,

    /// List every delivered gate instead of running one.
    #[arg(long, conflicts_with = "id")]
    pub list: bool,

    /// Report which gates judge a path, and the pattern that decided each
    /// answer, instead of running anything.
    #[arg(long, value_name = "PATH", conflicts_with = "list")]
    pub explain: Option<String>,

    /// Add one include glob to the gate's declared includes. Repeatable.
    ///
    /// A flag adds to the row's declaration rather than replacing it, and an
    /// exclude beats an include.
    #[arg(long, value_name = "GLOB")]
    pub include: Vec<String>,

    /// Add one exclude glob to the gate's declared excludes. Repeatable.
    #[arg(long, value_name = "GLOB")]
    pub exclude: Vec<String>,
}
