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
/// The instance directory, relative to the instance root.
pub const INSTANCE_DIR: &str = ".spec-driven-docs";
/// The manifest path, relative to the instance root.
pub const MANIFEST_PATH: &str = ".spec-driven-docs/manifest.json";

/// The environment variable that names the plan zone.
pub const PLAN_ZONE_VAR: &str = "SDD_PLAN_ZONE";
/// The environment variable that names the docs scratch.
pub const DOCS_SCRATCH_VAR: &str = "SDD_DOCS_SCRATCH";

/// Where the project's planning tool writes its entry documents.
///
/// The kind is part of the value because the three absent cases are not one
/// case. A tracked zone is a directory every clone has, so a command may
/// check it and an absent directory is drift. An untracked zone and a zone
/// reached through [`PLAN_ZONE_VAR`] are absent on a fresh clone, so the
/// same failure there would report a layout the project never promised.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PlanZone {
    /// A repository-relative directory under version control.
    Tracked {
        /// Where the zone sits, relative to the instance root.
        path: Utf8PathBuf,
    },
    /// A repository-relative directory version control does not carry.
    Untracked {
        /// Where the zone sits, relative to the instance root.
        path: Utf8PathBuf,
    },
    /// Wherever [`PLAN_ZONE_VAR`] resolves at run time.
    Env,
    /// The project keeps no plan zone.
    #[default]
    None,
}

/// A `--plan-zone` or `--docs-scratch` value the arguments cannot mean.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("{0}")]
pub struct DeclaredPathError(String);

/// A declared path, normalized: relative, non-empty, and `./` stripped.
///
/// `parents` says whether the path may leave the instance root. A plan zone
/// may not, because a gate resolves it against that root. A docs scratch
/// may, because staging beside the checkout is one of the offered answers.
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

impl PlanZone {
    /// Read the `--plan-zone` argument's four forms.
    ///
    /// `none` and `env` are words rather than paths, so a directory with
    /// either name is written `./none` or `./env`.
    ///
    /// # Errors
    ///
    /// [`DeclaredPathError`] for a path that is empty, absolute, or leaves
    /// the repository.
    pub fn parse(value: &str) -> Result<Self, DeclaredPathError> {
        match value.trim() {
            "none" => Ok(Self::None),
            "env" => Ok(Self::Env),
            rest => match rest.strip_prefix("untracked:") {
                Some(path) => Ok(Self::Untracked {
                    path: declared_path(path, false)?,
                }),
                None => Ok(Self::Tracked {
                    path: declared_path(rest, false)?,
                }),
            },
        }
    }
}

/// Read the `--docs-scratch` argument.
///
/// # Errors
///
/// [`DeclaredPathError`] for a path that is empty or absolute.
pub fn parse_docs_scratch(value: &str) -> Result<Utf8PathBuf, DeclaredPathError> {
    declared_path(value, true)
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
    /// Where the planning tool writes entry documents.
    #[serde(default)]
    pub plan_zone: PlanZone,
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
            plan_zone: PlanZone::Tracked {
                path: "tests/fixtures".into(),
            },
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

    /// A record written before the two locations were declared reads as the
    /// project declaring neither, rather than as a broken manifest.
    #[test]
    fn a_record_without_the_declared_locations_defaults_them() {
        let mut value: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        value.as_object_mut().unwrap().remove("plan_zone");
        value.as_object_mut().unwrap().remove("docs_scratch");
        let manifest = Manifest::parse(&value.to_string()).unwrap();
        assert_eq!(manifest.plan_zone, PlanZone::None);
        assert_eq!(manifest.docs_scratch, None);
    }

    #[test]
    fn the_plan_zone_round_trips_through_its_tagged_form() {
        let manifest = sample();
        let json = manifest.to_json();
        assert!(json.contains("\"kind\": \"tracked\""));
        assert_eq!(Manifest::parse(&json).unwrap(), manifest);

        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value["plan_zone"] = serde_json::json!({"kind": "none"});
        assert_eq!(
            Manifest::parse(&value.to_string()).unwrap().plan_zone,
            PlanZone::None
        );
    }

    #[test]
    fn the_plan_zone_argument_takes_four_forms() {
        assert_eq!(PlanZone::parse("none").unwrap(), PlanZone::None);
        assert_eq!(PlanZone::parse("env").unwrap(), PlanZone::Env);
        assert_eq!(
            PlanZone::parse("docs/plan").unwrap(),
            PlanZone::Tracked {
                path: "docs/plan".into()
            }
        );
        assert_eq!(
            PlanZone::parse("untracked:docs/plan").unwrap(),
            PlanZone::Untracked {
                path: "docs/plan".into()
            }
        );
        // A directory carrying one of the two words is written as a path.
        assert_eq!(
            PlanZone::parse("./none").unwrap(),
            PlanZone::Tracked {
                path: "none".into()
            }
        );
    }

    #[test]
    fn a_plan_zone_never_leaves_the_repository_and_a_docs_scratch_may() {
        assert!(PlanZone::parse("/etc/plan").is_err());
        assert!(PlanZone::parse("../plan").is_err());
        assert!(PlanZone::parse("untracked:../plan").is_err());
        assert!(PlanZone::parse("  ").is_err());

        assert_eq!(
            parse_docs_scratch("../beside-the-checkout").unwrap(),
            Utf8PathBuf::from("../beside-the-checkout")
        );
        assert!(parse_docs_scratch("/tmp/scratch").is_err());
        assert!(parse_docs_scratch("").is_err());
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

    /// A version-2 record carries integration blocks, and the upgrade path
    /// reads them: without them the edited-block conflict check is skipped.
    #[test]
    fn legacy_manifest_carries_a_version_two_integration_block() {
        let mut value: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        value["schema_version"] = 2.into();
        value["integration_blocks"] = serde_json::json!([{
            "path": ".pre-commit-config.yaml",
            "marker_hash": Sha256::of(b"block").to_string(),
        }]);
        let legacy: LegacyManifest = serde_json::from_str(&value.to_string()).unwrap();
        assert_eq!(legacy.integration_blocks.len(), 1);
        assert_eq!(legacy.integration_blocks[0].path, ".pre-commit-config.yaml");
    }
}
