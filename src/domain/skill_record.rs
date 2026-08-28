//! The user-scope skill record: what this tool last wrote outside an instance.
//!
//! Skill destinations live under the invoking user's home, where no instance
//! manifest reaches, so without a record the installer's only reference is
//! the payload it currently carries. That makes a copy left by an older
//! release indistinguishable from a file the user edited, and every release
//! that touches a skill refuses on destinations nobody touched. This record
//! closes that gap and nothing else: one digest per destination, written
//! after a successful apply, read to answer one question — are these bytes
//! ones we wrote?
//!
//! It is not an instance manifest and never becomes one. No verification
//! reads it, a missing or unreadable record only costs the caller the
//! benefit of the doubt, and `distribution:user-scope-files-stay-unrecorded`
//! keeps these paths out of the manifest that does drive verification.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::domain::ownership::Sha256;

/// The record schema this binary reads and writes.
pub const SCHEMA_VERSION: u32 = 1;

/// Where the record sits, relative to the home directory.
///
/// Home-relative rather than `XDG_STATE_HOME`-relative on purpose: the
/// destinations it describes are `$HOME/.agents` and `$HOME/.claude`, which
/// no XDG variable moves. A record reachable under a different home than the
/// roots it speaks for would be worse than no record at all.
pub const RECORD_PATH: &str = ".local/state/spec-driven-docs/skills.json";

/// The digests this tool last wrote to user-scope skill destinations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillRecord {
    /// Always [`SCHEMA_VERSION`] once parsed.
    pub schema_version: u32,
    /// Absolute destination path to the digest written there.
    pub written: BTreeMap<Utf8PathBuf, Sha256>,
}

impl Default for SkillRecord {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRecord {
    /// An empty record at the current schema.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            written: BTreeMap::new(),
        }
    }

    /// Read the record at `path`, or an empty one.
    ///
    /// Every failure resolves to an empty record: absent, unreadable,
    /// malformed, and written by a schema this binary does not know all mean
    /// the same thing to a caller — nothing here can vouch for a
    /// destination. Refusing instead would let a corrupt state file block an
    /// install that has a `--force` it does not need.
    #[must_use]
    pub fn load(path: &Utf8Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .filter(|record| record.schema_version == SCHEMA_VERSION)
            .unwrap_or_default()
    }

    /// Serialize as pretty JSON with a trailing newline.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| "{\"schema_version\":1,\"written\":{}}".to_string());
        text.push('\n');
        text
    }

    /// Whether `digest` is what this tool last wrote to `destination`.
    #[must_use]
    pub fn wrote(&self, destination: &Utf8Path, digest: &Sha256) -> bool {
        self.written.get(destination) == Some(digest)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn path(dir: &tempfile::TempDir, name: &str) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap()).join(name)
    }

    #[test]
    fn a_round_trip_preserves_every_entry() {
        let dir = tempfile::tempdir().unwrap();
        let file = path(&dir, "skills.json");
        let mut record = SkillRecord::new();
        record.written.insert(
            Utf8PathBuf::from("/home/a/.claude/skills/s/SKILL.md"),
            Sha256::of(b"x"),
        );
        std::fs::write(&file, record.to_json()).unwrap();
        assert_eq!(SkillRecord::load(&file), record);
    }

    #[test]
    fn wrote_answers_only_for_the_exact_path_and_digest() {
        let mut record = SkillRecord::new();
        let destination = Utf8PathBuf::from("/home/a/SKILL.md");
        record.written.insert(destination.clone(), Sha256::of(b"x"));
        assert!(record.wrote(&destination, &Sha256::of(b"x")));
        assert!(!record.wrote(&destination, &Sha256::of(b"y")));
        assert!(!record.wrote(Utf8Path::new("/home/b/SKILL.md"), &Sha256::of(b"x")));
    }

    /// A caller that cannot read the record loses the benefit of the doubt
    /// and nothing else, so every unreadable shape resolves the same way.
    #[test]
    fn an_unusable_record_reads_as_empty_rather_than_failing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            SkillRecord::load(&path(&dir, "absent.json")),
            SkillRecord::new()
        );

        let malformed = path(&dir, "malformed.json");
        std::fs::write(&malformed, "{not json").unwrap();
        assert_eq!(SkillRecord::load(&malformed), SkillRecord::new());

        let future = path(&dir, "future.json");
        std::fs::write(&future, "{\"schema_version\":99,\"written\":{}}").unwrap();
        assert_eq!(SkillRecord::load(&future), SkillRecord::new());
    }
}
