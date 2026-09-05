//! I/O at the edges.
//!
//! Adapters wrap the filesystem so services stay testable over plain values.
//! No policy lives here — what to hash, copy, or refuse is the services'
//! judgment.

pub mod fs;
pub mod git;
