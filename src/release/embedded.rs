//! The release this binary carries, behind the bundle interface.
//!
//! Every landing verb reads through this in the ordinary case, so the whole
//! suite exercises the boundary rather than a path only another release
//! takes.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::domain::ownership::Sha256;
use crate::domain::projection::PAYLOAD_SCHEMA;
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::release::{
    Provenance, ReleaseBundle, ReleaseManifest, Version, blob_from, manifest_from,
};

/// This binary's own version, in the form the registry speaks.
#[expect(
    clippy::expect_used,
    reason = "the crate version is a released triple; a build that says otherwise cannot ship"
)]
fn version() -> Version {
    CanonVersion::current()
        .to_string()
        .parse()
        .expect("the crate version parses as a semantic version")
}

/// Every embedded payload file, by the logical path the projection names.
static FILES: LazyLock<BTreeMap<String, Vec<u8>>> = LazyLock::new(|| {
    let mut files = BTreeMap::new();
    for (root, dir) in crate::embedded::roots() {
        collect(root, dir, &mut files);
    }
    files
});

fn collect(
    root: &str,
    dir: &'static include_dir::Dir<'static>,
    files: &mut BTreeMap<String, Vec<u8>>,
) {
    for file in dir.files() {
        if let Some(rest) = file.path().to_str() {
            files.insert(format!("{root}/{rest}"), file.contents().to_vec());
        }
    }
    for sub in dir.dirs() {
        collect(root, sub, files);
    }
}

/// The release this binary carries.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmbeddedReleaseBundle;

impl EmbeddedReleaseBundle {
    /// The bundle this binary is.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl ReleaseBundle for EmbeddedReleaseBundle {
    fn manifest(&self) -> Result<ReleaseManifest, AppError> {
        Ok(manifest_from(
            version(),
            PAYLOAD_SCHEMA,
            Provenance::Native,
            None,
            &FILES,
            &BTreeMap::new(),
        ))
    }

    fn blob(&self, digest: &Sha256) -> Result<Vec<u8>, AppError> {
        blob_from(&FILES, &BTreeMap::new(), digest)
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
    fn the_embedded_bundle_carries_every_payload_root() {
        let manifest = EmbeddedReleaseBundle::new().manifest().unwrap();
        for root in crate::embedded::PAYLOAD_ROOTS {
            assert!(
                manifest
                    .artifacts
                    .iter()
                    .any(|artifact| artifact.path.starts_with(&format!("{root}/"))),
                "{root} is absent from the bundle"
            );
        }
        assert_eq!(manifest.version, version());
        assert_eq!(manifest.provenance, Provenance::Native);
    }

    #[test]
    fn every_artifact_resolves_to_its_own_bytes() {
        let bundle = EmbeddedReleaseBundle::new();
        let manifest = bundle.manifest().unwrap();
        for artifact in &manifest.artifacts {
            let bytes = bundle.blob(&artifact.sha256).unwrap();
            assert_eq!(Sha256::of(&bytes), artifact.sha256);
            assert_eq!(bytes.len() as u64, artifact.bytes);
        }
    }

    #[test]
    fn the_declaration_reads_through_the_seam() {
        let declaration = EmbeddedReleaseBundle::new().declaration().unwrap();
        assert_eq!(declaration.payload_schema, PAYLOAD_SCHEMA);
        assert_eq!(
            declaration.managed,
            crate::domain::profile::DECLARATION.managed
        );
    }

    #[test]
    fn every_projected_source_is_an_artifact() {
        let manifest = EmbeddedReleaseBundle::new().manifest().unwrap();
        let declaration = &crate::domain::profile::DECLARATION;
        for entry in declaration.managed.iter().chain(&declaration.adopted) {
            assert!(
                manifest.digest_of(&entry.source).is_some(),
                "{} is projected but not carried",
                entry.source
            );
        }
    }
}
