//! Subcommand handlers.
//!
//! One module per subcommand. Each exposes a free function
//! `run(ctx, args) -> Result<(), AppError>` that projects clap args, calls
//! the gates or services, renders through `output`, and returns a typed
//! error. No clap derives here.

pub mod assess;
pub mod completions;
pub mod doctor;
pub mod gate;
pub mod hooks;
pub mod init;
pub mod ki;
pub mod license;
pub mod man;
pub mod read;
pub mod self_manifest;
pub mod skill;
pub mod status;
pub mod upgrade;
pub mod verify;
