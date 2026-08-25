//! `license` subcommand: parse-shape.
//!
//! Holds the clap derive struct only. No I/O, no business logic.

/// Print the license terms this binary carries.
#[derive(Debug, Clone, Copy, clap::Args)]
pub struct LicenseArgs {
    /// Print the CC BY 4.0 text covering the method.
    #[arg(long)]
    pub method: bool,

    /// Print the MIT text covering the distribution.
    #[arg(long)]
    pub payload: bool,
}
