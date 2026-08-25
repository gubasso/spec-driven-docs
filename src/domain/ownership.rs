//! Ownership classes: how an instance holds each file the canon delivered.
//!
//! Three classes exist. Managed files are byte projections of the payload;
//! adopted files are seeded once and owned locally against a recorded
//! baseline; integration blocks are marked regions inside files the project
//! owns. These are the record shapes only — hashing bytes and comparing
//! them to disk is the verifier's work.

use std::fmt;
use std::str::FromStr;

use camino::Utf8PathBuf;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A lowercase-hex SHA-256 digest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sha256(String);

impl Sha256 {
    /// Digest a byte string.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        use sha2::Digest;
        Self(hex::encode(sha2::Sha256::digest(bytes)))
    }

    /// The 64-character hex form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Rejection of a string that is not a 64-character lowercase hex digest.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("'{0}' is not a sha256 digest")]
pub struct Sha256Error(String);

impl FromStr for Sha256 {
    type Err = Sha256Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            Ok(Self(s.to_string()))
        } else {
            Err(Sha256Error(s.to_string()))
        }
    }
}

impl Serialize for Sha256 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Sha256 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// A byte-for-byte projection of a payload file into the instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedEntry {
    /// The payload path the bytes came from.
    pub source: Utf8PathBuf,
    /// Where the instance holds them, relative to its root.
    pub destination: Utf8PathBuf,
    /// The installed bytes; any difference on disk is drift.
    pub sha256: Sha256,
}

/// A file seeded from the payload and owned by the instance from then on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdoptedEntry {
    /// The payload path the seed came from.
    pub source: Utf8PathBuf,
    /// Where the instance holds its copy, relative to its root.
    pub destination: Utf8PathBuf,
    /// The instance's installed bytes.
    pub sha256: Sha256,
    /// What upstream shipped; an edit reports drift until reconciled.
    pub baseline_sha256: Sha256,
}

/// A marker-delimited region the canon owns inside a project-owned file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationBlock {
    /// The host file, relative to the instance root.
    pub path: Utf8PathBuf,
    /// The bytes between and including the markers.
    pub marker_hash: Sha256,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digests_deterministically() {
        let digest = Sha256::of(b"payload\n");
        assert_eq!(digest.as_str().len(), 64);
        assert_eq!(digest, Sha256::of(b"payload\n"));
        assert_ne!(digest, Sha256::of(b"payload"));
    }

    #[test]
    fn rejects_malformed_digests() {
        for bad in ["", "abc", &"A".repeat(64), &"g".repeat(64)] {
            assert!(bad.parse::<Sha256>().is_err(), "accepted {bad:?}");
        }
        assert!("0123456789abcdef".repeat(4).parse::<Sha256>().is_ok());
    }
}
