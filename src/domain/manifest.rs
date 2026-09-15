//! The instance manifest: the persistent record of what an instance holds.
//!
//! Schema version 3. The manifest is what lets a later `sdd` distinguish
//! managed drift from adopted reconciliation and its own version from the
//! instance's. This module owns the shape and its parse-time invariants;
//! reading it from disk, comparing it to bytes, and writing it belong to
//! the services.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::ownership::{AdoptedEntry, IntegrationBlock, ManagedEntry};
use crate::domain::profile::{DocsRoot, ProfileId};
use crate::domain::version::CanonVersion;

/// The manifest schema this binary reads and writes.
pub const SCHEMA_VERSION: u32 = 3;
/// Where the canon is published.
pub const CANON_SOURCE: &str = "https://github.com/gubasso/spec-driven-docs";
// Every control path this tool names is declared in `domain::paths`, and
// reaches its callers from here so that the names they already import stay
// where they were.
pub use crate::domain::paths::{DOCS_SCRATCH_VAR, INSTANCE_DIR, MANIFEST_PATH};

/// Fields an earlier release of this schema version wrote and this one
/// does not read.
///
/// A record carrying one parses as if the field were absent, and the next
/// write omits it, so an instance in the field crosses the removal without
/// an operator edit and without a schema bump.
pub const RETIRED_FIELDS: &[&str] = &["plan_zone"];

/// A `--docs-scratch` value the argument cannot mean.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("{0}")]
pub struct DeclaredPathError(String);

/// A declared path, normalized: relative, non-empty, and `./` stripped.
///
/// `parents` says whether the path may leave the instance root. A docs
/// scratch may, because staging beside the checkout is one of the offered
/// answers.
fn declared_path(value: &str, parents: bool) -> Result<Utf8PathBuf, DeclaredPathError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(DeclaredPathError("the path is empty".to_string()));
    }
    let path = Utf8Path::new(value);
    if path.is_absolute() {
        return Err(DeclaredPathError(format!("{value} is not relative")));
    }
    let mut normalized = Utf8PathBuf::new();
    for component in path.components() {
        match component {
            camino::Utf8Component::CurDir => {}
            camino::Utf8Component::ParentDir if parents => normalized.push(".."),
            camino::Utf8Component::ParentDir => {
                return Err(DeclaredPathError(format!("{value} leaves the repository")));
            }
            other => normalized.push(other.as_str()),
        }
    }
    if normalized.as_str().is_empty() {
        return Err(DeclaredPathError(format!("{value} names no directory")));
    }
    Ok(normalized)
}

/// Whether a recorded docs-scratch path is one a reader can resolve.
///
/// The same check the argument runs. A record reaches a reinstall through a
/// permissive read, so without this a hand-edited value is carried forward
/// and only the post-write verification catches it.
///
/// # Errors
///
/// [`DeclaredPathError`] for a path that is empty or absolute.
pub fn validate_docs_scratch_path(path: &Utf8Path) -> Result<(), DeclaredPathError> {
    declared_path(path.as_str(), true).map(|_| ())
}

/// Read the `--docs-scratch` argument.
///
/// `none` clears a recorded value. Without a clearing word an operator who
/// declares a scratch by typo can change it but never return to the
/// undeclared state.
///
/// # Errors
///
/// [`DeclaredPathError`] for a path that is empty or absolute.
pub fn parse_docs_scratch(value: &str) -> Result<Option<Utf8PathBuf>, DeclaredPathError> {
    if value.trim() == "none" {
        return Ok(None);
    }
    declared_path(value, true).map(Some)
}

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
    /// Where material that is not a statement yet is kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_scratch: Option<Utf8PathBuf>,
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
        let mut value: serde_json::Value =
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
        // A field an earlier release of this schema wrote is dropped on
        // read, so the record parses as it would have without it.
        if let Some(object) = value.as_object_mut() {
            for field in RETIRED_FIELDS {
                object.remove(*field);
            }
        }
        let manifest: Self = serde_json::from_value(value)
            .map_err(|e| ManifestParseError::Invalid(e.to_string()))?;
        if manifest.managed_files.is_empty() {
            return Err(ManifestParseError::Invalid(
                "managed_files is empty".to_string(),
            ));
        }
        // The declared location carries the same invariant the argument
        // enforces. Without this the record is the weaker gate: a
        // hand-edited absolute path makes a reader resolve outside the
        // repository.
        if let Some(path) = &manifest.docs_scratch
            && let Err(error) = validate_docs_scratch_path(path)
        {
            return Err(ManifestParseError::Invalid(format!(
                "docs_scratch: {error}"
            )));
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

/// The parts of an older manifest an upgrade needs.
///
/// Deliberately permissive: unknown fields of an older version pass through
/// unread, so the upgrader can migrate any instance a previous distribution
/// produced.
#[derive(Debug, Clone, Deserialize)]
pub struct LegacyManifest {
    /// The recorded schema version; the upgrader requires an older one.
    pub schema_version: u32,
    /// The canon release the instance was installed from.
    pub canon_version: CanonVersion,
    /// The profile the instance was installed with.
    pub profile: ProfileId,
    /// The documentation root the gates read.
    pub docs_root: DocsRoot,
    /// When the instance was first installed.
    pub installed_at: String,
    /// Destination and hash of every file the older version managed.
    pub managed_files: Vec<LegacyOwnedFile>,
    /// The marked regions the older version owned inside project files.
    ///
    /// Version 1 recorded none, so the default carries that case. Dropping
    /// the field instead would skip the edited-block conflict check on
    /// every version-2 upgrade and overwrite the operator's edits.
    #[serde(default)]
    pub integration_blocks: Vec<IntegrationBlock>,
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
            docs_scratch: Some("scratch".into()),
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
        value["schema_version"] = 4.into();
        assert!(matches!(
            Manifest::parse(&value.to_string()),
            Err(ManifestParseError::Newer(4))
        ));
    }

    /// A record written before the docs scratch was declared reads as the
    /// project declaring none, rather than as a broken manifest.
    #[test]
    fn a_record_without_the_declared_location_defaults_it() {
        let mut value: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        value.as_object_mut().unwrap().remove("docs_scratch");
        let manifest = Manifest::parse(&value.to_string()).unwrap();
        assert_eq!(manifest.docs_scratch, None);
    }

    /// A field an earlier release of this schema wrote parses as absent,
    /// whatever shape it carries, and the next write omits it.
    #[test]
    fn a_retired_field_is_dropped_on_read_and_omitted_on_write() {
        for retired in RETIRED_FIELDS {
            let mut value: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
            value[*retired] = serde_json::json!({"kind": "tracked", "path": "docs/plan"});
            let manifest = Manifest::parse(&value.to_string()).unwrap();
            assert_eq!(manifest, sample());
            assert!(!manifest.to_json().contains(retired));
        }
    }

    #[test]
    fn a_docs_scratch_may_leave_the_repository() {
        assert_eq!(
            parse_docs_scratch("../beside-the-checkout").unwrap(),
            Some(Utf8PathBuf::from("../beside-the-checkout"))
        );
        assert!(parse_docs_scratch("/tmp/scratch").is_err());
        assert!(parse_docs_scratch("").is_err());
    }

    /// The declared value has a clearing word, so a typo can be undone.
    #[test]
    fn the_declared_location_can_be_cleared() {
        assert_eq!(parse_docs_scratch("none").unwrap(), None);
        // And a directory literally named `none` is still reachable.
        assert_eq!(
            parse_docs_scratch("./none").unwrap(),
            Some(Utf8PathBuf::from("none"))
        );
    }

    /// The record carries the same invariant the argument enforces.
    #[test]
    fn a_recorded_location_the_argument_would_refuse_is_invalid() {
        let mut value: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        value["docs_scratch"] = "/tmp/scratch".into();
        assert!(matches!(
            Manifest::parse(&value.to_string()),
            Err(ManifestParseError::Invalid(_))
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
        assert!(legacy.integration_blocks.is_empty());
    }

    /// A record of the shape version 2 actually wrote — the current one
    /// without `docs_scratch`, which version 2 had no field for — parses,
    /// and its integration blocks reach the upgrade. Without them the
    /// edited-block conflict check is skipped on every version-2 hop.
    #[test]
    fn legacy_manifest_reads_the_version_two_shape() {
        let mut value: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        value["schema_version"] = 2.into();
        let object = value.as_object_mut().unwrap();
        object.remove("docs_scratch");
        value["integration_blocks"] = serde_json::json!([{
            "path": ".pre-commit-config.yaml",
            "marker_hash": Sha256::of(b"block").to_string(),
        }]);
        let legacy: LegacyManifest = serde_json::from_str(&value.to_string()).unwrap();
        assert_eq!(legacy.integration_blocks.len(), 1);
        assert_eq!(legacy.integration_blocks[0].path, ".pre-commit-config.yaml");
    }
}
