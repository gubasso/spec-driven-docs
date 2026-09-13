//! What a plan will do to a target, as a closed set.
//!
//! An operation names digests and never bytes. The bytes live in the
//! plan's blob store, so a plan can be read, printed, and compared without
//! carrying a repository's content around in it, and an apply cannot be
//! handed content the plan never described.
//!
//! There is no operation that runs a command. Every write is a typed write
//! into one validated target-relative path, so what a plan can do is
//! bounded by this enum rather than by what a string happens to say.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::ownership::Sha256;

/// A path an operation may name: relative, inside the target, and ordinary.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TargetPath(Utf8PathBuf);

/// A path no operation may name.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct TargetPathError(String);

impl TargetPath {
    /// Validate one target-relative path.
    ///
    /// # Errors
    ///
    /// [`TargetPathError`] for an empty path, an absolute path, a path
    /// that climbs out, a path carrying a NUL, or one with a trailing or
    /// repeated separator. A plan that could name any of those is a plan
    /// whose reach the type no longer bounds.
    pub fn new(value: &str) -> Result<Self, TargetPathError> {
        let refuse = |why: &str| TargetPathError(format!("{value}: {why}"));
        if value.is_empty() {
            return Err(refuse("the path is empty"));
        }
        if value.contains('\0') {
            return Err(refuse("the path carries a NUL"));
        }
        let path = Utf8Path::new(value);
        if path.is_absolute() {
            return Err(refuse("the path is absolute"));
        }
        let mut normalized = Utf8PathBuf::new();
        for component in path.components() {
            match component {
                camino::Utf8Component::Normal(part) => normalized.push(part),
                camino::Utf8Component::CurDir => {}
                camino::Utf8Component::ParentDir => {
                    return Err(refuse("the path climbs out of the target"));
                }
                camino::Utf8Component::RootDir | camino::Utf8Component::Prefix(_) => {
                    return Err(refuse("the path is absolute"));
                }
            }
        }
        if normalized.as_str().is_empty() {
            return Err(refuse("the path names no file"));
        }
        Ok(Self(normalized))
    }

    /// The path, relative to the target root.
    #[must_use]
    pub fn as_path(&self) -> &Utf8Path {
        &self.0
    }

    /// The path as it is written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for TargetPath {
    type Error = TargetPathError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl From<TargetPath> for String {
    fn from(value: TargetPath) -> Self {
        value.0.into_string()
    }
}

impl std::fmt::Display for TargetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// Who owns a file once it has landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Class {
    /// The canon keeps owning it, byte for byte.
    Managed,
    /// The project owns it from the moment it lands.
    Adopted,
}

/// One typed write a plan will make.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Operation {
    /// Put the release's bytes at a destination.
    WriteFile {
        /// Where.
        path: TargetPath,
        /// Who owns it afterwards.
        class: Class,
        /// What the target must still hold, or nothing where it holds none.
        before: Option<Sha256>,
        /// What the target will hold.
        after: Sha256,
    },
    /// Keep the target's own bytes and move the baseline they are read
    /// against.
    ///
    /// This is what an adopted file is: seeded once, then the project's.
    /// An upgrade moves the baseline so the next verification compares the
    /// project's copy against what the new release would have seeded.
    KeepFile {
        /// Where.
        path: TargetPath,
        /// What the target holds and keeps holding.
        held: Sha256,
        /// The baseline the record carries now.
        baseline_before: Sha256,
        /// The baseline the record will carry.
        baseline_after: Sha256,
    },
    /// Replace one marked region inside a file the project owns.
    SpliceBlock {
        /// Where.
        path: TargetPath,
        /// Which marked region.
        marker: String,
        /// The region's current digest, or nothing where it has none.
        before: Option<Sha256>,
        /// The region's digest afterwards.
        after: Sha256,
    },
    /// Take back a file the release no longer owns.
    ///
    /// And the directory that removal empties, where the directory is one
    /// the canon owns and is not itself an owned root. A skill is a
    /// directory holding one file, and an empty directory carrying a
    /// retired skill's name is one some agents still list. The directory
    /// is part of what this operation names rather than a second mutation
    /// beside it, because a file's removal is the only thing that can
    /// empty it.
    RemoveOwnedFile {
        /// Where.
        path: TargetPath,
        /// What the target must still hold.
        before: Sha256,
    },
    /// Write the inherited violations the operator asked to record.
    WriteDebt {
        /// Where.
        path: TargetPath,
        /// What the target must still hold, or nothing where it holds none.
        before: Option<Sha256>,
        /// What the target will hold.
        after: Sha256,
    },
    /// Write the instance record itself, last.
    WriteRecord {
        /// Where.
        path: TargetPath,
        /// What the target must still hold, or nothing where it holds none.
        before: Option<Sha256>,
        /// What the target will hold.
        after: Sha256,
    },
}

impl Operation {
    /// Where this operation writes.
    #[must_use]
    pub const fn path(&self) -> &TargetPath {
        match self {
            Self::WriteFile { path, .. }
            | Self::KeepFile { path, .. }
            | Self::SpliceBlock { path, .. }
            | Self::RemoveOwnedFile { path, .. }
            | Self::WriteDebt { path, .. }
            | Self::WriteRecord { path, .. } => path,
        }
    }

    /// The kebab-case kind, as the fingerprint and the JSON spell it.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::WriteFile { .. } => "write-file",
            Self::KeepFile { .. } => "keep-file",
            Self::SpliceBlock { .. } => "splice-block",
            Self::RemoveOwnedFile { .. } => "remove-owned-file",
            Self::WriteDebt { .. } => "write-debt",
            Self::WriteRecord { .. } => "write-record",
        }
    }

    /// What the target must still hold for this operation to be the one
    /// the plan described.
    #[must_use]
    pub const fn before(&self) -> Option<&Sha256> {
        match self {
            Self::WriteFile { before, .. }
            | Self::SpliceBlock { before, .. }
            | Self::WriteDebt { before, .. }
            | Self::WriteRecord { before, .. } => before.as_ref(),
            Self::RemoveOwnedFile { before, .. } => Some(before),
            Self::KeepFile { held, .. } => Some(held),
        }
    }

    /// What the target will hold, or nothing where the file goes.
    #[must_use]
    pub const fn after(&self) -> Option<&Sha256> {
        match self {
            Self::WriteFile { after, .. }
            | Self::SpliceBlock { after, .. }
            | Self::WriteDebt { after, .. }
            | Self::WriteRecord { after, .. } => Some(after),
            Self::KeepFile { held, .. } => Some(held),
            Self::RemoveOwnedFile { .. } => None,
        }
    }
}

/// Two operations naming one destination.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0} is written by two operations, {1} and {2}")]
pub struct DuplicateDestination(TargetPath, &'static str, &'static str);

/// Check that no destination is written twice.
///
/// # Errors
///
/// [`DuplicateDestination`] naming the path and both kinds. A plan that
/// wrote one destination twice would have an apply order nobody chose.
pub fn no_duplicate_destination(operations: &[Operation]) -> Result<(), DuplicateDestination> {
    for (index, operation) in operations.iter().enumerate() {
        for other in &operations[index + 1..] {
            if operation.path() == other.path() {
                return Err(DuplicateDestination(
                    operation.path().clone(),
                    operation.kind(),
                    other.kind(),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn path(value: &str) -> TargetPath {
        TargetPath::new(value).unwrap()
    }

    #[test]
    fn an_ordinary_relative_path_is_accepted_and_normalized() {
        assert_eq!(
            path("docs/specs/SPEC-x.md").as_str(),
            "docs/specs/SPEC-x.md"
        );
        assert_eq!(path("./docs/x.md").as_str(), "docs/x.md");
        assert_eq!(path("docs//x.md").as_str(), "docs/x.md");
    }

    #[test]
    fn a_path_a_plan_may_not_name_is_refused() {
        for value in ["", "/etc/passwd", "../escape.md", "docs/../../out.md"] {
            assert!(TargetPath::new(value).is_err(), "{value} was accepted");
        }
        assert!(TargetPath::new("docs/\0.md").is_err());
        assert!(TargetPath::new("./").is_err());
    }

    #[test]
    fn a_path_round_trips_through_its_string_form() {
        let held = path("docs/x.md");
        let json = serde_json::to_string(&held).unwrap();
        assert_eq!(json, "\"docs/x.md\"");
        assert_eq!(serde_json::from_str::<TargetPath>(&json).unwrap(), held);
        assert!(serde_json::from_str::<TargetPath>("\"../x.md\"").is_err());
    }

    #[test]
    fn every_operation_names_its_path_and_its_kind() {
        let write = Operation::WriteFile {
            path: path("a.md"),
            class: Class::Managed,
            before: None,
            after: Sha256::of(b"a"),
        };
        assert_eq!(write.kind(), "write-file");
        assert_eq!(write.path().as_str(), "a.md");
        assert_eq!(write.before(), None);
        assert_eq!(write.after(), Some(&Sha256::of(b"a")));

        let remove = Operation::RemoveOwnedFile {
            path: path("b.md"),
            before: Sha256::of(b"b"),
        };
        assert_eq!(remove.after(), None);
        assert_eq!(remove.before(), Some(&Sha256::of(b"b")));
    }

    #[test]
    fn a_kept_file_holds_its_bytes_and_moves_only_the_baseline() {
        let kept = Operation::KeepFile {
            path: path("docs/specs/SPEC-x.md"),
            held: Sha256::of(b"mine"),
            baseline_before: Sha256::of(b"old seed"),
            baseline_after: Sha256::of(b"new seed"),
        };
        assert_eq!(kept.before(), kept.after());
    }

    #[test]
    fn one_destination_written_twice_is_refused() {
        let operations = vec![
            Operation::WriteFile {
                path: path("a.md"),
                class: Class::Managed,
                before: None,
                after: Sha256::of(b"a"),
            },
            Operation::WriteDebt {
                path: path("a.md"),
                before: None,
                after: Sha256::of(b"b"),
            },
        ];
        let error = no_duplicate_destination(&operations).unwrap_err();
        assert!(error.to_string().contains("a.md"), "{error}");
        assert!(no_duplicate_destination(&operations[..1]).is_ok());
    }
}
