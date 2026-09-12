//! A directory served as a bundle, for tests.
//!
//! The seam's third implementation exists so a test can describe a release
//! the binary does not carry without reaching a registry. It is the only
//! bundle whose bytes nobody verified, so nothing outside a test builds
//! one.

use std::collections::BTreeMap;

use camino::Utf8Path;

use crate::domain::ownership::Sha256;
use crate::error::AppError;
use crate::release::{
    Provenance, ReleaseBundle, ReleaseManifest, ReleaseResolver, ResolvedRelease, Selector,
    Version, blob_from, manifest_from,
};

/// A release read from a directory tree.
#[derive(Debug, Clone)]
pub struct FixtureReleaseBundle {
    version: Version,
    payload_schema: u32,
    files: BTreeMap<String, Vec<u8>>,
}

impl FixtureReleaseBundle {
    /// Read every file under `root` as one release.
    ///
    /// # Errors
    ///
    /// Any I/O error walking or reading the directory.
    pub fn read(root: &Utf8Path, version: Version, payload_schema: u32) -> Result<Self, AppError> {
        let mut files = BTreeMap::new();
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(path) = camino::Utf8PathBuf::from_path_buf(entry.path().to_path_buf()) else {
                continue;
            };
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            files.insert(relative.to_string(), std::fs::read(&path)?);
        }
        Ok(Self {
            version,
            payload_schema,
            files,
        })
    }

    /// A release built from paths and bytes given directly.
    #[must_use]
    pub const fn of(
        version: Version,
        payload_schema: u32,
        files: BTreeMap<String, Vec<u8>>,
    ) -> Self {
        Self {
            version,
            payload_schema,
            files,
        }
    }
}

impl ReleaseBundle for FixtureReleaseBundle {
    fn manifest(&self) -> Result<ReleaseManifest, AppError> {
        Ok(manifest_from(
            self.version.clone(),
            self.payload_schema,
            Provenance::Native,
            None,
            &self.files,
            &BTreeMap::new(),
        ))
    }

    fn blob(&self, digest: &Sha256) -> Result<Vec<u8>, AppError> {
        blob_from(&self.files, &BTreeMap::new(), digest)
    }
}

impl ReleaseResolver for FixtureReleaseBundle {
    fn resolve(&self, selector: &Selector) -> Result<ResolvedRelease, AppError> {
        let manifest = self.manifest()?;
        Ok(ResolvedRelease {
            selector: selector.clone(),
            version: self.version.clone(),
            registry_checksum: None,
            payload_sha256: manifest.payload_sha256,
            yanked: false,
            bundle: Box::new(self.clone()),
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
    fn the_fixture_source_serves_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from(dir.path().to_str().unwrap());
        crate::adapters::fs::write_file(&root.join("method/one.md"), b"one\n").unwrap();
        crate::adapters::fs::write_file(&root.join("templates/two.md"), b"two\n").unwrap();

        let version: Version = "0.1.0".parse().unwrap();
        let bundle = FixtureReleaseBundle::read(&root, version.clone(), 1).unwrap();
        let manifest = bundle.manifest().unwrap();
        let paths: Vec<&str> = manifest
            .artifacts
            .iter()
            .map(|artifact| artifact.path.as_str())
            .collect();
        assert_eq!(paths, ["method/one.md", "templates/two.md"]);
        assert_eq!(bundle.artifact("method/one.md").unwrap(), b"one\n");
        assert!(bundle.artifact("absent.md").is_err());

        let resolved = bundle.resolve(&Selector::Exact(version.clone())).unwrap();
        assert_eq!(resolved.version, version);
        assert_eq!(resolved.payload_sha256, manifest.payload_sha256);
    }
}
