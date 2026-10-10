//! Compare one stage against the target it was rendered for.
//!
//! One row per destination the stage carries, plus one per recorded
//! managed file the candidate no longer lands. Each row says whether the
//! target holds the staged bytes, who owns the destination, and, where the
//! two differ, how much this tool can say about who wrote what is there.
//!
//! The record half of that answer is the attribution check the landing
//! runs before it refuses unvouched bytes, so a row this reports as
//! unvouched is exactly a destination the landing would refuse. The
//! history half asks the target's own Git history whether any commit
//! touched the path. Nothing here writes.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::adapters::history::History;
use crate::candidate::Ownership;
use crate::error::AppError;

/// The machine schema `sdd stage compare --json` declares.
pub const SCHEMA: &str = "sdd.stage-compare/1";

/// What the target holds at one destination, against the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// The target holds the staged bytes.
    Equal,
    /// The target holds nothing there.
    Absent,
    /// The target holds other bytes.
    Differs,
    /// The record names a managed file the candidate no longer lands.
    Retired,
}

/// How much this tool can say about the bytes that differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provenance {
    /// The record vouches for the bytes and the history touches the path.
    Attributed,
    /// One of the two holds.
    Reduced,
    /// Neither holds.
    Unknown,
}

/// Who owns one destination after a landing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RowOwnership {
    /// The canon owns the whole file.
    Managed,
    /// The canon owns one marked region of a file the project owns.
    Integration,
    /// The project owns the file from the moment it lands.
    Adopted,
}

impl RowOwnership {
    const fn landing(self) -> Ownership {
        match self {
            Self::Managed => Ownership::Managed,
            Self::Integration => Ownership::Integration,
            Self::Adopted => Ownership::Adopted,
        }
    }

    /// The word the JSON spells.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Integration => "integration",
            Self::Adopted => "adopted",
        }
    }
}

/// One destination's comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// The destination, relative to the target.
    pub path: String,
    /// What the target holds against the stage.
    pub state: State,
    /// Who owns it.
    pub ownership: RowOwnership,
    /// For a destination that differs or retires, what vouches for it.
    pub provenance: Option<Provenance>,
    /// Whether the record vouches for the bytes the target holds, where
    /// the target holds any. The landing refuses exactly the managed and
    /// integration rows that differ with this false.
    pub vouched: Option<bool>,
    /// Whether the verb that lands this stage accepts the path in
    /// `--replace`: true only for a conflict `sdd upgrade` would take a
    /// name for, false on every other row.
    pub replaceable: bool,
}

/// Whether the history half could be read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistoryState {
    /// Whether the target's Git history was read.
    pub available: bool,
    /// Why not, where it was not.
    pub reason: Option<String>,
}

/// The whole comparison.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The machine schema of this object.
    pub schema: &'static str,
    /// The stage compared.
    pub root: Utf8PathBuf,
    /// The target compared against.
    pub target: Utf8PathBuf,
    /// Whether the history half was read.
    pub history: HistoryState,
    /// One row per destination, in the stage's order, then the retired.
    pub rows: Vec<Row>,
}

/// Compare the stage at `root` against `target`.
///
/// SATISFIES staging:compare-reports-every-destination-the-stage-carries
///
/// # Errors
///
/// Whatever [`super::read_choices`] refuses, and I/O errors reading a
/// staged artifact.
pub fn compare(root: &Utf8Path, target: &Utf8Path) -> Result<Report, AppError> {
    let choices = super::read_choices(root, target)?;
    let replaceable = crate::services::upgrader::replaceable(&choices)?;
    let target = choices.target.clone();
    let recorded = crate::services::installer::recorded(&target);
    let (history, history_state) = match History::open(&target) {
        Ok(history) => (
            Some(history),
            HistoryState {
                available: true,
                reason: None,
            },
        ),
        Err(missing) => (
            None,
            HistoryState {
                available: false,
                reason: Some(missing.to_string()),
            },
        ),
    };
    let touches = |path: &Utf8Path| history.as_ref().is_some_and(|held| held.touches(path));
    let provenance = |vouched: bool, path: &Utf8Path| match (vouched, touches(path)) {
        (true, true) => Provenance::Attributed,
        (false, false) => Provenance::Unknown,
        _ => Provenance::Reduced,
    };

    let mut rows = Vec::new();
    let mut staged_paths = Vec::new();
    for artifact in &choices.artifacts {
        let ownership = match artifact.ownership.as_str() {
            "managed" => RowOwnership::Managed,
            "integration" => RowOwnership::Integration,
            "adopted" => RowOwnership::Adopted,
            // The record itself is rewritten by every landing, so it is
            // never compared.
            _ => continue,
        };
        let path = Utf8PathBuf::from(&artifact.path);
        staged_paths.push(artifact.path.clone());
        let staged = std::fs::read(root.join(super::ARTIFACTS_DIR).join(&path))?;
        let Ok(held) = std::fs::read(target.join(&path)) else {
            rows.push(Row {
                path: artifact.path.clone(),
                state: State::Absent,
                ownership,
                provenance: None,
                vouched: None,
                replaceable: replaceable.contains(&artifact.path),
            });
            continue;
        };
        let vouched = crate::landing::apply::vouches(&recorded, &path, ownership.landing(), &held);
        let equal = match ownership {
            // A marked region is the canon's and the rest of its host is the
            // project's, so only the region is compared.
            RowOwnership::Integration => region(&path, &held) == region(&path, &staged),
            RowOwnership::Managed | RowOwnership::Adopted => held == staged,
        };
        rows.push(Row {
            path: artifact.path.clone(),
            state: if equal { State::Equal } else { State::Differs },
            ownership,
            provenance: (!equal).then(|| provenance(vouched, &path)),
            vouched: Some(vouched),
            replaceable: replaceable.contains(&artifact.path),
        });
    }
    for (destination, _) in &recorded.managed {
        if staged_paths.contains(destination) {
            continue;
        }
        let path = Utf8PathBuf::from(destination);
        // Every recorded destination gets its row. One that is gone has no
        // bytes to vouch for, so its provenance is the Git half alone, and
        // one that cannot be read is vouched for by nobody, so the operator
        // is asked rather than the row dropped.
        let (provenance, vouched) = match std::fs::read(target.join(&path)) {
            Ok(held) => {
                let vouched = crate::landing::apply::vouches_retired(&recorded, destination, &held);
                (Some(provenance(vouched, &path)), Some(vouched))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (Some(provenance(false, &path)), None)
            }
            Err(_) => (Some(Provenance::Unknown), None),
        };
        rows.push(Row {
            path: destination.clone(),
            state: State::Retired,
            ownership: RowOwnership::Managed,
            provenance,
            vouched,
            replaceable: replaceable.contains(destination),
        });
    }
    Ok(Report {
        schema: SCHEMA,
        root: root.to_owned(),
        target,
        history: history_state,
        rows,
    })
}

/// The hash of the marked region one host file carries.
fn region(path: &Utf8Path, bytes: &[u8]) -> Option<crate::domain::ownership::Sha256> {
    use crate::domain::marker;

    let text = std::str::from_utf8(bytes).ok()?;
    if path == crate::domain::paths::HOOKS_CONFIG_PATH {
        marker::block_hash(text)
    } else {
        marker::block_hash_with(text, marker::AGENTS_BEGIN, marker::AGENTS_END)
    }
}
