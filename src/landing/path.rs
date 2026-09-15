//! The target-relative path every write is bounded by.
//!
//! A destination is a path inside the target, and this type is where that
//! is proven once. Empty, absolute, climbing out, or carrying a NUL are
//! refusals rather than values a later writer has to re-check.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
