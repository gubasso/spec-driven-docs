//! The instance manifest: the persistent record of what an instance holds.
//!
//! Schema version 2. The manifest is what lets a later `sdd` distinguish
//! managed drift from adopted reconciliation and its own version from the
//! instance's. This module owns the shape and its parse-time invariants;
//! reading it from disk, comparing it to bytes, and writing it belong to
//! the services.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::ownership::{AdoptedEntry, IntegrationBlock, ManagedEntry};
use crate::domain::profile::{DocsRoot, ProfileId};
use crate::domain::version::CanonVersion;

/// The manifest schema this binary reads and writes.
pub const SCHEMA_VERSION: u32 = 2;
/// Where the canon is published.
pub const CANON_SOURCE: &str = "https://github.com/gubasso/spec-driven-docs";
/// The instance directory, relative to the instance root.
pub const INSTANCE_DIR: &str = ".spec-driven-docs";
/// The manifest path, relative to the instance root.
pub const MANIFEST_PATH: &str = ".spec-driven-docs/manifest.json";

/// Everything an instance records about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Always [`SCHEMA_VERSION`] once parsed.
    pub schema_version: u32,
    /// The canon release that produced the installed payload.
    pub canon_version: CanonVersion,
    /// Where that canon is published.
    pub canon_source: String,
    /// The profile the instance was installed with.
    pub profile: ProfileId,
    /// The documentation root the gates read.
    pub docs_root: DocsRoot,
    /// When the instance was first installed; preserved across reinstalls.
    pub installed_at: String,
    /// Byte projections the canon keeps owning.
    pub managed_files: Vec<ManagedEntry>,
    /// Files the instance owns against a recorded baseline.
    pub adopted_files: Vec<AdoptedEntry>,
    /// Marked regions the canon owns inside project files.
    pub integration_blocks: Vec<IntegrationBlock>,
}

/// A manifest that could not be accepted.
#[derive(Debug, Error)]
pub enum ManifestParseError {
    /// Not JSON, or JSON that does not fit the schema.
    #[error("invalid manifest schema: {0}")]
    Invalid(String),
    /// A well-formed manifest of an older schema; upgradable, not readable.
    #[error("manifest schema_version {0} is older than this binary's; run 'sdd upgrade'")]
    Older(u32),
    /// A well-formed manifest of a newer schema; this binary is too old.
    #[error("manifest schema_version {0} is newer than this binary's; upgrade sdd")]
    Newer(u32),
}

impl Manifest {
    /// Parse and validate a serialized manifest.
    ///
    /// # Errors
    ///
    /// [`ManifestParseError::Older`] / [`ManifestParseError::Newer`] when the
    /// recorded schema version is not [`SCHEMA_VERSION`], and
    /// [`ManifestParseError::Invalid`] for anything that does not fit the
    /// schema or records no managed file.
    pub fn parse(json: &str) -> Result<Self, ManifestParseError> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| ManifestParseError::Invalid(e.to_string()))?;
        match value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
        {
            Some(v) if v == u64::from(SCHEMA_VERSION) => {}
            Some(v) if v < u64::from(SCHEMA_VERSION) => {
                return Err(ManifestParseError::Older(u32::try_from(v).unwrap_or(0)));
            }
            Some(v) => {
                return Err(ManifestParseError::Newer(
                    u32::try_from(v).unwrap_or(u32::MAX),
                ));
            }
            None => {
                return Err(ManifestParseError::Invalid(
                    "no numeric schema_version".to_string(),
                ));
            }
        }
        let manifest: Self = serde_json::from_value(value)
            .map_err(|e| ManifestParseError::Invalid(e.to_string()))?;
        if manifest.managed_files.is_empty() {
            return Err(ManifestParseError::Invalid(
                "managed_files is empty".to_string(),
            ));
        }
        let mut paths = std::collections::BTreeSet::new();
        for block in &manifest.integration_blocks {
            if !paths.insert(&block.path) {
                return Err(ManifestParseError::Invalid(format!(
                    "duplicate integration block path: {}",
                    block.path
                )));
            }
        }
        Ok(manifest)
    }

    /// Serialize in the canonical on-disk form: two-space indent, trailing newline.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut json = serde_json::to_string_pretty(self).unwrap_or_default();
        json.push('\n');
        json
    }
}

/// The parts of a schema-version-1 manifest an upgrade needs.
///
/// Deliberately permissive: unknown version-1 fields pass through unread so
/// the upgrader can migrate any instance the previous distribution produced.
#[derive(Debug, Clone, Deserialize)]
pub struct LegacyManifest {
    /// The recorded schema version; the upgrader requires `1`.
    pub schema_version: u32,
    /// The canon release the instance was installed from.
    pub canon_version: CanonVersion,
    /// The profile the instance was installed with.
    pub profile: ProfileId,
    /// The documentation root the gates read.
    pub docs_root: DocsRoot,
    /// When the instance was first installed.
    pub installed_at: String,
    /// Destination and hash of every file version 1 managed.
    pub managed_files: Vec<LegacyOwnedFile>,
}

/// One version-1 managed entry: only what the conflict scan reads.
#[derive(Debug, Clone, Deserialize)]
pub struct LegacyOwnedFile {
    /// Where the instance holds the file, relative to its root.
    pub destination: Utf8PathBuf,
    /// The bytes version 1 recorded for it.
    pub sha256: crate::domain::ownership::Sha256,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ownership::Sha256;

    fn sample() -> Manifest {
        Manifest {
            schema_version: SCHEMA_VERSION,
            canon_version: "0.2.0".parse().unwrap(),
            canon_source: CANON_SOURCE.to_string(),
            profile: ProfileId::KnowledgeBase,
            docs_root: DocsRoot::UnderscoreDocs,
            installed_at: "2026-08-25T00:00:00Z".to_string(),
            managed_files: vec![ManagedEntry {
                source: ".markdownlint/spec.markdownlint-cli2.jsonc".into(),
                destination: ".spec-driven-docs/markdownlint/spec.markdownlint-cli2.jsonc".into(),
                sha256: Sha256::of(b"x"),
            }],
            adopted_files: vec![],
            integration_blocks: vec![],
        }
    }

    #[test]
    fn round_trips_through_json() {
        let manifest = sample();
        let json = manifest.to_json();
        assert!(json.ends_with('\n'));
        assert_eq!(Manifest::parse(&json).unwrap(), manifest);
    }

    #[test]
    fn rejects_an_older_schema_as_upgradable() {
        let mut value: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        value["schema_version"] = 1.into();
        assert!(matches!(
            Manifest::parse(&value.to_string()),
            Err(ManifestParseError::Older(1))
        ));
    }

    #[test]
    fn rejects_a_newer_schema_as_binary_too_old() {
        let mut value: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        value["schema_version"] = 3.into();
        assert!(matches!(
            Manifest::parse(&value.to_string()),
            Err(ManifestParseError::Newer(3))
        ));
    }

    #[test]
    fn rejects_unknown_fields_and_empty_managed_sets() {
        let mut value: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        value["canon_ref"] = "v0.2.0".into();
        assert!(matches!(
            Manifest::parse(&value.to_string()),
            Err(ManifestParseError::Invalid(_))
        ));

        let mut value: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        value["managed_files"] = serde_json::Value::Array(vec![]);
        assert!(matches!(
            Manifest::parse(&value.to_string()),
            Err(ManifestParseError::Invalid(_))
        ));
    }

    #[test]
    fn legacy_manifest_reads_a_version_one_shape() {
        let json = r#"{
            "schema_version": 1,
            "canon_version": "0.1.6",
            "canon_source": "https://github.com/gubasso/spec-driven-docs",
            "canon_ref": "pre-release",
            "profile": "knowledge-base",
            "docs_root": "_docs",
            "installed_at": "2026-08-24T00:00:00Z",
            "managed_files": [
                {"source": "scripts/verify.sh", "destination": ".spec-driven-docs/verify.sh",
                 "sha256": "dc17d596ae2c196cc01b439c291416f91198cc274e2376fd01a4d614c1ff60ad"}
            ],
            "adopted_files": [],
            "integration_blocks": []
        }"#;
        let legacy: LegacyManifest = serde_json::from_str(json).unwrap();
        assert_eq!(legacy.schema_version, 1);
        assert_eq!(legacy.canon_version.to_string(), "0.1.6");
        assert_eq!(legacy.managed_files.len(), 1);
    }
}
