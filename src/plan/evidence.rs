//! What the plan observed, and how.
//!
//! A plan's fields come from several places at once: the instance record,
//! the disk, a release bundle, and sometimes a registry read. A reader who
//! cannot tell which is reading a claim without its source. The ledger is
//! that source: one item per observation, and a reference from every field
//! that rests on it.

use serde::{Deserialize, Serialize};

use crate::domain::ownership::Sha256;

/// Where one observation came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Producer {
    /// The target's working tree.
    Disk,
    /// The instance record.
    Record,
    /// The project's own declaration.
    Declaration,
    /// A release bundle.
    Bundle,
    /// The registry index.
    Registry,
    /// The host this command runs on.
    Host,
}

/// One thing the plan observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// A stable reference other sections cite.
    pub id: String,
    /// What was observed.
    pub about: String,
    /// Who produced it.
    pub producer: Producer,
    /// When, as an RFC 3339 timestamp the caller supplied.
    pub observed_at: String,
    /// The digest of what was read, where the observation has bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<Sha256>,
    /// How it was read, in one phrase.
    pub method: String,
}

/// The ledger a plan carries, in the order the observations were made.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    /// Every observation.
    pub items: Vec<Evidence>,
}

impl Ledger {
    /// An empty ledger.
    #[must_use]
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Record one observation and return the reference to cite.
    pub fn record(
        &mut self,
        id: &str,
        about: &str,
        producer: Producer,
        observed_at: &str,
        sha256: Option<Sha256>,
        method: &str,
    ) -> String {
        self.items.push(Evidence {
            id: id.to_string(),
            about: about.to_string(),
            producer,
            observed_at: observed_at.to_string(),
            sha256,
            method: method.to_string(),
        });
        id.to_string()
    }

    /// Whether the ledger carries one reference.
    #[must_use]
    pub fn holds(&self, id: &str) -> bool {
        self.items.iter().any(|item| item.id == id)
    }

    /// Every reference, in order.
    #[must_use]
    pub fn ids(&self) -> Vec<&str> {
        self.items.iter().map(|item| item.id.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recorded_observation_is_citable_by_the_reference_it_returns() {
        let mut ledger = Ledger::new();
        let reference = ledger.record(
            "record",
            "the instance manifest",
            Producer::Record,
            "2026-09-12T00:00:00Z",
            Some(Sha256::of(b"x")),
            "read from disk",
        );
        assert_eq!(reference, "record");
        assert!(ledger.holds("record"));
        assert!(!ledger.holds("absent"));
        assert_eq!(ledger.ids(), ["record"]);
    }

    #[test]
    fn an_observation_without_bytes_carries_no_digest() {
        let mut ledger = Ledger::new();
        ledger.record(
            "host",
            "the resolved paths",
            Producer::Host,
            "2026-09-12T00:00:00Z",
            None,
            "read from the environment",
        );
        assert_eq!(ledger.items[0].sha256, None);
        assert_eq!(ledger.items[0].producer, Producer::Host);
    }
}
