//! Read another release from the registry that published it.
//!
//! This is the one network read in the tool, and it produces bytes a later
//! apply may write into a repository, so every step is bounded and every
//! byte is verified. The registry's own protocol decides the URLs: the
//! sparse index serves `config.json` and one line per version, and the
//! `dl` template in that configuration says where an archive lives. None
//! of it is a hard-coded download layout.
//!
//! The archive is untrusted input. It is verified against the index
//! checksum before it is parsed, expanded under caps on compressed bytes,
//! expanded bytes, entry count, and per-file bytes, and admitted only for
//! regular files under a declared payload root. A path that is absolute,
//! that climbs out, that is a link of either kind, or that repeats is
//! refused rather than skipped, because an archive that carries one is not
//! the archive the registry says it is.
//!
//! Nothing is executed. A bundle is data the planner reads.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::domain::ownership::Sha256;
use crate::domain::projection::Declaration;
use crate::error::AppError;
use crate::release::legacy::LegacyCatalog;
use crate::release::{
    Provenance, ReleaseBundle, ReleaseManifest, ReleaseResolver, ResolvedRelease, Selector,
    Version, blob_from, manifest_from,
};

/// The crate this tool distributes itself as.
pub const CRATE_NAME: &str = "spec-driven-docs";

/// The sparse index this tool reads.
pub const INDEX_ROOT: &str = "https://index.crates.io";

/// How long a connection may take to open.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the response headers may take to arrive.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
/// How long one whole read may take.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(120);
/// How many times a transient read is retried.
const RETRY_BUDGET: u32 = 2;
/// The longest a `Retry-After` is honoured.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(10);
/// The largest index document this tool reads.
const MAX_INDEX_BYTES: u64 = 16 * 1024 * 1024;
/// The largest archive this tool reads.
const MAX_COMPRESSED_BYTES: u64 = 32 * 1024 * 1024;
/// The most bytes an archive may expand to.
const MAX_EXPANDED_BYTES: u64 = 128 * 1024 * 1024;
/// The most entries an archive may hold.
const MAX_ENTRIES: usize = 20_000;
/// The largest single file an archive may hold.
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// One version, as the sparse index describes it.
#[derive(Debug, Clone, Deserialize)]
struct IndexEntry {
    vers: String,
    cksum: String,
    #[serde(default)]
    yanked: bool,
}

/// What the registry's own configuration says about download URLs.
#[derive(Debug, Clone, Deserialize)]
struct RegistryConfig {
    dl: String,
}

/// The frozen identity of one cached release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CachedIdentity {
    version: String,
    cksum: String,
    yanked: bool,
}

/// A release read from its published crate.
#[derive(Debug, Clone)]
pub struct CrateReleaseBundle {
    version: Version,
    payload_schema: u32,
    provenance: Provenance,
    descriptor_sha256: Option<Sha256>,
    files: BTreeMap<String, Vec<u8>>,
    metadata: BTreeMap<String, Vec<u8>>,
}

impl ReleaseBundle for CrateReleaseBundle {
    fn manifest(&self) -> Result<ReleaseManifest, AppError> {
        Ok(manifest_from(
            self.version.clone(),
            self.payload_schema,
            self.provenance,
            self.descriptor_sha256.clone(),
            &self.files,
            &self.metadata,
        ))
    }

    fn blob(&self, digest: &Sha256) -> Result<Vec<u8>, AppError> {
        blob_from(&self.files, &self.metadata, digest)
    }
}

/// Resolve a selector against crates.io, or against the cache alone.
#[derive(Debug, Clone)]
pub struct CratesIoResolver {
    cache: Utf8PathBuf,
    offline: bool,
    index_root: String,
    catalog: LegacyCatalog,
}

impl CratesIoResolver {
    /// A resolver caching under `cache`.
    #[must_use]
    pub fn new(cache: &Utf8Path) -> Self {
        Self {
            cache: cache.to_owned(),
            // The variable is the host saying the network is not there,
            // which is a stronger statement than a flag nobody passed.
            // The doctor already reads it; so does every read that would
            // otherwise fail slowly at a name it cannot resolve.
            offline: crate::domain::paths::variable(crate::domain::paths::OFFLINE_VAR).is_some(),
            index_root: INDEX_ROOT.to_string(),
            catalog: LegacyCatalog::embedded(),
        }
    }

    /// The same resolver, forbidden to reach the network.
    #[must_use]
    pub const fn offline(mut self, offline: bool) -> Self {
        self.offline = self.offline || offline;
        self
    }

    /// The same resolver, reading a different index root.
    ///
    /// A test serves the protocol from a local directory this way, so the
    /// suite exercises the resolver rather than a stub of it.
    #[must_use]
    pub fn with_index_root(mut self, root: &str) -> Self {
        self.index_root = root.to_string();
        self
    }

    fn identity_path(&self, version: &str) -> Utf8PathBuf {
        self.cache.join(format!("{version}.json"))
    }

    fn archive_path(&self, cksum: &str) -> Utf8PathBuf {
        self.cache.join(format!("{cksum}.crate"))
    }

    /// The sparse index path for a crate name, as the protocol defines it.
    #[must_use]
    pub fn index_path(name: &str) -> String {
        let lower = name.to_lowercase();
        match lower.len() {
            0 => lower,
            1 => format!("1/{lower}"),
            2 => format!("2/{lower}"),
            3 => format!("3/{}/{lower}", &lower[..1]),
            _ => format!("{}/{}/{lower}", &lower[..2], &lower[2..4]),
        }
    }

    /// One HTTP read, bounded and retried only where a retry is honest.
    fn read(&self, url: &str, limit: u64) -> Result<Vec<u8>, AppError> {
        if self.offline {
            return Err(AppError::Refused(format!(
                "--offline forbids the read of {url}; drop --offline or resolve a version already cached"
            )));
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_recv_response(Some(RESPONSE_TIMEOUT))
            .timeout_global(Some(TOTAL_TIMEOUT))
            .user_agent(format!(
                "sdd/{} (+{})",
                env!("CARGO_PKG_VERSION"),
                CRATE_NAME
            ))
            .build()
            .into();
        let mut attempt = 0;
        loop {
            match agent.get(url).call() {
                Ok(mut response) => {
                    let status = response.status().as_u16();
                    if transient(status) && attempt < RETRY_BUDGET {
                        std::thread::sleep(retry_after(&response));
                        attempt += 1;
                        continue;
                    }
                    if status != 200 {
                        return Err(AppError::Refused(format!("{url} answered {status}")));
                    }
                    return response
                        .body_mut()
                        .with_config()
                        .limit(limit)
                        .read_to_vec()
                        .map_err(|source| {
                            AppError::Refused(format!(
                                "{url} did not read within {limit} bytes: {source}"
                            ))
                        });
                }
                Err(source) if attempt < RETRY_BUDGET && is_transport(&source) => {
                    std::thread::sleep(Duration::from_millis(250));
                    attempt += 1;
                }
                Err(source) => {
                    return Err(AppError::Refused(format!(
                        "{url} could not be read: {source}"
                    )));
                }
            }
        }
    }

    /// Every version the index serves, in index order.
    fn index(&self) -> Result<Vec<IndexEntry>, AppError> {
        let url = format!("{}/{}", self.index_root, Self::index_path(CRATE_NAME));
        let bytes = self.read(&url, MAX_INDEX_BYTES)?;
        let text = String::from_utf8(bytes)
            .map_err(|source| AppError::Refused(format!("{url} is not text: {source}")))?;
        let mut entries = Vec::new();
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let entry: IndexEntry = serde_json::from_str(line).map_err(|source| {
                AppError::Refused(format!(
                    "{url} carries a line this engine cannot read: {source}"
                ))
            })?;
            entries.push(entry);
        }
        if entries.is_empty() {
            return Err(AppError::Refused(format!("{url} lists no version")));
        }
        Ok(entries)
    }

    /// Where the registry says an archive for one version lives.
    fn download_url(&self, version: &str, cksum: &str) -> Result<String, AppError> {
        let url = format!("{}/config.json", self.index_root);
        let bytes = self.read(&url, MAX_INDEX_BYTES)?;
        let config: RegistryConfig = serde_json::from_slice(&bytes)
            .map_err(|source| AppError::Refused(format!("{url} does not parse: {source}")))?;
        Ok(expand_download(&config.dl, CRATE_NAME, version, cksum))
    }

    /// The identity a previous resolution froze for this version.
    fn cached_identity(&self, version: &str) -> Option<CachedIdentity> {
        let text = std::fs::read_to_string(self.identity_path(version)).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn remember(&self, identity: &CachedIdentity, archive: &[u8]) -> Result<(), AppError> {
        std::fs::create_dir_all(&self.cache)?;
        crate::adapters::fs::write_atomic(&self.archive_path(&identity.cksum), archive)?;
        let text = serde_json::to_string_pretty(identity)
            .map_err(|source| anyhow::anyhow!("the cached identity did not serialize: {source}"))?;
        crate::adapters::fs::write_atomic(
            &self.identity_path(&identity.version),
            format!("{text}\n").as_bytes(),
        )?;
        Ok(())
    }

    /// The verified archive for one identity, from the cache or the network.
    ///
    /// The second value says whether these bytes are already cached. A
    /// checksum that matches the index proves the bytes are the published
    /// crate, and nothing more: the archive can still hold a traversal, a
    /// duplicate path, or a declaration this engine cannot read. Caching
    /// it here would serve a refusal offline forever, so the caller
    /// publishes it only after the whole bundle is admitted.
    fn archive(&self, identity: &CachedIdentity) -> Result<(Vec<u8>, bool), AppError> {
        let held = self.archive_path(&identity.cksum);
        if let Ok(bytes) = std::fs::read(&held)
            && Sha256::of(&bytes).as_str() == identity.cksum
        {
            return Ok((bytes, true));
        }
        let url = self.download_url(&identity.version, &identity.cksum)?;
        let bytes = self.read(&url, MAX_COMPRESSED_BYTES)?;
        let found = Sha256::of(&bytes);
        if found.as_str() != identity.cksum {
            // Nothing is cached: an archive whose checksum disagrees with
            // the registry is not the release, and keeping it would serve
            // the disagreement again offline.
            return Err(AppError::Refused(format!(
                "the archive for {} hashes to {found} and the index says {}; nothing was cached",
                identity.version, identity.cksum
            )));
        }
        Ok((bytes, false))
    }

    /// Build the bundle one verified archive carries.
    fn bundle(&self, identity: &CachedIdentity) -> Result<CrateReleaseBundle, AppError> {
        let version: Version = identity.version.parse().map_err(|_| {
            AppError::Refused(format!("{} is not a semantic version", identity.version))
        })?;
        // A release the audit already classified unavailable is refused
        // before anything is fetched: a download whose answer is a refusal
        // is a download nobody needed.
        if let Some(entry) = self.catalog.entry(&identity.version)
            && !entry.eligible
        {
            self.catalog.descriptor(&identity.version)?;
        }
        let (archive, cached) = self.archive(identity)?;
        let files = admit(&archive, &format!("{CRATE_NAME}-{}", identity.version))?;
        let native = files
            .get(crate::domain::projection::DECLARATION_PATH)
            .map(|bytes| Declaration::parse(bytes))
            .transpose()
            .map_err(|source| AppError::Refused(source.to_string()))?;
        if let Some(declaration) = native {
            if !cached {
                self.remember(identity, &archive)?;
            }
            return Ok(CrateReleaseBundle {
                version,
                payload_schema: declaration.payload_schema,
                provenance: Provenance::Native,
                descriptor_sha256: None,
                files,
                metadata: BTreeMap::new(),
            });
        }
        let adapted = self.catalog.adapt(&identity.version, &files)?;
        if !cached {
            self.remember(identity, &archive)?;
        }
        Ok(CrateReleaseBundle {
            version,
            payload_schema: adapted.payload_schema,
            provenance: Provenance::LegacyAdapted,
            descriptor_sha256: Some(adapted.descriptor_sha256),
            files,
            metadata: adapted.metadata,
        })
    }
}

impl ReleaseResolver for CratesIoResolver {
    fn resolve(&self, selector: &Selector) -> Result<ResolvedRelease, AppError> {
        let identity = match selector {
            Selector::Embedded => {
                return Err(AppError::Usage(
                    "the embedded release is not resolved through the registry".to_string(),
                ));
            }
            Selector::Exact(version) => {
                // A release the audit already classified unavailable is
                // answered from the catalog, before anything is fetched.
                // Its identity would cost an index read that only ever
                // leads to the same refusal, and on a host with no network
                // that read fails at the name rather than at the verdict.
                if self
                    .catalog
                    .entry(&version.to_string())
                    .is_some_and(|entry| !entry.eligible)
                {
                    self.catalog.descriptor(&version.to_string())?;
                }
                // An exact selector may be answered from a verified cache,
                // because the identity of an exact version cannot change.
                if let Some(held) = self.cached_identity(&version.to_string()) {
                    held
                } else {
                    let wanted = version.to_string();
                    let entries = self.index()?;
                    let found = entries
                        .iter()
                        .find(|entry| entry.vers == wanted)
                        .ok_or_else(|| {
                            AppError::Refused(format!(
                                "the registry serves no {CRATE_NAME} {wanted}"
                            ))
                        })?;
                    CachedIdentity {
                        version: found.vers.clone(),
                        cksum: found.cksum.clone(),
                        yanked: found.yanked,
                    }
                }
            }
            Selector::Latest => {
                // A cached answer cannot prove freshness, so `latest` always
                // reaches the index and `--offline` refuses it outright.
                if self.offline {
                    return Err(AppError::Refused(
                        "--offline cannot resolve latest, because only the index says which release is newest; name an exact version instead".to_string(),
                    ));
                }
                let entries = self.index()?;
                let mut stable: Vec<(Version, &IndexEntry)> = entries
                    .iter()
                    .filter(|entry| !entry.yanked)
                    .filter_map(|entry| Some((entry.vers.parse::<Version>().ok()?, entry)))
                    .filter(|(version, _)| version.pre.is_empty())
                    .collect();
                stable.sort_by(|left, right| left.0.cmp(&right.0));
                let (_, found) = stable.last().ok_or_else(|| {
                    AppError::Refused(format!("the registry serves no stable {CRATE_NAME}"))
                })?;
                CachedIdentity {
                    version: found.vers.clone(),
                    cksum: found.cksum.clone(),
                    yanked: found.yanked,
                }
            }
        };
        let bundle = self.bundle(&identity)?;
        let manifest = bundle.manifest()?;
        Ok(ResolvedRelease {
            selector: selector.clone(),
            version: manifest.version.clone(),
            registry_checksum: identity.cksum.parse().ok(),
            payload_sha256: manifest.payload_sha256,
            yanked: identity.yanked,
            bundle: Box::new(bundle),
        })
    }
}

/// Whether a status is worth one more try.
const fn transient(status: u16) -> bool {
    status == 429 || matches!(status, 500..=599)
}

/// Whether a failure was the transport rather than the answer.
const fn is_transport(error: &ureq::Error) -> bool {
    matches!(
        error,
        ureq::Error::Io(_) | ureq::Error::Timeout(_) | ureq::Error::ConnectionFailed
    )
}

/// What the response asks a client to wait, bounded.
fn retry_after(response: &ureq::http::Response<ureq::Body>) -> Duration {
    response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map_or(Duration::from_millis(500), |seconds| {
            Duration::from_secs(seconds).min(MAX_RETRY_AFTER)
        })
}

/// Fill the registry's download template, or fall back to the default form.
#[must_use]
#[allow(
    clippy::literal_string_with_formatting_args,
    reason = "the braces are the registry protocol's markers, not formatting arguments"
)]
pub fn expand_download(template: &str, name: &str, version: &str, cksum: &str) -> String {
    // Each entry is a marker the registry protocol defines, not a
    // formatting argument this function fills in.
    const MARKERS: [&str; 5] = [
        "{crate}",
        "{version}",
        "{prefix}",
        "{lowerprefix}",
        "{sha256-checksum}",
    ];
    if !MARKERS.iter().any(|marker| template.contains(marker)) {
        return format!("{template}/{name}/{version}/download");
    }
    let prefix = CratesIoResolver::index_path(name)
        .rsplit_once('/')
        .map_or_else(String::new, |(head, _)| head.to_string());
    template
        .replace("{crate}", name)
        .replace("{version}", version)
        .replace("{prefix}", &prefix)
        .replace("{lowerprefix}", &prefix.to_lowercase())
        .replace("{sha256-checksum}", cksum)
}

/// Read an archive into the payload files it is allowed to carry.
///
/// # Errors
///
/// [`AppError::Refused`] on any bound, on any entry that is not a regular
/// file under the package prefix, and on any duplicate logical path.
pub fn admit(archive: &[u8], prefix: &str) -> Result<BTreeMap<String, Vec<u8>>, AppError> {
    let refuse = |what: &str| AppError::Refused(format!("the archive is refused: {what}"));
    if archive.len() as u64 > MAX_COMPRESSED_BYTES {
        return Err(refuse("it is larger than the compressed cap"));
    }
    let decoder = flate2::read::GzDecoder::new(archive);
    let mut tar = tar::Archive::new(decoder.take(MAX_EXPANDED_BYTES));
    let roots: Vec<String> = crate::embedded::PAYLOAD_ROOTS
        .iter()
        .map(|root| format!("{root}/"))
        .collect();
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut entries = 0usize;
    let mut expanded = 0u64;
    for entry in tar
        .entries()
        .map_err(|source| refuse(&format!("its index does not read: {source}")))?
    {
        let mut entry =
            entry.map_err(|source| refuse(&format!("an entry does not read: {source}")))?;
        entries += 1;
        if entries > MAX_ENTRIES {
            return Err(refuse("it holds more entries than the cap"));
        }
        let path = entry
            .path()
            .map_err(|source| refuse(&format!("an entry has no readable path: {source}")))?
            .to_string_lossy()
            .to_string();
        if path.starts_with('/') || path.split('/').any(|part| part == "..") {
            return Err(refuse(&format!("{path} leaves the package")));
        }
        if !seen.insert(path.clone()) {
            return Err(refuse(&format!("{path} appears twice")));
        }
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            continue;
        }
        if !kind.is_file() {
            return Err(refuse(&format!("{path} is not a regular file")));
        }
        let Some(relative) = path
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('/'))
        else {
            return Err(refuse(&format!("{path} is outside {prefix}")));
        };
        if !roots.iter().any(|root| relative.starts_with(root)) {
            continue;
        }
        let size = entry.header().size().unwrap_or(u64::MAX);
        if size > MAX_FILE_BYTES {
            return Err(refuse(&format!(
                "{relative} is larger than the per-file cap"
            )));
        }
        expanded = expanded.saturating_add(size);
        if expanded > MAX_EXPANDED_BYTES {
            return Err(refuse("it expands past the cap"));
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|source| refuse(&format!("{relative} does not read: {source}")))?;
        files.insert(relative.to_string(), bytes);
    }
    if files.is_empty() {
        return Err(refuse("it carries no payload root"));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    /// One `.crate`-shaped archive built in memory.
    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (path, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *bytes).unwrap();
        }
        let tarred = builder.into_inner().unwrap();
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, &tarred).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn the_index_path_follows_the_registry_protocol() {
        assert_eq!(CratesIoResolver::index_path("a"), "1/a");
        assert_eq!(CratesIoResolver::index_path("ab"), "2/ab");
        assert_eq!(CratesIoResolver::index_path("abc"), "3/a/abc");
        assert_eq!(
            CratesIoResolver::index_path("spec-driven-docs"),
            "sp/ec/spec-driven-docs"
        );
    }

    #[test]
    fn a_download_template_without_markers_takes_the_default_form() {
        assert_eq!(
            expand_download("https://static.crates.io/crates", "x", "1.0.0", "ab"),
            "https://static.crates.io/crates/x/1.0.0/download"
        );
    }

    #[test]
    fn a_download_template_with_markers_is_filled() {
        assert_eq!(
            expand_download(
                "https://example.test/{prefix}/{crate}/{version}/{sha256-checksum}",
                "spec-driven-docs",
                "1.0.0",
                "abc"
            ),
            "https://example.test/sp/ec/spec-driven-docs/1.0.0/abc"
        );
    }

    #[test]
    fn an_archive_admits_only_payload_roots_under_the_package_prefix() {
        let bytes = archive(&[
            ("spec-driven-docs-1.0.0/method/one.md", b"one\n"),
            ("spec-driven-docs-1.0.0/src/main.rs", b"fn main() {}\n"),
            ("spec-driven-docs-1.0.0/Cargo.toml", b"[package]\n"),
        ]);
        let files = admit(&bytes, "spec-driven-docs-1.0.0").unwrap();
        assert_eq!(files.keys().collect::<Vec<_>>(), ["method/one.md"]);
    }

    #[test]
    fn an_archive_with_no_payload_root_refuses() {
        let bytes = archive(&[("spec-driven-docs-1.0.0/src/main.rs", b"x")]);
        let error = admit(&bytes, "spec-driven-docs-1.0.0").unwrap_err();
        assert!(error.to_string().contains("no payload root"), "{error}");
    }

    #[test]
    fn an_entry_outside_the_package_prefix_refuses() {
        let bytes = archive(&[("elsewhere/method/one.md", b"one\n")]);
        let error = admit(&bytes, "spec-driven-docs-1.0.0").unwrap_err();
        assert!(error.to_string().contains("outside"), "{error}");
    }

    /// One archive whose entry name the `tar` writer would refuse.
    ///
    /// The header is built by hand because the library will not produce a
    /// hostile name, and a hostile name is exactly what the admission rule
    /// exists to refuse.
    fn hostile(name: &str, body: &[u8]) -> Vec<u8> {
        let mut header = [0u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..108].copy_from_slice(b"0000644\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        header[124..136].copy_from_slice(format!("{:011o}\0", body.len()).as_bytes());
        header[136..148].copy_from_slice(b"00000000000\0");
        header[148..156].copy_from_slice(b"        ");
        header[156] = b'0';
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        let sum: u32 = header.iter().map(|byte| u32::from(*byte)).sum();
        header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());

        let mut tarred = header.to_vec();
        tarred.extend_from_slice(body);
        tarred.resize(tarred.len().next_multiple_of(512), 0);
        tarred.extend_from_slice(&[0u8; 1024]);

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, &tarred).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn traversal_and_absolute_paths_refuse() {
        for name in ["spec-driven-docs-1.0.0/../escape.md", "/etc/passwd"] {
            let bytes = hostile(name, b"x");
            let error = admit(&bytes, "spec-driven-docs-1.0.0").unwrap_err();
            assert!(
                error.to_string().contains("leaves the package"),
                "{name}: {error}"
            );
        }
    }

    #[test]
    fn a_duplicate_logical_path_refuses() {
        let bytes = archive(&[
            ("spec-driven-docs-1.0.0/method/one.md", b"one\n"),
            ("spec-driven-docs-1.0.0/method/one.md", b"two\n"),
        ]);
        let error = admit(&bytes, "spec-driven-docs-1.0.0").unwrap_err();
        assert!(error.to_string().contains("appears twice"), "{error}");
    }

    #[test]
    fn a_link_entry_refuses() {
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(0);
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_mode(0o777);
        builder
            .append_link(
                &mut header,
                "spec-driven-docs-1.0.0/method/link.md",
                "/etc/passwd",
            )
            .unwrap();
        let tarred = builder.into_inner().unwrap();
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, &tarred).unwrap();
        let bytes = encoder.finish().unwrap();
        let error = admit(&bytes, "spec-driven-docs-1.0.0").unwrap_err();
        assert!(error.to_string().contains("not a regular file"), "{error}");
    }

    #[test]
    fn a_file_over_the_per_file_cap_refuses() {
        let big = vec![b'x'; usize::try_from(MAX_FILE_BYTES).unwrap() + 1];
        let bytes = archive(&[("spec-driven-docs-1.0.0/method/big.md", &big)]);
        let error = admit(&bytes, "spec-driven-docs-1.0.0").unwrap_err();
        assert!(error.to_string().contains("per-file cap"), "{error}");
    }

    #[test]
    fn offline_refuses_latest_and_an_uncached_exact() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Utf8PathBuf::from(dir.path().to_str().unwrap());
        let resolver = CratesIoResolver::new(&cache).offline(true);
        let error = resolver.resolve(&Selector::Latest).unwrap_err();
        assert!(
            error.to_string().contains("cannot resolve latest"),
            "{error}"
        );
        let error = resolver
            .resolve(&Selector::Exact("0.8.0".parse().unwrap()))
            .unwrap_err();
        assert!(error.to_string().contains("--offline forbids"), "{error}");
    }

    #[test]
    fn the_embedded_selector_is_not_the_registry_resolvers_business() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Utf8PathBuf::from(dir.path().to_str().unwrap());
        let error = CratesIoResolver::new(&cache)
            .resolve(&Selector::Embedded)
            .unwrap_err();
        assert_eq!(error.exit_code(), 64);
    }
}
