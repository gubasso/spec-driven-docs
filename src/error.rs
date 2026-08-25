//! Crate-level error type and exit-code mapping.
//!
//! [`AppError`] aggregates errors from every layer via `#[from]`, and
//! [`AppError::exit_code`] maps each variant to its process exit code. No
//! other module decides exit codes. Exit `1` is reserved for one meaning —
//! a check ran and found violations — while the BSD sysexits range covers
//! the tool failing to do its job at all.

use camino::Utf8PathBuf;
use thiserror::Error;

use crate::domain::marker::MarkerError;

/// Every failure the binary can exit with.
#[derive(Debug, Error)]
pub enum AppError {
    /// Semantically invalid arguments clap cannot reject on shape alone.
    #[error("usage: {0}")]
    Usage(String),

    /// A check ran to completion and found violations; the findings are
    /// already on stdout, so the process only carries the red exit.
    #[error("{count} violation(s) found")]
    Violations {
        /// How many findings were reported.
        count: usize,
    },

    /// The target has no instance manifest where one is required.
    #[error("missing manifest: {0}")]
    ManifestMissing(Utf8PathBuf),

    /// The manifest exists but does not parse as a supported schema.
    #[error("invalid manifest: {0}")]
    ManifestInvalid(String),

    /// The managed pre-commit block or its host file is malformed.
    #[error(transparent)]
    Marker(#[from] MarkerError),

    /// The install or upgrade refused to touch the target as found, and the
    /// target was left (or restored) unchanged.
    #[error("{0}")]
    Refused(String),

    /// Filesystem failure, classified by its I/O kind.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// Escape hatch for ad-hoc contexts at the binary boundary.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppError {
    /// Map to the process exit code.
    ///
    /// `1` means a completed check found violations; `64..=78` follow BSD
    /// `sysexits(3)` and mean the tool itself could not do its job.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Violations { .. } => 1,
            Self::Usage(_) => 64,
            Self::ManifestInvalid(_) | Self::Marker(_) => 65,
            Self::ManifestMissing(_) => 66,
            Self::Refused(_) => 73,
            Self::Io(e) if e.kind() == std::io::ErrorKind::NotFound => 66,
            Self::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => 77,
            Self::Io(_) => 74,
            Self::Other(_) => 70,
        }
    }

    /// Stable machine-readable kind for the error envelope.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Usage(_) => "Usage",
            Self::Violations { .. } => "Violations",
            Self::ManifestMissing(_) => "ManifestMissing",
            Self::ManifestInvalid(_) => "ManifestInvalid",
            Self::Marker(_) => "Marker",
            Self::Refused(_) => "Refused",
            Self::Io(_) => "Io",
            Self::Other(_) => "Other",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn io(kind: std::io::ErrorKind) -> AppError {
        AppError::Io(std::io::Error::from(kind))
    }

    #[test]
    fn violations_are_one() {
        assert_eq!(AppError::Violations { count: 3 }.exit_code(), 1);
    }

    #[test]
    fn usage_is_sixty_four() {
        assert_eq!(AppError::Usage("bad target".into()).exit_code(), 64);
    }

    #[test]
    fn invalid_manifest_is_sixty_five() {
        assert_eq!(
            AppError::ManifestInvalid("schema_version 3".into()).exit_code(),
            65
        );
    }

    #[test]
    fn malformed_marker_is_sixty_five() {
        assert_eq!(AppError::Marker(MarkerError::Malformed).exit_code(), 65);
    }

    #[test]
    fn missing_manifest_is_sixty_six() {
        assert_eq!(
            AppError::ManifestMissing("x/.spec-driven-docs/manifest.json".into()).exit_code(),
            66
        );
    }

    #[test]
    fn refused_is_seventy_three() {
        assert_eq!(
            AppError::Refused("apply aborted; the target was restored".into()).exit_code(),
            73
        );
    }

    #[test]
    fn not_found_io_is_sixty_six() {
        assert_eq!(io(std::io::ErrorKind::NotFound).exit_code(), 66);
    }

    #[test]
    fn permission_denied_io_is_seventy_seven() {
        assert_eq!(io(std::io::ErrorKind::PermissionDenied).exit_code(), 77);
    }

    #[test]
    fn other_io_is_seventy_four() {
        assert_eq!(io(std::io::ErrorKind::BrokenPipe).exit_code(), 74);
    }

    #[test]
    fn anyhow_is_seventy() {
        assert_eq!(AppError::Other(anyhow::anyhow!("boom")).exit_code(), 70);
    }
}
