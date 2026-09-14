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
//! reads it, and `distribution:user-scope-files-stay-unrecorded` keeps
//! these paths out of the manifest that does drive verification.
//!
//! It is required state all the same. An apply that cannot write it fails
//! and rolls back, because a landing this tool cannot vouch for is a
//! landing it will refuse to take back. Reading is the forgiving half: a
//! record that is absent, unreadable, or written by a schema this binary
//! does not know reads as empty, and the caller loses the benefit of the
//! doubt and nothing else.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::domain::ownership::Sha256;

/// The record schema this binary writes.
pub const SCHEMA_VERSION: u32 = 2;

pub use crate::domain::paths::LEGACY_SKILL_RECEIPT_PATH as RECORD_PATH;

/// The digests this tool last wrote to user-scope skill destinations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillRecord {
    /// Always [`SCHEMA_VERSION`] once parsed.
    pub schema_version: u32,
    /// The engine version whose apply last wrote this record.
    #[serde(default)]
    pub engine_version: String,
    /// When that apply finished.
    #[serde(default)]
    pub installed_at: String,
    /// Absolute destination path to the digest written there.
    pub written: BTreeMap<Utf8PathBuf, Sha256>,
}

/// The shape schema one wrote, read through an adapter.
///
/// It carried the digests and nothing else. A home installed by a release
/// that wrote this shape keeps every vouched-for file, which is the whole
/// point of reading it rather than starting empty.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaOne {
    schema_version: u32,
    written: BTreeMap<Utf8PathBuf, Sha256>,
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
            engine_version: String::new(),
            installed_at: String::new(),
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
            .and_then(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    /// Read the record at `resolved`, or the one an older release left at
    /// `legacy`.
    ///
    /// One fallback read, and only where the resolved path holds nothing.
    /// The next apply writes the resolved path alone, so the legacy copy is
    /// read once in a home's life and then superseded.
    #[must_use]
    pub fn load_with_fallback(resolved: &Utf8Path, legacy: &Utf8Path) -> Self {
        let held = Self::load(resolved);
        if !held.written.is_empty() || resolved == legacy {
            return held;
        }
        Self::load(legacy)
    }

    /// Parse either schema this binary reads.
    fn parse(text: &str) -> Option<Self> {
        if let Ok(current) = serde_json::from_str::<Self>(text)
            && current.schema_version == SCHEMA_VERSION
        {
            return Some(current);
        }
        let older: SchemaOne = serde_json::from_str(text).ok()?;
        (older.schema_version == 1).then(|| Self {
            schema_version: SCHEMA_VERSION,
            written: older.written,
            ..Self::new()
        })
    }

    /// Serialize as pretty JSON with a trailing newline.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| "{\"schema_version\":2,\"written\":{}}".to_string());
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
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

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
            Utf8PathBuf::from("/home/<user>/.claude/skills/s/SKILL.md"),
            Sha256::of(b"x"),
        );
        std::fs::write(&file, record.to_json()).unwrap();
        assert_eq!(SkillRecord::load(&file), record);
    }

    #[test]
    fn wrote_answers_only_for_the_exact_path_and_digest() {
        let mut record = SkillRecord::new();
        let destination = Utf8PathBuf::from("/home/<user>/SKILL.md");
        record.written.insert(destination.clone(), Sha256::of(b"x"));
        assert!(record.wrote(&destination, &Sha256::of(b"x")));
        assert!(!record.wrote(&destination, &Sha256::of(b"y")));
        assert!(!record.wrote(Utf8Path::new("/home/<other>/SKILL.md"), &Sha256::of(b"x")));
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

    #[test]
    fn a_schema_one_record_reads_through_the_adapter() {
        let dir = tempfile::tempdir().unwrap();
        let older = path(&dir, "older.json");
        let destination = Utf8PathBuf::from("/home/<user>/.claude/skills/s/SKILL.md");
        std::fs::write(
            &older,
            format!(
                "{{\"schema_version\":1,\"written\":{{\"{destination}\":\"{}\"}}}}",
                Sha256::of(b"x")
            ),
        )
        .unwrap();
        let read = SkillRecord::load(&older);
        assert_eq!(read.schema_version, SCHEMA_VERSION);
        assert!(read.wrote(&destination, &Sha256::of(b"x")));
    }

    #[test]
    fn the_legacy_path_is_read_only_where_the_resolved_one_holds_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = path(&dir, "resolved.json");
        let legacy = path(&dir, "legacy.json");
        let mut older = SkillRecord::new();
        older
            .written
            .insert(Utf8PathBuf::from("/x/SKILL.md"), Sha256::of(b"x"));
        std::fs::write(&legacy, older.to_json()).unwrap();
        assert_eq!(SkillRecord::load_with_fallback(&resolved, &legacy), older);

        let mut current = SkillRecord::new();
        current
            .written
            .insert(Utf8PathBuf::from("/y/SKILL.md"), Sha256::of(b"y"));
        std::fs::write(&resolved, current.to_json()).unwrap();
        assert_eq!(SkillRecord::load_with_fallback(&resolved, &legacy), current);
    }
}
