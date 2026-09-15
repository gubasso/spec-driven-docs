//! Read the registry index, to learn which release is newest.
//!
//! This is the one network read this tool makes, and it reads one thing:
//! the sparse index line per published version. Nothing is downloaded and
//! nothing is written into a project, because moving a consumer's pin is a
//! text edit and the version number is all it needs.
//!
//! The registry's own protocol decides the URL. The index path is derived
//! from the crate name the way the protocol defines it, so no layout is
//! hard-coded beyond the index root.

use std::time::Duration;

use semver::Version;
use serde::Deserialize;

use crate::error::AppError;

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

/// One published version, as the sparse index states it.
#[derive(Debug, Clone, Deserialize)]
struct IndexEntry {
    vers: String,
    #[serde(default)]
    yanked: bool,
}

/// The registry index, read over the network.
#[derive(Debug, Clone)]
pub struct Index {
    root: String,
    offline: bool,
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Index {
    /// The public index.
    ///
    /// The offline variable is the host saying the network is not there,
    /// which is a stronger statement than a flag nobody passed. A read that
    /// would otherwise fail slowly at a name it cannot resolve refuses at
    /// once instead.
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: INDEX_ROOT.to_string(),
            offline: crate::domain::paths::variable(crate::domain::paths::OFFLINE_VAR).is_some(),
        }
    }

    /// Refuse every read, for a caller that declared itself offline.
    #[must_use]
    pub const fn offline(mut self, offline: bool) -> Self {
        self.offline = self.offline || offline;
        self
    }

    /// Read another index root, for a test that serves its own.
    #[must_use]
    pub fn with_root(mut self, root: &str) -> Self {
        self.root = root.trim_end_matches('/').to_string();
        self
    }

    /// The newest stable release the registry serves.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`] when the caller is offline, when the index
    /// cannot be read, or when it lists no stable version.
    pub fn latest_version(&self) -> Result<Version, AppError> {
        if self.offline {
            return Err(AppError::Refused(
                "offline: only the index says which release is newest".to_string(),
            ));
        }
        let url = format!("{}/{}", self.root, index_path(CRATE_NAME));
        let bytes = read(&url)?;
        let text = String::from_utf8(bytes)
            .map_err(|source| AppError::Refused(format!("{url} is not text: {source}")))?;
        let mut entries = Vec::new();
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let entry: IndexEntry = serde_json::from_str(line).map_err(|source| {
                AppError::Refused(format!(
                    "{url} carries a line this tool cannot read: {source}"
                ))
            })?;
            entries.push(entry);
        }
        entries
            .iter()
            .filter(|entry| !entry.yanked)
            .filter_map(|entry| entry.vers.parse::<Version>().ok())
            .filter(|version| version.pre.is_empty())
            .max()
            .ok_or_else(|| AppError::Refused(format!("the registry serves no stable {CRATE_NAME}")))
    }
}

/// One bounded read, retried where the answer says to.
fn read(url: &str) -> Result<Vec<u8>, AppError> {
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
                    .limit(MAX_INDEX_BYTES)
                    .read_to_vec()
                    .map_err(|source| {
                        AppError::Refused(format!(
                            "{url} did not read within {MAX_INDEX_BYTES} bytes: {source}"
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

/// Whether an answer is worth asking for again.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_path_follows_the_registry_protocol() {
        assert_eq!(index_path("a"), "1/a");
        assert_eq!(index_path("ab"), "2/ab");
        assert_eq!(index_path("abc"), "3/a/abc");
        assert_eq!(index_path(CRATE_NAME), "sp/ec/spec-driven-docs");
    }

    #[test]
    fn an_offline_index_refuses_rather_than_guessing() {
        let error = Index::new().offline(true).latest_version().unwrap_err();
        assert_eq!(error.kind(), "Refused");
        assert!(error.to_string().contains("newest"), "{error}");
    }

    #[test]
    fn a_transient_answer_is_worth_asking_again() {
        assert!(transient(429));
        assert!(transient(503));
        assert!(!transient(404));
    }
}
