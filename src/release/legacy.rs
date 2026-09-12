//! Releases that predate the projection declaration.
//!
//! A published crate is immutable, so a release that shipped before the
//! declaration existed can never acquire one. Either the engine describes
//! those releases from outside, or every plan toward one is a guess. This
//! module is that description: a finite catalog, audited against the
//! registry index and the published archives, and closed the day schema 1
//! ships.
//!
//! Nothing here infers. A release below the capability floor, or one the
//! catalog does not name, is unavailable as a destination and says so with
//! the evidence. The current projection is never applied retroactively: a
//! descriptor states what that release landed, and the adapter overlays it
//! as metadata beside the archive's own bytes.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::domain::ownership::Sha256;
use crate::domain::projection::{DECLARATION_PATH, Declaration};
use crate::error::AppError;

/// The catalog index this binary carries.
pub static INDEX_TOML: &str = include_str!("../../release-compat/index.toml");

/// Every descriptor this binary carries, by version.
static DESCRIPTORS: &[(&str, &str)] = &[
    (
        "0.6.6",
        include_str!("../../release-compat/legacy/0.6.6.toml"),
    ),
    (
        "0.7.0",
        include_str!("../../release-compat/legacy/0.7.0.toml"),
    ),
    (
        "0.7.1",
        include_str!("../../release-compat/legacy/0.7.1.toml"),
    ),
    (
        "0.7.2",
        include_str!("../../release-compat/legacy/0.7.2.toml"),
    ),
    (
        "0.8.0",
        include_str!("../../release-compat/legacy/0.8.0.toml"),
    ),
    (
        "0.8.1",
        include_str!("../../release-compat/legacy/0.8.1.toml"),
    ),
];

/// One release, as the audit classified it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    /// The published version.
    pub version: String,
    /// The registry's checksum for its archive.
    pub checksum: String,
    /// Whether this engine can read it as a destination.
    pub eligible: bool,
    /// Why it cannot, where it cannot.
    #[serde(default)]
    pub reason: Option<String>,
    /// The descriptor file that supplies its facts.
    #[serde(default)]
    pub descriptor: Option<String>,
    /// That file's digest, so a correction invalidates an outstanding plan.
    #[serde(default)]
    pub descriptor_sha256: Option<String>,
}

/// The whole catalog.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogIndex {
    /// The catalog's own schema.
    pub schema: u32,
    /// The date the registry and the archives were read.
    pub audited_on: String,
    /// The lowest release this engine can read.
    pub capability_floor: String,
    /// Every published release, in registry order.
    pub releases: Vec<CatalogEntry>,
}

/// What a descriptor states about one pre-schema release.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    /// The release the descriptor describes.
    pub version: String,
    /// The payload roots that release's archive carries.
    pub payload_roots: Vec<String>,
    /// What it landed, written in the schema this engine reads.
    pub projection: Declaration,
}

/// What the adapter overlays onto a pre-schema archive.
#[derive(Debug, Clone)]
pub struct Adapted {
    /// The schema the release itself declared, which is none.
    pub payload_schema: u32,
    /// The descriptor that supplied the facts.
    pub descriptor_sha256: Sha256,
    /// The virtual metadata the bundle serves beside its payload.
    pub metadata: BTreeMap<String, Vec<u8>>,
}

/// The pre-schema protocol version, which no release declared.
pub const PRE_SCHEMA: u32 = 0;

/// The audited catalog of pre-schema releases.
#[derive(Debug, Clone)]
pub struct LegacyCatalog {
    index: CatalogIndex,
    descriptors: BTreeMap<String, String>,
}

impl LegacyCatalog {
    /// The catalog this binary carries.
    ///
    /// A catalog that does not parse is a build defect rather than a state
    /// a command can meet, and the canon suite parses the same bytes.
    ///
    /// # Panics
    ///
    /// Never in a shipped build, for the reason above.
    #[expect(
        clippy::expect_used,
        reason = "the catalog is compiled in; a parse failure is a build defect the canon suite catches first"
    )]
    #[must_use]
    pub fn embedded() -> Self {
        Self {
            index: toml::from_str(INDEX_TOML).expect("the embedded release catalog parses"),
            descriptors: DESCRIPTORS
                .iter()
                .map(|(version, text)| ((*version).to_string(), (*text).to_string()))
                .collect(),
        }
    }

    /// What the catalog says about one version.
    #[must_use]
    pub fn entry(&self, version: &str) -> Option<&CatalogEntry> {
        self.index
            .releases
            .iter()
            .find(|entry| entry.version == version)
    }

    /// The index itself, for the canon suite and the diagnostics.
    #[must_use]
    pub const fn index(&self) -> &CatalogIndex {
        &self.index
    }

    /// Every descriptor, by version.
    #[must_use]
    pub const fn descriptors(&self) -> &BTreeMap<String, String> {
        &self.descriptors
    }

    /// The descriptor one version resolves to, checked against the index.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`] when the version is unavailable, uncataloged,
    /// or whose descriptor no longer matches the digest the index records.
    pub fn descriptor(&self, version: &str) -> Result<(Descriptor, Sha256), AppError> {
        let entry = self.entry(version).ok_or_else(|| {
            AppError::Refused(format!(
                "{version} is not in the release catalog, and it declares no projection of its own, so this engine cannot say what it lands"
            ))
        })?;
        if !entry.eligible {
            let reason = entry.reason.as_deref().unwrap_or("it was not audited");
            return Err(AppError::Refused(format!(
                "{version} is unavailable as a destination: {reason}. The lowest release this engine reads is {}",
                self.index.capability_floor
            )));
        }
        let name = entry.descriptor.as_deref().ok_or_else(|| {
            AppError::Refused(format!("{version} is eligible and names no descriptor"))
        })?;
        let text = self
            .descriptors
            .get(version)
            .ok_or_else(|| AppError::Refused(format!("this engine does not carry {name}")))?;
        let digest = Sha256::of(text.as_bytes());
        let recorded = entry.descriptor_sha256.as_deref().unwrap_or_default();
        if digest.as_str() != recorded {
            return Err(AppError::Refused(format!(
                "the descriptor for {version} hashes to {digest} and the catalog records {recorded}"
            )));
        }
        let held: Descriptor = toml::from_str(text)
            .map_err(|source| AppError::Refused(format!("{name} does not parse: {source}")))?;
        if held.version != version {
            return Err(AppError::Refused(format!(
                "{name} describes {} and the catalog files it under {version}",
                held.version
            )));
        }
        Ok((held, digest))
    }

    /// Overlay a descriptor's facts onto one pre-schema archive.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`] when the version is not cataloged, when the
    /// descriptor disagrees with the archive it describes, or when the
    /// overlay cannot be rendered.
    pub fn adapt(
        &self,
        version: &str,
        files: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Adapted, AppError> {
        let (descriptor, descriptor_sha256) = self.descriptor(version)?;
        for root in &descriptor.payload_roots {
            let prefix = format!("{root}/");
            if !files.keys().any(|path| path.starts_with(&prefix)) {
                return Err(AppError::Refused(format!(
                    "the descriptor for {version} names the root {root}, and the published archive carries no file under it"
                )));
            }
        }
        for entry in descriptor
            .projection
            .managed
            .iter()
            .chain(&descriptor.projection.adopted)
        {
            if !files.contains_key(&entry.source) {
                return Err(AppError::Refused(format!(
                    "the descriptor for {version} projects {}, and the published archive does not carry it",
                    entry.source
                )));
            }
        }
        let rendered = toml::to_string_pretty(&descriptor.projection).map_err(|source| {
            AppError::Refused(format!(
                "the descriptor for {version} did not render as a declaration: {source}"
            ))
        })?;
        Ok(Adapted {
            payload_schema: PRE_SCHEMA,
            descriptor_sha256,
            metadata: BTreeMap::from([(DECLARATION_PATH.to_string(), rendered.into_bytes())]),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    #[test]
    fn every_eligible_release_has_one_descriptor_and_no_other_does() {
        let catalog = LegacyCatalog::embedded();
        for entry in &catalog.index().releases {
            if entry.eligible {
                assert!(
                    entry.descriptor.is_some(),
                    "{} names no descriptor",
                    entry.version
                );
                assert!(
                    catalog.descriptors().contains_key(&entry.version),
                    "{} is eligible and not carried",
                    entry.version
                );
            } else {
                assert!(
                    entry.descriptor.is_none(),
                    "{} is ineligible and named one",
                    entry.version
                );
                assert!(
                    !catalog.descriptors().contains_key(&entry.version),
                    "{} is ineligible and carried",
                    entry.version
                );
                assert!(
                    entry.reason.is_some(),
                    "{} refuses without a reason",
                    entry.version
                );
            }
        }
        assert_eq!(
            catalog.descriptors().len(),
            catalog
                .index()
                .releases
                .iter()
                .filter(|e| e.eligible)
                .count()
        );
    }

    #[test]
    fn no_version_appears_twice() {
        let catalog = LegacyCatalog::embedded();
        let mut seen: Vec<&str> = Vec::new();
        for entry in &catalog.index().releases {
            assert!(
                !seen.contains(&entry.version.as_str()),
                "{} twice",
                entry.version
            );
            seen.push(&entry.version);
        }
    }

    #[test]
    fn release_0_6_5_is_unavailable_and_0_6_6_is_the_capability_floor() {
        let catalog = LegacyCatalog::embedded();
        assert_eq!(catalog.index().capability_floor, "0.6.6");
        let error = catalog.descriptor("0.6.5").unwrap_err();
        assert!(error.to_string().contains("unavailable"), "{error}");
        assert!(error.to_string().contains("instance/seeds"), "{error}");
        assert!(catalog.descriptor("0.6.6").is_ok());
    }

    #[test]
    fn an_uncataloged_version_is_never_a_candidate() {
        let error = LegacyCatalog::embedded().descriptor("9.9.9").unwrap_err();
        assert!(
            error.to_string().contains("not in the release catalog"),
            "{error}"
        );
    }

    #[test]
    fn every_descriptor_matches_the_digest_the_index_records() {
        let catalog = LegacyCatalog::embedded();
        for (version, text) in catalog.descriptors() {
            let entry = catalog.entry(version).unwrap();
            assert_eq!(
                entry.descriptor_sha256.as_deref(),
                Some(Sha256::of(text.as_bytes()).as_str()),
                "{version} descriptor digest drifted"
            );
        }
    }

    #[test]
    fn an_overlay_needs_the_archive_to_carry_what_it_projects() {
        let catalog = LegacyCatalog::embedded();
        let error = catalog.adapt("0.6.6", &BTreeMap::new()).unwrap_err();
        assert!(
            error.to_string().contains("carries no file under it"),
            "{error}"
        );
    }

    #[test]
    fn an_overlay_renders_a_declaration_this_engine_reads() {
        let catalog = LegacyCatalog::embedded();
        let (descriptor, _) = catalog.descriptor("0.8.0").unwrap();
        // Stand in for the archive: every projected source and one file
        // under every declared root.
        let mut files: BTreeMap<String, Vec<u8>> = descriptor
            .projection
            .managed
            .iter()
            .chain(&descriptor.projection.adopted)
            .map(|entry| (entry.source.clone(), b"x".to_vec()))
            .collect();
        for root in &descriptor.payload_roots {
            files.insert(format!("{root}/present.md"), b"x".to_vec());
        }
        let adapted = catalog.adapt("0.8.0", &files).unwrap();
        assert_eq!(adapted.payload_schema, PRE_SCHEMA);
        let rendered = adapted.metadata.get(DECLARATION_PATH).unwrap();
        let read = Declaration::parse(rendered).unwrap();
        assert_eq!(read, descriptor.projection);
    }
}
