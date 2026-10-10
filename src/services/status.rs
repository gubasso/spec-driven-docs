//! Report an instance's state as data.
//!
//! Status answers what the verifier's text answers, but as one typed
//! record a machine can branch on, and it treats a missing instance as an
//! answer rather than an error. A corrupt manifest still raises: absence
//! and breakage are different findings, and reporting a broken instance as
//! absent would invite a destructive re-init.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::domain::manifest::DOCS_SCRATCH_VAR;
use crate::domain::paths::{
    self, ActivePaths, Paths, UserEnv, docs_scratch_location, recorded_paths, variable,
};
use crate::domain::profile::{DocsRoot, ProfileId};
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::landing::classify::{Classification, Signals, classify};
use crate::services::verifier;

/// The machine schema this report declares.
///
/// Adding a field within a schema is additive. Removing one, renaming one,
/// changing one's type, or giving a field a value a reader of the old shape
/// would misread is the next schema, because a reader that branched on the
/// old shape cannot tell the two apart otherwise. Schema 4 added
/// `classification`, whose `needs-decision` no earlier reader knew. Schema
/// 5 added `migration`, and `classification` reads `migration` over a
/// landed instance while one is open, which no earlier reader expected.
pub const SCHEMA: &str = "sdd.status/5";

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
    /// The machine schema this object declares.
    pub schema: &'static str,
    /// What the target is, read from the target alone, or nothing where the
    /// instance is newer than this binary: an older binary cannot judge a
    /// record a newer release wrote, and `upgrade` would say it can.
    ///
    /// SATISFIES distribution:a-landing-classifies-its-target-first
    pub classification: Option<Classification>,
    /// Why the record or the declaration cannot be trusted, where the
    /// classification is `invalid`.
    pub invalid: Option<String>,
    /// Whether the target carries an instance.
    pub instance: bool,
    /// The installed profile.
    pub profile: Option<ProfileId>,
    /// The instance's documentation root.
    pub docs_root: Option<DocsRoot>,
    /// The docs scratch the instance records.
    pub docs_scratch: Option<Utf8PathBuf>,
    /// What `SDD_DOCS_SCRATCH` carries here, when it is set. The variable
    /// overrides the recorded value, so an audit needs both.
    pub docs_scratch_env: Option<String>,
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
    /// What an older release left under this user's state and cache roots.
    ///
    /// No verb of this release reads any of it, so each entry is a path an
    /// operator removes by hand once nothing of the older release is still
    /// running.
    pub legacy_state: Vec<LegacyState>,
    /// The migration the declaration holds open, by its id. While one is
    /// open, `classification` reads `migration`.
    ///
    /// SATISFIES setup:an-open-migration-is-declared
    pub migration: Option<String>,
    /// Every path this binary can name for this target and this user.
    ///
    /// A skill reads this section rather than spelling a path of its own.
    /// Every entry states what decided it, so a reader can tell a recorded
    /// answer from a derived one.
    pub paths: Paths,
}

/// What this target offers for the docs scratch.
fn proposals(target: &Utf8Path, env: &UserEnv) -> paths::Proposals {
    paths::proposals(env, |relative| target.join(relative).is_dir())
}

/// Every path, with no instance recorded.
fn derived_paths(target: &Utf8Path, env: &UserEnv) -> Paths {
    Paths {
        user: env.user_paths(),
        active: None,
        candidates: paths::candidates(),
        proposals: proposals(target, env),
    }
}

fn absent(target: &Utf8Path, classified: Classified) -> StatusReport {
    let env = UserEnv::from_process();
    StatusReport {
        schema: SCHEMA,
        classification: Some(classified.classification),
        invalid: classified.invalid,
        migration: classified.migration,
        legacy_state: legacy_state(&env),
        paths: derived_paths(target, &env),
        instance: false,
        profile: None,
        docs_root: None,
        docs_scratch: None,
        docs_scratch_env: variable(DOCS_SCRATCH_VAR),
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
        Err(AppError::ManifestMissing(_)) => return Ok(absent(target, classify_target(target)?)),
        Err(error) => return Err(error),
    };
    let classified = classify_target(target)?;
    let report = verifier::verify(target)?;
    let binary = CanonVersion::current();
    let alignment = match manifest.canon_version.cmp(&binary) {
        std::cmp::Ordering::Equal => Alignment::Aligned,
        std::cmp::Ordering::Less => Alignment::BinaryNewer,
        std::cmp::Ordering::Greater => Alignment::InstanceNewer,
    };
    let env = UserEnv::from_process();
    let mut resolved = derived_paths(target, &env);
    resolved.active = Some(ActivePaths {
        profile: manifest.profile,
        destinations: recorded_paths(manifest.docs_root),
        docs_scratch: docs_scratch_location(manifest.docs_scratch.as_deref(), &env),
    });
    Ok(StatusReport {
        schema: SCHEMA,
        classification: (alignment != Alignment::InstanceNewer)
            .then_some(classified.classification),
        invalid: classified.invalid,
        migration: classified.migration,
        legacy_state: legacy_state(&env),
        paths: resolved,
        instance: true,
        profile: Some(manifest.profile),
        docs_root: Some(manifest.docs_root),
        docs_scratch: manifest.docs_scratch,
        docs_scratch_env: variable(DOCS_SCRATCH_VAR),
        canon_version: Some(manifest.canon_version),
        binary_version: binary,
        alignment: Some(alignment),
        managed_drift: report.managed_drift,
        adopted_drift: report.adopted_drift,
        failures: report.failures,
        ok: Some(report.failures == 0),
    })
}

/// One directory or file an older release kept, and what it held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyState {
    /// Where it is.
    pub path: Utf8PathBuf,
    /// What it held, as one word.
    pub kind: &'static str,
    /// Whether it still binds a run: a plan the older release has not
    /// finished, or a journal of a run it did not finish. Only that release
    /// can finish or abandon such a run.
    pub active: bool,
}

/// What the releases that stored plans left under the user-scope roots.
///
/// The names are the ones those releases wrote, and this release writes
/// none of them. The list is closed: a path this tool never wrote is not
/// its to name.
const LEGACY_STATE: [(&str, &str); 3] = [
    ("plans", "plan-store"),
    ("backups", "transaction-backups"),
    ("skills.journal", "skill-journal"),
];

/// The bundle cache those releases kept under the cache root.
const LEGACY_CACHE: (&str, &str) = ("bundles", "bundle-cache");

/// Every legacy entry present on this host.
///
/// SATISFIES setup:status-reports-what-an-older-release-left
fn legacy_state(env: &UserEnv) -> Vec<LegacyState> {
    let mut found = Vec::new();
    if let Some(state) = env.state_root() {
        for (name, kind) in LEGACY_STATE {
            let path = state.path.join(name);
            if path.symlink_metadata().is_ok() {
                let active = match kind {
                    // A finished plan leaves the store with its result, so
                    // a plan still there is one nobody finished.
                    "plan-store" => std::fs::read_dir(path.join("plans"))
                        .is_ok_and(|mut plans| plans.any(|plan| plan.is_ok())),
                    // A run that finished removed its journal.
                    "skill-journal" => true,
                    _ => false,
                };
                found.push(LegacyState { path, kind, active });
            }
        }
    }
    if let Some(cache) = env.cache_root() {
        let path = cache.path.join(LEGACY_CACHE.0);
        if path.symlink_metadata().is_ok() {
            found.push(LegacyState {
                path,
                kind: LEGACY_CACHE.1,
                active: false,
            });
        }
    }
    found
}

/// One target's classification, and why it is `invalid` where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classified {
    /// What the target is.
    pub classification: Classification,
    /// Why the metadata cannot be trusted, where it cannot.
    pub invalid: Option<String>,
    /// The migration the declaration holds open, by its id.
    pub migration: Option<String>,
    /// What the target is beneath an open migration: the state its files
    /// alone show, which a landing verb serves while the migration is open.
    pub beneath: Classification,
}

/// Read what `target` is, from the target alone.
///
/// One rule serves every reader: the status report and each front verb.
/// With no instance, documents decide, not directories: a target with no
/// durable document beyond root metadata is a first landing, whatever
/// layout it started. Documents in a recognized home, by the rule
/// `sdd assess` applies, are a migration, and documents with no such home
/// are a decision the operator owes. With an instance, the record decides,
/// and a declaration that does not parse is metadata nobody can trust. A
/// path that is not a directory has nothing written in it, so it reads as
/// `setup`.
///
/// # Errors
///
/// [`AppError::Io`] when the walk cannot complete.
pub fn classify_target(target: &Utf8Path) -> Result<Classified, AppError> {
    if !target.is_dir() {
        return Ok(Classified {
            classification: Classification::Setup,
            invalid: None,
            migration: None,
            beneath: Classification::Setup,
        });
    }
    let observation = crate::landing::observe::observe(target)?;
    let invalid = match (&observation.invalid, &observation.installation) {
        (Some(reason), _) => Some(reason.clone()),
        // A declaration an archive wrote stands before any instance, so it
        // is judged whenever it exists: an id that is not a migration id is
        // an invalid declaration, never a migration nobody can find.
        (None, _) => crate::domain::instance_config::InstanceConfig::read(target)
            .err()
            .map(|error| error.to_string()),
    };
    let written =
        observation.installation.is_none() && invalid.is_none() && observation.corpus.settled();
    // Where the written documents belong is the assess rule's answer. A
    // target whose only documents sit in the docs scratch has written none
    // the convention keeps, so it reads as a first landing.
    let verdict = if written {
        Some(
            crate::services::assess::corpus(target, crate::gates::paths::docs_scratch_variable())?
                .verdict(),
        )
    } else {
        None
    };
    let homed = verdict == Some(crate::services::assess::Classification::Brownfield);
    let scattered = verdict == Some(crate::services::assess::Classification::NeedsDecision);
    let classification = classify(Signals {
        invalid: invalid.is_some(),
        installed: observation.installation.is_some(),
        // The destination is this binary's own release for every reader.
        at_destination: observation
            .installation
            .as_ref()
            .is_some_and(|installed| installed.canon_version == CanonVersion::current()),
        drifted: observation
            .installation
            .as_ref()
            .is_some_and(crate::landing::observe::Installation::drifted),
        settled: homed,
        scattered,
    });
    // An open migration is the state until it closes, whatever the files
    // beneath it show: the corpus sits in the archive, and the instance a
    // migration lands is the start of the rebuild, not its end.
    let migration = if invalid.is_none() {
        crate::domain::instance_config::InstanceConfig::read(target)
            .ok()
            .and_then(|declaration| declaration.migration)
    } else {
        None
    };
    Ok(Classified {
        classification: if migration.is_some() {
            Classification::Migration
        } else {
            classification
        },
        invalid,
        migration,
        beneath: classification,
    })
}
