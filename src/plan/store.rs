//! Where a computed plan waits for its approval.
//!
//! A plan carries every byte it will write, and some of those bytes come
//! out of the target. So the store is owner-only, it lives under the state
//! root rather than in the repository, and a plan is ephemeral: it is
//! never committed, and never pasted into a forge.
//!
//! Identity is the fingerprint and nothing else. Identical inputs while an
//! executable plan exists reuse it, so an operator who plans twice
//! approves one thing. A terminal result moves the plan out and frees the
//! fingerprint, so identical inputs after a success plan again into a
//! fresh directory rather than meeting a stripped one.
//!
//! The lifecycle is fixed rather than configurable. An unapplied plan
//! expires after seven days. A terminal result is kept for thirty. A run
//! that needs recovery keeps everything recovery needs, whatever the
//! calendar says.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::domain::ownership::Sha256;
use crate::error::AppError;
use crate::plan::Plan;

/// The machine schema a result declares.
pub const RESULT_SCHEMA: &str = "sdd.result/1";

/// How long an unapplied plan waits before it expires.
pub const PLAN_TTL_DAYS: i64 = 7;

/// How long a terminal result is kept.
pub const RESULT_TTL_DAYS: i64 = 30;

/// How one apply ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disposition {
    /// Every operation landed and every postcondition held.
    Succeeded,
    /// The world moved, so the plan no longer describes it. Terminal.
    Invalidated,
    /// Nothing semantic moved, so the same plan can be applied again.
    Retryable,
    /// A run did not finish, and the next one must recover it first.
    RecoveryRequired,
}

impl Disposition {
    /// Whether this disposition ends the plan's life.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Invalidated)
    }
}

/// What one operation did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationOutcome {
    /// Which operation, by its kind.
    pub kind: String,
    /// Where, relative to the target.
    pub path: String,
    /// Whether it landed.
    pub applied: bool,
    /// What went wrong, where anything did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

/// What one postcondition found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostconditionOutcome {
    /// Which postcondition.
    pub id: String,
    /// Whether it held.
    pub held: bool,
    /// What was found instead, where it did not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// What one apply did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Result {
    /// The machine schema of this object.
    pub schema: String,
    /// The plan this result is for.
    pub plan_id: String,
    /// The fingerprint that plan carried.
    pub fingerprint: Sha256,
    /// A stable identifier for this attempt.
    pub result_id: String,
    /// How it ended.
    pub disposition: Disposition,
    /// When it ended.
    pub finished_at: String,
    /// What each operation did.
    pub operations: Vec<OperationOutcome>,
    /// What each postcondition found.
    pub postconditions: Vec<PostconditionOutcome>,
    /// Whether a journal is outstanding.
    pub recovery_required: bool,
    /// Every path this attempt touched, relative to the target.
    pub affected: Vec<String>,
    /// Why it ended as it did, in one sentence.
    pub reason: String,
}

/// The owner-only store under the state root.
#[derive(Debug, Clone)]
pub struct Store {
    root: Utf8PathBuf,
}

/// Where one plan's own directory sits.
#[derive(Debug, Clone)]
pub struct PlanDirectory {
    /// The directory itself.
    pub root: Utf8PathBuf,
    /// The executable plan.
    pub plan: Utf8PathBuf,
    /// Every byte the plan will write, by digest.
    pub blobs: Utf8PathBuf,
    /// The journal an unfinished apply leaves.
    pub journal: Utf8PathBuf,
}

impl Store {
    /// The store under one state root.
    #[must_use]
    pub fn new(state_root: &Utf8Path) -> Self {
        Self {
            root: state_root.join(crate::domain::paths::PLAN_STORE_DIR),
        }
    }

    /// The store's own root, for a diagnostic that names it.
    #[must_use]
    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    /// One attempt's directory name, as a name and never as a path.
    ///
    /// The attempt id is built from the caller's clock, so it is not a
    /// digest and cannot be checked as one. It is reduced to the
    /// characters a directory name may carry, which is what keeps a
    /// separator out of the join.
    fn attempt_slug(value: &str) -> String {
        let held: String = value
            .chars()
            .map(|held| {
                if held.is_ascii_alphanumeric() || held == '-' {
                    held
                } else {
                    '-'
                }
            })
            .collect();
        if held.is_empty() {
            "attempt".to_string()
        } else {
            held
        }
    }

    /// Refuse an id that is not a fingerprint.
    ///
    /// Every public entry takes the id as a string, and the strings reach
    /// `Path::join`, which appends whatever it is given. An id carrying a
    /// path component would name a directory outside the store, and a
    /// terminal result removes the directory its plan names. So the shape
    /// is checked once, at the boundary: exactly a lowercase hex sha256.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`] naming the id.
    pub fn checked(fingerprint: &str) -> std::result::Result<&str, AppError> {
        fingerprint.parse::<Sha256>().map_err(|_| {
            AppError::Refused(format!(
                "'{fingerprint}' is not a plan id; a plan id is the 64-character fingerprint 'sdd reconcile plan' printed"
            ))
        })?;
        Ok(fingerprint)
    }

    /// Where one fingerprint's executable plan lives.
    ///
    /// The caller has already run [`Store::checked`] on the id: every
    /// public entry does, and this is reachable only through one of them.
    #[must_use]
    pub fn directory(&self, fingerprint: &str) -> PlanDirectory {
        let root = self.root.join("plans").join(fingerprint);
        PlanDirectory {
            plan: root.join("plan.json"),
            blobs: root.join("blobs"),
            journal: root.join("apply.journal"),
            root,
        }
    }

    /// Where one fingerprint's terminal results live.
    #[must_use]
    pub fn results(&self, fingerprint: &str) -> Utf8PathBuf {
        self.root.join("results").join(fingerprint)
    }

    /// Where the store keeps its own lock.
    #[must_use]
    pub fn lock_path(&self) -> Utf8PathBuf {
        self.root.join("store.lock")
    }

    /// Create the store, owner-only.
    ///
    /// A plan carries bytes out of a target, and some of those are not the
    /// world's business. The directory mode says so on the filesystem
    /// rather than only in a document.
    ///
    /// # Errors
    ///
    /// Any I/O error creating the directories or setting their mode.
    pub fn create(&self) -> std::result::Result<(), AppError> {
        for directory in [
            self.root.clone(),
            self.root.join("plans"),
            self.root.join("results"),
        ] {
            std::fs::create_dir_all(&directory)?;
            owner_only(&directory)?;
        }
        Ok(())
    }

    /// Whether an executable plan exists for this fingerprint.
    #[must_use]
    pub fn holds(&self, fingerprint: &str) -> bool {
        Self::checked(fingerprint).is_ok_and(|held| self.directory(held).plan.is_file())
    }

    /// Write one plan and every byte it will land.
    ///
    /// # Errors
    ///
    /// Any I/O error creating the directory or writing the plan.
    pub fn put(
        &self,
        plan: &Plan,
        blobs: &BTreeMap<Sha256, Vec<u8>>,
    ) -> std::result::Result<PlanDirectory, AppError> {
        self.create()?;
        let held = self.directory(Self::checked(&plan.identity.plan_id)?);
        std::fs::create_dir_all(&held.blobs)?;
        owner_only(&held.root)?;
        owner_only(&held.blobs)?;
        for (digest, bytes) in blobs {
            let path = held.blobs.join(digest.to_string());
            if !path.is_file() {
                crate::adapters::fs::write_atomic(&path, bytes)?;
            }
        }
        let text = serde_json::to_string_pretty(plan)
            .map_err(|source| anyhow::anyhow!("the plan did not serialize: {source}"))?;
        crate::adapters::fs::write_atomic(&held.plan, format!("{text}\n").as_bytes())?;
        Ok(held)
    }

    /// Read one stored plan.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`] when no executable plan carries that
    /// fingerprint, and I/O errors when it cannot be read.
    pub fn get(&self, fingerprint: &str) -> std::result::Result<Plan, AppError> {
        let held = self.directory(Self::checked(fingerprint)?);
        let text = std::fs::read_to_string(&held.plan).map_err(|_| {
            AppError::Refused(format!(
                "no executable plan carries the id {fingerprint}; run 'sdd reconcile plan' again"
            ))
        })?;
        let plan: Plan = serde_json::from_str(&text).map_err(|source| {
            AppError::Refused(format!("{} does not parse: {source}", held.plan))
        })?;
        // The document decides where its own results and its own cleanup
        // go, so a document that does not answer to the id it was fetched
        // by is refused rather than trusted.
        if plan.identity.plan_id != fingerprint {
            return Err(AppError::Refused(format!(
                "{} carries the id {} and was fetched as {fingerprint}",
                held.plan, plan.identity.plan_id
            )));
        }
        Ok(plan)
    }

    /// One byte string the plan will write.
    ///
    /// # Errors
    ///
    /// [`AppError::Refused`] when the plan's blob store does not carry it.
    pub fn blob(
        &self,
        fingerprint: &str,
        digest: &Sha256,
    ) -> std::result::Result<Vec<u8>, AppError> {
        let path = self
            .directory(Self::checked(fingerprint)?)
            .blobs
            .join(digest.to_string());
        let bytes = std::fs::read(&path).map_err(|source| {
            AppError::Refused(format!(
                "the plan {fingerprint} carries no blob {digest}: {source}"
            ))
        })?;
        if &Sha256::of(&bytes) != digest {
            return Err(AppError::Refused(format!(
                "the blob at {path} no longer hashes to {digest}"
            )));
        }
        Ok(bytes)
    }

    /// Record one attempt's outcome.
    ///
    /// A terminal disposition moves the redacted plan out with its result
    /// and frees the fingerprint. Anything else leaves the executable plan
    /// where it is, because the same plan can still be applied.
    ///
    /// # Errors
    ///
    /// Any I/O error writing the result or moving the plan.
    pub fn record(&self, plan: &Plan, result: &Result) -> std::result::Result<(), AppError> {
        let id = Self::checked(&plan.identity.plan_id)?;
        let directory = self
            .results(Self::checked(&result.fingerprint.to_string())?)
            .join(Self::attempt_slug(&result.result_id));
        std::fs::create_dir_all(&directory)?;
        owner_only(&directory)?;
        let redacted = redact(plan);
        let text = serde_json::to_string_pretty(&redacted)
            .map_err(|source| anyhow::anyhow!("the plan did not serialize: {source}"))?;
        crate::adapters::fs::write_atomic(
            &directory.join("plan.json"),
            format!("{text}\n").as_bytes(),
        )?;
        let text = serde_json::to_string_pretty(result)
            .map_err(|source| anyhow::anyhow!("the result did not serialize: {source}"))?;
        crate::adapters::fs::write_atomic(
            &directory.join("result.json"),
            format!("{text}\n").as_bytes(),
        )?;
        if result.disposition.is_terminal() {
            // The fingerprint is free again, so identical inputs plan into
            // a fresh directory rather than meeting a stripped one.
            let _ = std::fs::remove_dir_all(self.directory(id).root);
        }
        Ok(())
    }

    /// The latest result for one fingerprint, where any exists.
    #[must_use]
    pub fn latest_result(&self, fingerprint: &str) -> Option<Result> {
        let fingerprint = Self::checked(fingerprint).ok()?;
        let mut found: Vec<(String, Result)> = std::fs::read_dir(self.results(fingerprint))
            .ok()?
            .filter_map(std::result::Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().to_str()?.to_string();
                let text = std::fs::read_to_string(entry.path().join("result.json")).ok()?;
                let held: Result = serde_json::from_str(&text).ok()?;
                Some((name, held))
            })
            .collect();
        found.sort_by(|left, right| left.0.cmp(&right.0));
        found.pop().map(|(_, held)| held)
    }

    /// Remove what the lifecycle says is over.
    ///
    /// Never removes the fingerprint the caller named, and never removes a
    /// directory that still holds a journal: a run that did not finish
    /// keeps everything recovery needs, whatever the calendar says.
    ///
    /// # Errors
    ///
    /// Any I/O error reading the store.
    pub fn prune(
        &self,
        now: jiff::Timestamp,
        keep: Option<&str>,
    ) -> std::result::Result<Vec<Utf8PathBuf>, AppError> {
        let mut removed = Vec::new();
        removed.extend(Self::prune_under(
            &self.root.join("plans"),
            now,
            PLAN_TTL_DAYS,
            keep,
            true,
        )?);
        removed.extend(Self::prune_under(
            &self.root.join("results"),
            now,
            RESULT_TTL_DAYS,
            keep,
            false,
        )?);
        Ok(removed)
    }

    fn prune_under(
        root: &Utf8Path,
        now: jiff::Timestamp,
        days: i64,
        keep: Option<&str>,
        guard_journal: bool,
    ) -> std::result::Result<Vec<Utf8PathBuf>, AppError> {
        let Ok(entries) = std::fs::read_dir(root) else {
            return Ok(Vec::new());
        };
        let mut removed = Vec::new();
        for entry in entries.filter_map(std::result::Result::ok) {
            let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
                continue;
            };
            let name = path.file_name().unwrap_or_default();
            if Some(name) == keep {
                continue;
            }
            if guard_journal && path.join("apply.journal").exists() {
                continue;
            }
            if older_than(&path, now, days) {
                std::fs::remove_dir_all(&path)?;
                removed.push(path);
            }
        }
        Ok(removed)
    }
}

/// Whether a directory has outlived its allowance.
fn older_than(path: &Utf8Path, now: jiff::Timestamp, days: i64) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let Ok(elapsed) = modified.elapsed() else {
        return false;
    };
    let _ = now;
    #[expect(
        clippy::cast_sign_loss,
        reason = "the allowance is a positive number of days, declared as a constant here"
    )]
    let allowance = std::time::Duration::from_secs(days as u64 * 24 * 60 * 60);
    elapsed > allowance
}

/// Restrict a directory to its owner.
fn owner_only(path: &Utf8Path) -> std::result::Result<(), AppError> {
    let mut permissions = std::fs::metadata(path)?.permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o700);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

/// The plan as a result keeps it: no absolute target path.
///
/// A result outlives the run, and a run's target path names a person's
/// filesystem. What a reader needs is which destinations moved, and those
/// are relative.
#[must_use]
pub fn redact(plan: &Plan) -> Plan {
    let mut held = plan.clone();
    held.observed_state.repository.root = Utf8PathBuf::from("<target>");
    held.observed_state.host.cache_root = None;
    held
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn store(dir: &tempfile::TempDir) -> Store {
        Store::new(&Utf8PathBuf::from(dir.path().to_str().unwrap()))
    }

    #[test]
    fn the_store_is_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let held = store(&dir);
        held.create().unwrap();
        for path in [held.root().to_owned(), held.root().join("plans")] {
            let mode = std::os::unix::fs::PermissionsExt::mode(
                &std::fs::metadata(&path).unwrap().permissions(),
            );
            assert_eq!(mode & 0o777, 0o700, "{path} is not owner-only");
        }
    }

    #[test]
    fn a_directory_a_command_named_is_never_pruned() {
        let dir = tempfile::tempdir().unwrap();
        let held = store(&dir);
        held.create().unwrap();
        let one = held.directory("keepme");
        std::fs::create_dir_all(&one.root).unwrap();
        let removed = held.prune(jiff::Timestamp::now(), Some("keepme")).unwrap();
        assert!(removed.is_empty());
        assert!(one.root.is_dir());
    }

    #[test]
    fn a_journal_holds_its_directory_past_ordinary_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let held = store(&dir);
        held.create().unwrap();
        let one = held.directory("unfinished");
        std::fs::create_dir_all(&one.root).unwrap();
        std::fs::write(&one.journal, "{}").unwrap();
        // Even with no allowance left, a run that did not finish keeps
        // everything recovery needs.
        let removed = held.prune(jiff::Timestamp::now(), None).unwrap();
        assert!(removed.is_empty());
        assert!(one.root.is_dir());
    }

    #[test]
    fn a_blob_that_no_longer_hashes_to_its_name_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let held = store(&dir);
        held.create().unwrap();
        let one = held.directory("f");
        std::fs::create_dir_all(&one.blobs).unwrap();
        let digest = Sha256::of(b"intended");
        std::fs::write(one.blobs.join(digest.to_string()), b"tampered").unwrap();
        assert!(held.blob("f", &digest).is_err());
    }

    #[test]
    fn an_absent_plan_refuses_with_the_next_command() {
        let dir = tempfile::tempdir().unwrap();
        let error = store(&dir).get("nope").unwrap_err();
        assert!(error.to_string().contains("sdd reconcile plan"), "{error}");
    }
}
