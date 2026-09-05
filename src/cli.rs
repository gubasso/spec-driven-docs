//! Root clap parser.
//!
//! Holds the top-level [`Cli`], the [`Commands`] enum, and the global args.
//! Per-subcommand arg structs live in sibling files (`cli/<name>.rs`). No
//! business logic anywhere in this module tree.

pub mod assess;
pub mod completions;
pub mod doctor;
pub mod gate;
pub mod hooks;
pub mod init;
pub mod ki;
pub mod license;
pub mod read;
pub mod skill;
pub mod status;
pub mod track;
pub mod upgrade;
pub mod verify;

use clap::{ArgAction, Parser, Subcommand};

/// The `sdd` command line.
#[derive(Debug, Parser)]
#[command(name = "sdd", version, about, long_about = None)]
pub struct Cli {
    /// Flags every subcommand shares.
    #[command(flatten)]
    pub global: GlobalArgs,

    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// Flags every subcommand shares.
#[derive(Debug, clap::Args)]
pub struct GlobalArgs {
    /// Increase log verbosity (-v info, -vv debug, -vvv trace).
    /// Overridden by `RUST_LOG` if set.
    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub verbose: u8,
}

/// Every subcommand `sdd` offers.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Install the payload into a target repository.
    Init(init::InitArgs),
    /// Verify an installed instance offline.
    Verify(verify::VerifyArgs),
    /// Upgrade an installed instance to this binary's version.
    Upgrade(upgrade::UpgradeArgs),
    /// Run one delivered gate, or list them all.
    Gate(gate::GateArgs),
    /// Render the delivered gate set as pre-commit hook entries.
    Hooks(hooks::HooksArgs),
    /// Read the known-issue zone.
    Ki(ki::KiArgs),
    /// Read a method chapter, or list them.
    Method(read::ReadArgs),
    /// Read a spec seed, or list them.
    Spec(read::ReadArgs),
    /// Read a document template, or list them.
    Template(read::ReadArgs),
    /// Read the embedded skills, or install them for coding agents.
    Skill(skill::SkillArgs),
    /// Report an instance's state without gating on it.
    Status(status::StatusArgs),
    /// Report tracking freshness offline, or check upstreams over the network.
    Track(track::TrackArgs),
    /// Probe this host's readiness and report by class; never fails.
    Doctor(doctor::DoctorArgs),
    /// Classify a target repository before anything lands; every classification exits 0.
    Assess(assess::AssessArgs),
    /// Print the license terms this binary carries.
    License(license::LicenseArgs),
    /// Regenerate the canon checkout's own instance manifest.
    SelfManifest,
    /// Generate shell completions.
    Completions(completions::CompletionsArgs),
    /// Render the manual page.
    Man,
}

/// Every subcommand name the binary answers to.
#[must_use]
pub fn subcommand_names() -> Vec<String> {
    <Cli as clap::CommandFactory>::command()
        .get_subcommands()
        .map(|command| command.get_name().to_string())
        .collect()
}
