//! Spec-driven documentation: current specs, immutable decision records, and
//! executable gates kept coherent for people and coding agents.
//!
//! This crate is the canon's distribution: the `sdd` binary installs, verifies,
//! upgrades, and gates instances, and carries the whole payload — spec seeds,
//! templates, lint configs, and the method chapters — embedded at compile
//! time. The method itself lives in the repository's
//! markdown, not here; this crate only ships and enforces it.
//!
//! Major modules:
//! - [`domain`] — pure types and invariants: manifest, profiles, rule and gate
//!   identifiers, marker-block splicing, versions.
//! - [`embedded`] — every compile-time embedded asset and its accessors.
//! - [`gates`] — the delivered gate implementations and their registry.
//! - [`services`] — install, verify, upgrade, and hook-rendering orchestration.
//! - [`adapters`] — filesystem I/O at the edges.
//! - [`plan`] — the typed document every landing write comes from.
//! - [`release`] — the bundle and resolver boundary every landing verb
//!   reads a release through.
//! - [`self_depend`] — how a consumer pins this tool and keeps the pin
//!   fresh.
//! - [`transaction`] — the lock, staging, and journal a recoverable
//!   multi-file write runs through.
//! - [`cli`] / [`commands`] — clap parse shapes and their handlers.
//! - [`error`] — [`error::AppError`] and the exit-code matrix.
//!
//! # The supported target
//!
//! Linux is the only target this crate supports
//! (ADR-linux-is-the-only-supported-target). The refusal below states that
//! boundary to the compiler rather than to a reader, because the source is
//! otherwise portable enough that a build elsewhere would succeed and then
//! keep promises nothing here proves. The guard names the operating system
//! and not the architecture: no source in this crate varies by
//! architecture, and `dist-workspace.toml` states which architecture the
//! release builds.

#[cfg(not(target_os = "linux"))]
compile_error!(
    "spec-driven-docs supports Linux only. Its permission, path, and \
     filesystem behavior is written and tested against one target, and a \
     build for another would carry promises this project does not keep. \
     See _docs/decisions/ADR-linux-is-the-only-supported-target.md."
);

pub mod adapters;
pub mod candidate;
pub mod cli;
pub mod commands;
pub mod context;
pub mod domain;
pub mod embedded;
pub mod error;
pub mod gates;
pub mod logging;
pub mod output;
pub mod payload_roots;
pub mod plan;
pub mod probes;
pub mod release;
pub mod self_depend;
pub mod services;
pub mod transaction;
