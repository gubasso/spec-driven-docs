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
//! - [`cli`] / [`commands`] — clap parse shapes and their handlers.
//! - [`error`] — [`error::AppError`] and the exit-code matrix.

pub mod adapters;
pub mod cli;
pub mod commands;
pub mod context;
pub mod domain;
pub mod embedded;
pub mod error;
pub mod gates;
pub mod logging;
pub mod output;
pub mod services;
