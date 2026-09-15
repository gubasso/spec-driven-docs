//! Write one rendered candidate into a target, directly.
//!
//! Everything is decided before the first byte moves: every destination is
//! checked for containment, every whole file this tool would own is checked
//! for attribution, and the target lock is taken for the whole run. Then
//! each destination is replaced in its own directory, and the record is
//! written last.
//!
//! There is no journal and no rollback. A run that stops leaves whole
//! files, the previous record, a visible Git diff, and a command that runs
//! again. What it finished is reported, because that is what the operator
//! needs and what the next run reconciles against.

use camino::{Utf8Path, Utf8PathBuf};

use crate::candidate::{Candidate, Ownership};
use crate::domain::manifest::MANIFEST_PATH;
use crate::domain::ownership::Sha256;
use crate::domain::paths::PRUNABLE_ROOTS;
use crate::error::AppError;
use crate::transaction::stage::Stage;

/// What one landing did.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Every destination this run wrote, in the order it wrote them.
    pub written: Vec<Utf8PathBuf>,
    /// Every managed destination this release no longer owns, taken back.
    pub removed: Vec<String>,
}

/// What the target's own record says this tool owns today.
///
/// It decides three things: which whole files may be refreshed, which
/// marked regions may be re-spliced, and which files a release that
/// stopped landing them may take back.
#[derive(Debug, Default, Clone)]
pub struct Recorded {
    /// Each managed destination and the digest the record vouches for.
    pub managed: Vec<(String, Sha256)>,
    /// Each integration host and the hash of the region this tool owns.
    pub integration: Vec<(String, Sha256)>,
}

/// Write the candidate into the target, and record it last.
///
/// The caller holds the target lock for the whole of this, including the
/// observation it passes in: a candidate rendered before the lock would
/// describe a target somebody else could still be changing.
///
/// # Errors
///
/// [`AppError::Refused`] where a destination escapes the target or holds
/// bytes no record accounts for, [`AppError::Busy`] where another process
/// holds the target, and I/O errors of the writes themselves.
pub fn land(
    target: &Utf8Path,
    candidate: &Candidate,
    recorded: &Recorded,
) -> Result<Outcome, AppError> {
    contained(target, candidate)?;
    unattributed(target, candidate, recorded)?;
    let retired = retired(target, candidate, &recorded.managed)?;
    let mut outcome = Outcome::default();

    for destination in &candidate.destinations {
        let path = target.join(&destination.path);
        if std::fs::read(&path).is_ok_and(|held| held == destination.bytes) {
            continue;
        }
        if let Err(error) = write_one(target, &destination.path, &destination.bytes) {
            // A reported rename failure is re-observed before it is
            // believed. A remote filesystem may have completed the rename
            // it reported as failed, and a report that called that
            // destination unwritten would send the operator looking for
            // bytes that are already there.
            if std::fs::read(&path).is_ok_and(|held| held == destination.bytes) {
                outcome.written.push(destination.path.clone());
            }
            return Err(stopped(&error, &outcome));
        }
        outcome.written.push(destination.path.clone());
    }

    for (destination, vouched) in retired {
        let path = target.join(&destination);
        // Re-checked here, not only when the list was built: a component
        // swapped since then is refused rather than followed, and bytes
        // that changed since then are left alone. A removal is authorized
        // by the bytes the record holds, never by the path alone.
        if let Some((_, kind)) = escapes(target, Utf8Path::new(&destination)) {
            return Err(stopped(
                &AppError::Refused(format!("destination {kind}: {destination}")),
                &outcome,
            ));
        }
        match std::fs::read(&path) {
            Ok(held) if Sha256::of(&held) == vouched => {}
            // Gone already, or no longer the bytes the record vouches for.
            _ => continue,
        }
        match std::fs::remove_file(&path) {
            Ok(()) => {
                prune_empty(target, &destination);
                outcome.removed.push(destination);
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(stopped(&AppError::Io(source), &outcome)),
        }
    }

    // The record is last. Until it lands, the previous one describes the
    // target, which is what the next run reads.
    let record = candidate.manifest.to_json().into_bytes();
    if let Err(error) = write_one(target, Utf8Path::new(MANIFEST_PATH), &record) {
        if std::fs::read(target.join(MANIFEST_PATH)).is_ok_and(|held| held == record) {
            outcome.written.push(Utf8PathBuf::from(MANIFEST_PATH));
        }
        return Err(stopped(&error, &outcome));
    }
    outcome.written.push(Utf8PathBuf::from(MANIFEST_PATH));
    Ok(outcome)
}

/// Replace one destination in its own directory.
///
/// The chain is inspected again immediately before the rename, so a
/// component swapped between the check and the write is refused rather
/// than followed.
fn write_one(target: &Utf8Path, destination: &Utf8Path, bytes: &[u8]) -> Result<(), AppError> {
    let refuse = || {
        escapes(target, destination)
            .map(|(component, kind)| AppError::Refused(format!("{component} {kind}")))
    };
    // Before the scratch file is created, so nothing is written through a
    // component that already escapes.
    if let Some(refusal) = refuse() {
        return Err(refusal);
    }
    let path = target.join(destination);
    let scratch = Stage::write(&path, bytes)?;
    // And again before the rename, so a component swapped in between is
    // refused rather than followed.
    if let Some(refusal) = refuse() {
        Stage::discard(&scratch);
        return Err(refusal);
    }
    Stage::replace(&scratch, &path)
}

/// The refusal a run that stopped partway carries.
fn stopped(cause: &AppError, outcome: &Outcome) -> AppError {
    let mut finished: Vec<String> = Vec::new();
    if !outcome.written.is_empty() {
        let done: Vec<String> = outcome.written.iter().map(ToString::to_string).collect();
        finished.push(format!(
            "these destinations hold candidate bytes: {}",
            done.join(", ")
        ));
    }
    if !outcome.removed.is_empty() {
        finished.push(format!(
            "these destinations were removed: {}",
            outcome.removed.join(", ")
        ));
    }
    if finished.is_empty() {
        finished.push("nothing was written or removed".to_string());
    }
    AppError::Refused(format!(
        "the landing stopped: {cause}; {}, the previous record still stands, and running this again finishes the rest",
        finished.join("; ")
    ))
}

/// Refuse any destination that does not resolve beneath the target.
///
/// Each component is inspected without being followed, so a link standing
/// in the path cannot redirect a write outside the tree the operator named.
fn contained(target: &Utf8Path, candidate: &Candidate) -> Result<(), AppError> {
    let mut escaping: Vec<String> = Vec::new();
    let mut note = |found: Option<(Utf8PathBuf, &'static str)>| {
        if let Some((component, kind)) = found {
            let reason = format!("{component} {kind}");
            // One blocked directory holds many destinations, and naming it
            // once is what an operator can act on.
            if !escaping.contains(&reason) {
                escaping.push(reason);
            }
        }
    };
    for destination in &candidate.destinations {
        note(escapes(target, &destination.path));
    }
    note(escapes(target, Utf8Path::new(MANIFEST_PATH)));
    if escaping.is_empty() {
        return Ok(());
    }
    Err(AppError::Refused(format!(
        "the landing writes nothing: {}",
        escaping.join("; ")
    )))
}

/// Why one destination does not resolve beneath the target, if it does not.
fn escapes(target: &Utf8Path, destination: &Utf8Path) -> Option<(Utf8PathBuf, &'static str)> {
    if destination.is_absolute() {
        return Some((destination.to_owned(), "is absolute"));
    }
    if destination
        .components()
        .any(|part| part.as_str() == ".." || part.as_str() == ".")
    {
        return Some((destination.to_owned(), "climbs out of the target"));
    }
    let mut current = target.to_owned();
    let components: Vec<&str> = destination.as_str().split('/').collect();
    let last = components.len().saturating_sub(1);
    for (index, part) in components.iter().enumerate() {
        current = current.join(part);
        let Ok(held) = std::fs::symlink_metadata(&current) else {
            // Absent, so nothing below it exists to be redirected through.
            return None;
        };
        if held.file_type().is_symlink() {
            return Some((current, "escapes the target through a symlink"));
        }
        if index < last && !held.is_dir() {
            return Some((current, "is not a directory"));
        }
        if index == last && !held.is_file() {
            return Some((current, "is not a regular file"));
        }
    }
    None
}

/// Refuse a whole file this tool would own whose bytes no record vouches
/// for.
///
/// The record authorizes exact bytes, never a path. A destination whose
/// contents are not the ones the record holds is one somebody edited or
/// one somebody else wrote, and neither is this tool's to replace.
/// Missing provenance routes to the agent, never to an automatic
/// overwrite.
fn unattributed(
    target: &Utf8Path,
    candidate: &Candidate,
    recorded: &Recorded,
) -> Result<(), AppError> {
    let mut collisions: Vec<String> = Vec::new();
    for destination in &candidate.destinations {
        let path = target.join(&destination.path);
        let Ok(held) = std::fs::read(&path) else {
            continue;
        };
        if held == destination.bytes {
            continue;
        }
        let vouched = match destination.ownership {
            Ownership::Managed => recorded
                .managed
                .iter()
                .find(|(name, _)| name == destination.path.as_str())
                .is_some_and(|(_, digest)| digest == &Sha256::of(&held)),
            // A marked region sits in a file the project owns, so what the
            // record vouches for is the region rather than the file. Every
            // byte outside it is the project's and survives either way.
            Ownership::Integration => region_vouched(destination, &held, recorded),
            // An adopted destination is the project's from the moment it
            // lands, and the projection already kept what is there.
            Ownership::Adopted => true,
        };
        if vouched {
            continue;
        }
        collisions.push(destination.path.to_string());
    }
    if collisions.is_empty() {
        return Ok(());
    }
    Err(AppError::Refused(format!(
        "destinations hold bytes no record vouches for: {}; move them aside, or let the setup skill reconcile them",
        collisions.join(", ")
    )))
}

/// Whether the record vouches for the marked region a host file holds.
///
/// A host with no region yet is a first landing into a file the project
/// wrote, which the record cannot have an entry for and which the splice
/// leaves otherwise untouched.
fn region_vouched(
    destination: &crate::candidate::Destination,
    held: &[u8],
    recorded: &Recorded,
) -> bool {
    use crate::domain::marker;

    let Ok(text) = std::str::from_utf8(held) else {
        return false;
    };
    let hash = if destination.path == crate::domain::paths::HOOKS_CONFIG_PATH {
        marker::block_hash(text)
    } else {
        marker::block_hash_with(text, marker::AGENTS_BEGIN, marker::AGENTS_END)
    };
    let Some(hash) = hash else {
        return true;
    };
    recorded
        .integration
        .iter()
        .find(|(name, _)| name == destination.path.as_str())
        .is_some_and(|(_, recorded)| recorded == &hash)
}

/// Remove the directories one removal emptied, inside the roots this tool
/// owns.
///
/// A release that stops landing a whole package would otherwise leave its
/// directory behind, and an empty directory reads as something the tool
/// still owns.
fn prune_empty(target: &Utf8Path, destination: &str) {
    let mut parent = Utf8Path::new(destination).parent();
    while let Some(directory) = parent {
        if !PRUNABLE_ROOTS
            .iter()
            .any(|prefix| format!("{directory}/").starts_with(prefix))
        {
            return;
        }
        if std::fs::remove_dir(target.join(directory)).is_err() {
            return;
        }
        parent = directory.parent();
    }
}

/// Every managed destination the record holds that this release no longer
/// lands.
///
/// Only managed, and only under a root this tool owns. An adopted file a
/// release stops seeding stays: the project owns it from the moment it
/// lands, and a version moving is not permission to take it back. A file
/// whose bytes are not the ones the record vouches for stays too, because
/// somebody edited it and a removal would be a silent loss.
fn retired(
    target: &Utf8Path,
    candidate: &Candidate,
    recorded_managed: &[(String, Sha256)],
) -> Result<Vec<(String, Sha256)>, AppError> {
    let landing: Vec<&str> = candidate
        .destinations
        .iter()
        .map(|destination| destination.path.as_str())
        .collect();
    let mut retired = Vec::new();
    for (destination, recorded) in recorded_managed {
        if landing.contains(&destination.as_str()) {
            continue;
        }
        if !PRUNABLE_ROOTS
            .iter()
            .any(|prefix| destination.starts_with(prefix))
        {
            continue;
        }
        // A removal reached through a link would delete somebody else's
        // file, so it refuses rather than following it.
        if let Some((_, kind)) = escapes(target, Utf8Path::new(destination)) {
            return Err(AppError::Refused(format!(
                "destination {kind}: {destination}"
            )));
        }
        let Ok(held) = std::fs::read(target.join(destination)) else {
            continue;
        };
        if &Sha256::of(&held) != recorded {
            continue;
        }
        retired.push((destination.clone(), recorded.clone()));
    }
    Ok(retired)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;
    use crate::candidate::{Destination, Input, Placement, project};
    use crate::domain::profile::ProfileId;
    use crate::domain::version::CanonVersion;

    fn target(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap())
    }

    fn candidate() -> Candidate {
        project(&Input {
            profile: ProfileId::Codebase,
            version: CanonVersion::current(),
            installed_at: "2026-01-01T00:00:00Z".to_string(),
            docs_scratch: None,
            reserve: Vec::new(),
            writing_style: None,
            evidence: crate::candidate::Evidence::default(),
        })
        .unwrap()
    }

    #[test]
    fn a_landing_writes_every_destination_and_the_record_last() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let candidate = candidate();

        let outcome = land(&target, &candidate, &Recorded::default()).unwrap();

        assert_eq!(
            outcome.written.last().unwrap(),
            &Utf8PathBuf::from(MANIFEST_PATH)
        );
        for destination in &candidate.destinations {
            assert_eq!(
                std::fs::read(target.join(&destination.path)).unwrap(),
                destination.bytes,
                "{}",
                destination.path
            );
        }
    }

    #[test]
    fn a_second_landing_writes_nothing_and_still_records() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let candidate = candidate();
        land(&target, &candidate, &Recorded::default()).unwrap();

        let outcome = land(&target, &candidate, &Recorded::default()).unwrap();
        assert_eq!(outcome.written, vec![Utf8PathBuf::from(MANIFEST_PATH)]);
    }

    #[test]
    fn a_managed_destination_no_record_accounts_for_refuses_before_any_write() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let candidate = candidate();
        let managed = candidate
            .destinations
            .iter()
            .find(|destination| destination.ownership == Ownership::Managed)
            .unwrap();
        crate::adapters::fs::write_file(&target.join(&managed.path), b"somebody else wrote this")
            .unwrap();

        let error = land(&target, &candidate, &Recorded::default()).unwrap_err();
        assert!(error.to_string().contains(managed.path.as_str()), "{error}");
        // Nothing else landed: the refusal is before the first write.
        assert_eq!(
            std::fs::read(target.join(&managed.path)).unwrap(),
            b"somebody else wrote this"
        );
        assert!(!target.join(MANIFEST_PATH).exists());
    }

    #[test]
    fn a_managed_destination_edited_since_the_record_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let candidate = candidate();
        let managed = candidate
            .destinations
            .iter()
            .find(|destination| destination.ownership == Ownership::Managed)
            .unwrap()
            .clone();
        // The record names the path and vouches for other bytes, which is
        // what a local edit to a managed file looks like.
        crate::adapters::fs::write_file(&target.join(&managed.path), b"edited since").unwrap();
        let recorded = Recorded {
            managed: vec![(
                managed.path.to_string(),
                Sha256::of(b"what the record holds"),
            )],
            integration: Vec::new(),
        };

        let error = land(&target, &candidate, &recorded).unwrap_err();
        assert!(error.to_string().contains(managed.path.as_str()), "{error}");
        assert_eq!(
            std::fs::read(target.join(&managed.path)).unwrap(),
            b"edited since"
        );
        assert!(!target.join(MANIFEST_PATH).exists());
    }

    #[test]
    fn a_recorded_managed_destination_is_refreshed() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let candidate = candidate();
        let managed = candidate
            .destinations
            .iter()
            .find(|destination| destination.ownership == Ownership::Managed)
            .unwrap()
            .clone();
        crate::adapters::fs::write_file(&target.join(&managed.path), b"older").unwrap();
        // The record vouches for exactly the bytes that are there, which is
        // what an older landing of this tool leaves.
        let recorded = Recorded {
            managed: vec![(managed.path.to_string(), Sha256::of(b"older"))],
            integration: Vec::new(),
        };

        land(&target, &candidate, &recorded).unwrap();
        assert_eq!(
            std::fs::read(target.join(&managed.path)).unwrap(),
            managed.bytes
        );
    }

    #[test]
    fn a_managed_file_this_release_dropped_is_taken_back() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let dropped = ".spec-driven-docs/markdownlint/retired.jsonc";
        crate::adapters::fs::write_file(&target.join(dropped), b"old").unwrap();
        let recorded = Recorded {
            managed: vec![(dropped.to_string(), Sha256::of(b"old"))],
            integration: Vec::new(),
        };

        let outcome = land(&target, &candidate(), &recorded).unwrap();
        assert_eq!(outcome.removed, vec![dropped.to_string()]);
        assert!(!target.join(dropped).exists());
    }

    #[test]
    fn an_edited_file_this_release_dropped_stays() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let dropped = ".spec-driven-docs/markdownlint/retired.jsonc";
        crate::adapters::fs::write_file(&target.join(dropped), b"edited since").unwrap();
        let recorded = Recorded {
            managed: vec![(dropped.to_string(), Sha256::of(b"old"))],
            integration: Vec::new(),
        };

        let outcome = land(&target, &candidate(), &recorded).unwrap();
        assert!(outcome.removed.is_empty());
        assert!(target.join(dropped).exists());
    }

    #[test]
    fn a_destination_reached_through_a_link_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let outside = target.join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::create_dir_all(target.join(".spec-driven-docs")).unwrap();
        std::os::unix::fs::symlink(
            outside.as_std_path(),
            target.join(".spec-driven-docs/markdownlint").as_std_path(),
        )
        .unwrap();

        let error = land(&target, &candidate(), &Recorded::default()).unwrap_err();
        assert!(error.to_string().contains("symlink"), "{error}");
        assert!(!target.join(MANIFEST_PATH).exists());
    }

    #[test]
    fn a_destination_that_climbs_out_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let mut candidate = candidate();
        candidate.destinations.push(Destination {
            path: Utf8PathBuf::from("../escaped.md"),
            bytes: b"x".to_vec(),
            ownership: Ownership::Managed,
            placement: Placement::WholeFile,
            source: None,
        });

        let error = land(&target, &candidate, &Recorded::default()).unwrap_err();
        assert!(error.to_string().contains("climbs out"), "{error}");
    }
}
