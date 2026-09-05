//! Pure domain types and invariants.
//!
//! Everything here is computable without I/O: identifiers, the instance
//! manifest schema, profiles, marker-block string surgery, and version
//! ordering. No filesystem access, no process state — modules that touch
//! the world live in `services` and `adapters`.

pub mod finding;
pub mod gate_id;
pub mod manifest;
pub mod marker;
pub mod ownership;
pub mod profile;
pub mod rule_id;
pub mod skill_record;
pub mod tracking;
pub mod version;
