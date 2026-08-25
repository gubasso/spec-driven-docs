//! Canon version: one strict semantic-version triple.
//!
//! Holds parsing, ordering, and rendering only. What a version difference
//! means for an instance — compatible, upgradable, ahead — is the
//! verifier's and upgrader's judgment, not this type's.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A `MAJOR.MINOR.PATCH` version as released by the canon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonVersion {
    /// Breaking position.
    pub major: u64,
    /// Feature position (breaking while the major is zero).
    pub minor: u64,
    /// Fix position.
    pub patch: u64,
}

impl CanonVersion {
    /// The version this binary was built as.
    #[must_use]
    pub fn current() -> Self {
        Self::from_str(env!("CARGO_PKG_VERSION")).unwrap_or(Self {
            major: 0,
            minor: 0,
            patch: 0,
        })
    }
}

impl fmt::Display for CanonVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Rejection of a string that is not a strict `X.Y.Z` triple.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("'{0}' is not a semantic version")]
pub struct VersionError(String);

impl FromStr for CanonVersion {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || VersionError(s.to_string());
        let mut parts = s.split('.');
        let mut next = || -> Result<u64, VersionError> {
            let part = parts.next().ok_or_else(err)?;
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return Err(err());
            }
            part.parse().map_err(|_| err())
        };
        let version = Self {
            major: next()?,
            minor: next()?,
            patch: next()?,
        };
        if parts.next().is_some() {
            return Err(err());
        }
        Ok(version)
    }
}

impl Serialize for CanonVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for CanonVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_triple() {
        let v: CanonVersion = "0.2.0".parse().unwrap();
        assert_eq!((v.major, v.minor, v.patch), (0, 2, 0));
        assert_eq!(v.to_string(), "0.2.0");
    }

    #[test]
    fn rejects_non_triples() {
        for bad in ["", "1.2", "1.2.3.4", "v1.2.3", "1.2.x", "1..3", "1.2.3-rc1"] {
            assert!(bad.parse::<CanonVersion>().is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn orders_numerically() {
        let a: CanonVersion = "0.9.0".parse().unwrap();
        let b: CanonVersion = "0.10.0".parse().unwrap();
        assert!(a < b);
    }

    #[test]
    fn current_matches_cargo_version() {
        assert_eq!(
            CanonVersion::current().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn serde_round_trips_as_a_string() {
        let v: CanonVersion = "1.4.2".parse().unwrap();
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"1.4.2\"");
        assert_eq!(serde_json::from_str::<CanonVersion>(&json).unwrap(), v);
    }
}
