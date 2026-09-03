//! Orchestration services: the work behind the lifecycle commands.
//!
//! Services compose domain types, embedded assets, and filesystem adapters
//! into the install, verify, upgrade, and rendering operations. Argument
//! parsing stays in `cli`, printing in `output`; nothing here reads argv or
//! writes a terminal.

pub mod assess;
pub mod hooks_render;
pub mod installer;
pub mod reader;
pub mod self_manifest;
pub mod skill_installer;
pub mod status;
pub mod upgrader;
pub mod verifier;
