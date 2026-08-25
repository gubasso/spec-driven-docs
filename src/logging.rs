//! tracing-subscriber installation. Called once from `main`.
//!
//! Honors `RUST_LOG` when set; otherwise derives a directive from the
//! `-v`/`-vv`/`-vvv` flag count. Diagnostics go to stderr so stdout stays
//! reserved for command output. No other module installs a subscriber.

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

/// Install the process-wide subscriber.
///
/// # Errors
///
/// Fails when a subscriber is already installed.
pub fn init(verbosity: u8) -> anyhow::Result<()> {
    let default_directive = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_directive));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr).with_target(false))
        .try_init()
        .map_err(|e| anyhow::anyhow!("install tracing subscriber: {e}"))
}
