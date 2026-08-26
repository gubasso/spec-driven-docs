//! Report an instance's state as data.
//!
//! Status answers what the verifier's text answers, but as one typed
//! record a machine can branch on, and it treats a missing instance as an
//! answer rather than an error. A corrupt manifest still raises: absence
//! and breakage are different findings, and reporting a broken instance as
//! absent would invite a destructive re-init.

use camino::Utf8Path;
use serde::Serialize;

use crate::domain::profile::{DocsRoot, ProfileId};
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::services::verifier;

/// How an instance's canon version relates to this binary's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Alignment {
    /// The versions are equal.
    Aligned,
    /// The binary is newer; `sdd upgrade` moves the instance.
    BinaryNewer,
    /// The instance is newer; a newer `sdd` is required.
    InstanceNewer,
}

/// One instance's state, as `sdd status` reports it.
#[derive(Debug, Serialize)]
pub struct StatusReport {
    /// Whether the target carries an instance.
    pub instance: bool,
    /// The installed profile.
    pub profile: Option<ProfileId>,
    /// The instance's documentation root.
    pub docs_root: Option<DocsRoot>,
    /// The canon version that produced the instance.
    pub canon_version: Option<CanonVersion>,
    /// The version this binary carries.
    pub binary_version: CanonVersion,
    /// How the two versions relate.
    pub alignment: Option<Alignment>,
    /// Managed files missing, symlinked, or byte-drifted.
    pub managed_drift: usize,
    /// Adopted files awaiting reconciliation.
    pub adopted_drift: usize,
    /// Verification failures in total.
    pub failures: usize,
    /// Whether verification passes.
    pub ok: Option<bool>,
}

fn absent() -> StatusReport {
    StatusReport {
        instance: false,
        profile: None,
        docs_root: None,
        canon_version: None,
        binary_version: CanonVersion::current(),
        alignment: None,
        managed_drift: 0,
        adopted_drift: 0,
        failures: 0,
        ok: None,
    }
}

/// Report the target's instance state.
///
/// # Errors
///
/// [`AppError::ManifestInvalid`] when a manifest exists but cannot be
/// trusted, and I/O errors when the disk cannot be read. A missing
/// manifest is a report, not an error.
pub fn status(target: &Utf8Path) -> Result<StatusReport, AppError> {
    let manifest = match verifier::read_manifest(target) {
        Ok(manifest) => manifest,
        Err(AppError::ManifestMissing(_)) => return Ok(absent()),
        Err(error) => return Err(error),
    };
    let report = verifier::verify(target)?;
    let binary = CanonVersion::current();
    let alignment = match manifest.canon_version.cmp(&binary) {
        std::cmp::Ordering::Equal => Alignment::Aligned,
        std::cmp::Ordering::Less => Alignment::BinaryNewer,
        std::cmp::Ordering::Greater => Alignment::InstanceNewer,
    };
    Ok(StatusReport {
        instance: true,
        profile: Some(manifest.profile),
        docs_root: Some(manifest.docs_root),
        canon_version: Some(manifest.canon_version),
        binary_version: binary,
        alignment: Some(alignment),
        managed_drift: report.managed_drift,
        adopted_drift: report.adopted_drift,
        failures: report.failures,
        ok: Some(report.failures == 0),
    })
}
