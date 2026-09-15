//! Writing one rendered candidate into a target.
//!
//! The candidate says what the target should hold; this layer puts it
//! there. It owns the checks that come before the first byte moves, the
//! one lock a target takes, and the order the writes happen in.

pub mod apply;
pub mod lock;
