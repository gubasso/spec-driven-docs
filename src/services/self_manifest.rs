//! Write the payload record: what this tree embeds, described by the tree.
//!
//! The record is an artifact of the tree, like the binary. Its identity is
//! the crate version of `Cargo.toml` and the hash of every file under the
//! declared payload roots, so it describes the payload of the checkout that
//! writes it whatever sdd a devshell pins. It holds no instance field, and
//! nothing compares it with an instance record: this repository's own
//! instance is an ordinary one, landed by its pinned previous release.
//!
//! SATISFIES release:the-payload-record-describes-its-tree

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::adapters::fs::sha256_file;
use crate::domain::ownership::Sha256;
use crate::domain::version::CanonVersion;
use crate::error::AppError;

/// Where the tree keeps its payload record, relative to its root.
pub const PAYLOAD_RECORD_PATH: &str = "instance/payload-manifest.json";

/// The schema id the payload record carries.
pub const PAYLOAD_RECORD_SCHEMA: &str = "sdd.payload-manifest/1";

/// One payload source and its digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayloadFile {
    /// The path relative to the tree root.
    pub path: Utf8PathBuf,
    /// The digest of its bytes.
    pub sha256: Sha256,
}

/// The description of the payload one tree embeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadRecord {
    /// The schema id, [`PAYLOAD_RECORD_SCHEMA`].
    pub schema: String,
    /// The crate version of the tree.
    pub version: CanonVersion,
    /// Every file under the declared payload roots, sorted by path, the
    /// record itself excepted.
    pub files: Vec<PayloadFile>,
}

impl PayloadRecord {
    /// The record as the tree writes it: pretty JSON with a final newline.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }
}

pub(crate) fn is_canon_checkout(root: &Utf8Path) -> bool {
    std::fs::read_to_string(root.join("Cargo.toml"))
        .is_ok_and(|cargo| cargo.contains("name = \"spec-driven-docs\""))
}

/// Describe the payload the tree at `root` carries.
///
/// # Errors
///
/// I/O errors walking a root or reading a file.
pub fn describe(root: &Utf8Path) -> Result<PayloadRecord, AppError> {
    let mut files = Vec::new();
    for declared in crate::payload_roots::PAYLOAD_ROOTS {
        if !root.join(declared).exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(root.join(declared)).sort_by_file_name() {
            let entry = entry.map_err(|source| AppError::Io(std::io::Error::from(source)))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let Some(path) = Utf8Path::from_path(entry.path()) else {
                continue;
            };
            let relative = path.strip_prefix(root).unwrap_or(path).to_owned();
            if relative == PAYLOAD_RECORD_PATH {
                continue;
            }
            files.push(PayloadFile {
                sha256: sha256_file(path)?,
                path: relative,
            });
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(PayloadRecord {
        schema: PAYLOAD_RECORD_SCHEMA.to_string(),
        version: CanonVersion::current(),
        files,
    })
}

/// Write the payload record of the canon checkout at `root`.
///
/// # Errors
///
/// [`AppError::Refused`] outside the canon checkout, and I/O errors when a
/// payload file cannot be read or the record cannot be written.
pub fn regenerate(root: &Utf8Path) -> Result<String, AppError> {
    if !is_canon_checkout(root) {
        return Err(AppError::Refused("not the canon checkout".to_string()));
    }
    let record = describe(root)?;
    crate::adapters::fs::write_file(&root.join(PAYLOAD_RECORD_PATH), record.to_json().as_bytes())?;
    Ok(format!("OK regenerated {PAYLOAD_RECORD_PATH}"))
}
