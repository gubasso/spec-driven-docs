//! The release boundary: what a release is, and how one is obtained.
//!
//! Two small interfaces sit here and nothing else. A [`ReleaseBundle`]
//! answers with a manifest and with blobs by digest, and it does not say
//! where the bytes came from. A [`ReleaseResolver`] turns a selector into
//! exactly one verified bundle, once.
//!
//! They are separate on purpose. Resolution reaches the network and freezes
//! an identity; content access never does. An apply that could resolve a
//! second time could apply a release the plan never described.
//!
//! Parsing what a bundle carries is the domain's work, not this layer's.
//! The boundary hands over bytes; [`crate::domain::projection`] says what
//! the projection declaration in them means.

pub mod crates_io;
pub mod embedded;
pub mod fixture;
pub mod legacy;

use std::collections::BTreeMap;

use serde::Serialize;

use crate::domain::ownership::Sha256;
use crate::domain::projection::{DECLARATION_PATH, Declaration};
use crate::domain::version::CanonVersion;
use crate::error::AppError;

/// The version type the registry speaks, which admits a prerelease where
/// [`CanonVersion`] admits only a released triple.
pub use semver::Version;

/// The machine schema `sdd payload --json` declares.
///
/// Independent of the status schema, the installed-record schema, and the
/// payload schema. They version different things and move on their own
/// dates, and one number for four of them would tie every reader to every
/// change.
pub const MANIFEST_SCHEMA: &str = "sdd.payload/1";

/// Where a release's facts came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provenance {
    /// The release declares itself.
    Native,
    /// The release predates the declaration, and an audited catalog entry
    /// supplies its projection facts.
    LegacyAdapted,
}

/// What one artifact is to the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// A file the release can project into a target.
    Payload,
    /// A fact about the release that no target receives.
    Metadata,
}

/// One file a bundle carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Artifact {
    /// The logical path, as the projection names it.
    pub path: String,
    /// Whether a target can receive it.
    pub role: Role,
    /// How many bytes it is.
    pub bytes: u64,
    /// Its digest, which is also its key in the blob store.
    pub sha256: Sha256,
}

/// What a bundle says about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReleaseManifest {
    /// The machine schema of this object.
    pub schema: &'static str,
    /// The release this bundle is.
    pub version: Version,
    /// The protocol version between the engine and this bundle.
    pub payload_schema: u32,
    /// Where the release's facts came from.
    pub provenance: Provenance,
    /// The catalog descriptor used, where one was.
    pub descriptor_sha256: Option<Sha256>,
    /// A digest over every artifact's path and digest, in path order.
    pub payload_sha256: Sha256,
    /// Every file the bundle carries, in path order.
    pub artifacts: Vec<Artifact>,
}

impl ReleaseManifest {
    /// The digest one logical path resolves to.
    #[must_use]
    pub fn digest_of(&self, path: &str) -> Option<&Sha256> {
        self.artifacts
            .iter()
            .find(|artifact| artifact.path == path)
            .map(|artifact| &artifact.sha256)
    }

    /// A digest over every artifact's path and digest, in path order.
    ///
    /// Identity over content rather than over the archive: two bundles that
    /// carry the same files are the same release to the planner, whichever
    /// transport served them.
    #[must_use]
    pub fn payload_digest(artifacts: &[Artifact]) -> Sha256 {
        let mut joined = String::new();
        for artifact in artifacts {
            joined.push_str(&artifact.path);
            joined.push('\0');
            joined.push_str(artifact.sha256.as_str());
            joined.push('\n');
        }
        Sha256::of(joined.as_bytes())
    }
}

/// One release's bytes, however they were obtained.
pub trait ReleaseBundle: Send + Sync {
    /// What this bundle says about itself.
    ///
    /// # Errors
    ///
    /// [`AppError`] when the bundle cannot describe itself.
    fn manifest(&self) -> Result<ReleaseManifest, AppError>;

    /// One artifact's bytes, by digest.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`] when the bundle carries no such digest.
    fn blob(&self, digest: &Sha256) -> Result<Vec<u8>, AppError>;

    /// One artifact's bytes, by the path the manifest names.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`] when the bundle carries no such path.
    fn artifact(&self, path: &str) -> Result<Vec<u8>, AppError> {
        let manifest = self.manifest()?;
        let digest = manifest.digest_of(path).ok_or_else(|| {
            AppError::Refused(format!("release {} carries no {path}", manifest.version))
        })?;
        self.blob(digest)
    }

    /// What this release declares it lands.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`] when the bundle carries no declaration or the
    /// declaration is one this engine does not decode.
    fn declaration(&self) -> Result<Declaration, AppError> {
        let bytes = self.artifact(DECLARATION_PATH)?;
        Declaration::parse(&bytes).map_err(|source| AppError::Refused(source.to_string()))
    }
}

/// Which release a caller asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// The release this binary carries.
    Embedded,
    /// The highest stable release the registry serves.
    Latest,
    /// Exactly this version.
    Exact(Version),
}

impl std::fmt::Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Embedded => f.write_str("embedded"),
            Self::Latest => f.write_str("latest"),
            Self::Exact(version) => write!(f, "{version}"),
        }
    }
}

/// One selector, frozen to one verified bundle.
pub struct ResolvedRelease {
    /// What the caller asked for.
    pub selector: Selector,
    /// The exact release the selector resolved to.
    pub version: Version,
    /// The registry's checksum for the archive, where a registry served it.
    pub registry_checksum: Option<Sha256>,
    /// The digest over the bundle's content.
    pub payload_sha256: Sha256,
    /// Whether the registry marks this release yanked.
    pub yanked: bool,
    /// The bundle itself.
    pub bundle: Box<dyn ReleaseBundle>,
}

impl std::fmt::Debug for ResolvedRelease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedRelease")
            .field("selector", &self.selector)
            .field("version", &self.version)
            .field("registry_checksum", &self.registry_checksum)
            .field("payload_sha256", &self.payload_sha256)
            .field("yanked", &self.yanked)
            .finish_non_exhaustive()
    }
}

impl ResolvedRelease {
    /// The resolved version as the installed record spells it.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`] where the release is a prerelease or carries
    /// build metadata. A record holds a released triple, so a plan toward
    /// anything else has nothing to write into one.
    pub fn canon_version(&self) -> Result<CanonVersion, AppError> {
        self.version.to_string().parse().map_err(|_| {
            AppError::Refused(format!(
                "{} is not a released triple, so no instance record can name it",
                self.version
            ))
        })
    }
}

/// Turn a selector into exactly one verified bundle.
pub trait ReleaseResolver {
    /// Resolve once.
    ///
    /// # Errors
    ///
    /// [`AppError`] when the selector cannot be resolved, when the bundle
    /// cannot be verified, or when the network is needed and refused.
    fn resolve(&self, selector: &Selector) -> Result<ResolvedRelease, AppError>;
}

/// Build a manifest from a set of logical paths and their bytes.
///
/// Shared by every bundle, so identity is computed one way whichever
/// transport served the bytes.
#[must_use]
pub fn manifest_from(
    version: Version,
    payload_schema: u32,
    provenance: Provenance,
    descriptor_sha256: Option<Sha256>,
    files: &BTreeMap<String, Vec<u8>>,
    metadata: &BTreeMap<String, Vec<u8>>,
) -> ReleaseManifest {
    let mut artifacts: Vec<Artifact> = files
        .iter()
        .map(|(path, bytes)| artifact_of(path, bytes, Role::Payload))
        .chain(
            metadata
                .iter()
                .map(|(path, bytes)| artifact_of(path, bytes, Role::Metadata)),
        )
        .collect();
    artifacts.sort_by(|left, right| left.path.cmp(&right.path));
    ReleaseManifest {
        schema: MANIFEST_SCHEMA,
        version,
        payload_schema,
        provenance,
        descriptor_sha256,
        payload_sha256: ReleaseManifest::payload_digest(&artifacts),
        artifacts,
    }
}

fn artifact_of(path: &str, bytes: &[u8], role: Role) -> Artifact {
    Artifact {
        path: path.to_string(),
        role,
        bytes: bytes.len() as u64,
        sha256: Sha256::of(bytes),
    }
}

/// Serve a blob out of a byte map keyed by path.
///
/// # Errors
///
/// [`AppError::Refused`] when no artifact carries that digest.
pub fn blob_from(
    files: &BTreeMap<String, Vec<u8>>,
    metadata: &BTreeMap<String, Vec<u8>>,
    digest: &Sha256,
) -> Result<Vec<u8>, AppError> {
    files
        .values()
        .chain(metadata.values())
        .find(|bytes| &Sha256::of(bytes) == digest)
        .cloned()
        .ok_or_else(|| AppError::Refused(format!("the bundle carries no blob {digest}")))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn current() -> Version {
        CanonVersion::current().to_string().parse().unwrap()
    }

    fn files() -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([
            ("b.md".to_string(), b"two".to_vec()),
            ("a.md".to_string(), b"one".to_vec()),
        ])
    }

    #[test]
    fn a_manifest_lists_every_artifact_in_path_order() {
        let manifest = manifest_from(
            current(),
            1,
            Provenance::Native,
            None,
            &files(),
            &BTreeMap::new(),
        );
        let paths: Vec<&str> = manifest
            .artifacts
            .iter()
            .map(|artifact| artifact.path.as_str())
            .collect();
        assert_eq!(paths, ["a.md", "b.md"]);
        assert_eq!(manifest.schema, MANIFEST_SCHEMA);
        assert_eq!(manifest.digest_of("a.md"), Some(&Sha256::of(b"one")));
        assert_eq!(manifest.digest_of("absent.md"), None);
    }

    #[test]
    fn the_payload_digest_reads_the_content_and_not_the_order_it_was_given() {
        let one = manifest_from(
            current(),
            1,
            Provenance::Native,
            None,
            &files(),
            &BTreeMap::new(),
        );
        let mut reversed = BTreeMap::new();
        reversed.insert("a.md".to_string(), b"one".to_vec());
        reversed.insert("b.md".to_string(), b"two".to_vec());
        let two = manifest_from(
            current(),
            1,
            Provenance::Native,
            None,
            &reversed,
            &BTreeMap::new(),
        );
        assert_eq!(one.payload_sha256, two.payload_sha256);
    }

    #[test]
    fn metadata_joins_the_manifest_as_a_second_role() {
        let metadata = BTreeMap::from([("m.toml".to_string(), b"x".to_vec())]);
        let manifest = manifest_from(
            current(),
            1,
            Provenance::LegacyAdapted,
            Some(Sha256::of(b"descriptor")),
            &files(),
            &metadata,
        );
        let roles: Vec<Role> = manifest
            .artifacts
            .iter()
            .map(|artifact| artifact.role)
            .collect();
        assert_eq!(roles, [Role::Payload, Role::Payload, Role::Metadata]);
        assert_eq!(manifest.provenance, Provenance::LegacyAdapted);
    }

    #[test]
    fn a_blob_is_served_by_digest_and_an_unknown_digest_refuses() {
        let held = files();
        assert_eq!(
            blob_from(&held, &BTreeMap::new(), &Sha256::of(b"one")).unwrap(),
            b"one"
        );
        assert!(blob_from(&held, &BTreeMap::new(), &Sha256::of(b"nope")).is_err());
    }
}
