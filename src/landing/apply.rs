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
    /// Every managed destination this release retired, deleted because
    /// its bytes were the recorded ones.
    pub removed: Vec<String>,
    /// Every managed destination this release dropped: edited since it
    /// landed, so it stays on disk, leaves the record, and is the
    /// project's from this landing.
    pub dropped: Vec<String>,
}

/// What a landing does with every managed destination the record names and
/// the candidate no longer lands.
///
/// SATISFIES staging:a-landing-retires-or-drops-what-it-stops-landing
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Departures {
    /// Each destination whose bytes match the recorded digest, with that
    /// digest. The landing deletes it.
    pub retired: Vec<(String, Sha256)>,
    /// Each destination whose bytes do not, or cannot be read. It stays.
    pub dropped: Vec<String>,
}

impl Departures {
    /// The retired paths and the dropped paths, as a report names them.
    #[must_use]
    pub fn into_paths(self) -> (Vec<String>, Vec<String>) {
        let retired = self.retired.into_iter().map(|(path, _)| path).collect();
        (retired, self.dropped)
    }
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
    /// Destinations the operator named for replacement, whatever they hold.
    ///
    /// The one authority beside the record. The caller saved each one's
    /// bytes before the landing, so a named destination is overwritten
    /// rather than refused.
    pub replace: Vec<String>,
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
    unattributed_packages(target, recorded)?;
    unattributed(target, candidate, recorded)?;
    let departures = departures(target, candidate, recorded);
    let mut outcome = Outcome {
        dropped: departures.dropped,
        ..Outcome::default()
    };

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

    for (destination, vouched) in departures.retired {
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
            // Gone already, or edited since the list was built: what stays
            // is the project's, as every dropped destination is.
            Ok(_) => {
                outcome.dropped.push(destination);
                continue;
            }
            Err(_) => continue,
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

/// Why the landing would refuse one destination the operator names, where
/// it would.
///
/// A name answers a conflict, and every conflict is a destination the
/// candidate writes, so it must resolve beneath the target.
#[must_use]
pub(crate) fn refuses_named(
    target: &Utf8Path,
    candidate: &Candidate,
    destination: &str,
) -> Option<(Utf8PathBuf, &'static str)> {
    candidate
        .destinations
        .iter()
        .any(|landed| landed.path.as_str() == destination)
        .then(|| escapes(target, Utf8Path::new(destination)))
        .flatten()
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
        if vouches(recorded, &destination.path, destination.ownership, &held)
            || recorded
                .replace
                .iter()
                .any(|named| named == destination.path.as_str())
        {
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

/// Every skill package folder the landing must leave alone, as
/// `<root>/<name>/`.
///
/// The one package-level ownership check. A folder the inventory lands
/// into is refused whole where it holds anything and the record
/// attributes none of its files: an extra file, or bytes equal to the
/// candidate's, still came from somebody else. The landing, the conflict
/// scan of `sdd upgrade`, and `sdd stage compare` all read this answer, so
/// a preview and an apply agree.
///
/// SATISFIES distribution:an-unattributed-skill-folder-is-never-replaced
#[must_use]
pub fn refused_skill_packages(target: &Utf8Path, recorded: &Recorded) -> Vec<String> {
    crate::embedded::skill_package_folders()
        .into_iter()
        .filter_map(|folder| {
            let prefix = format!("{folder}/");
            let holds = std::fs::read_dir(target.join(&folder))
                .is_ok_and(|mut entries| entries.next().is_some());
            let attributed = recorded
                .managed
                .iter()
                .any(|(destination, _)| destination.starts_with(&prefix));
            (holds && !attributed).then_some(prefix)
        })
        .collect()
}

/// The line that names one refused package folder.
#[must_use]
pub fn refused_package_line(folder: &str) -> String {
    format!(
        "{folder} holds a skill package no record attributes; sdd lands its own copy there, so move the folder aside and run this again (distribution:an-unattributed-skill-folder-is-never-replaced)"
    )
}

/// Refuse every skill package folder [`refused_skill_packages`] names.
fn unattributed_packages(target: &Utf8Path, recorded: &Recorded) -> Result<(), AppError> {
    let refused = refused_skill_packages(target, recorded);
    if refused.is_empty() {
        return Ok(());
    }
    Err(AppError::Refused(
        refused
            .iter()
            .map(|folder| refused_package_line(folder))
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

/// Whether the record vouches for the bytes one destination holds.
///
/// The one attribution check. The landing refuses a destination this
/// answers no for, and `sdd stage compare` reports the same answer as the
/// record half of a destination's provenance, so the two cannot disagree.
#[must_use]
pub fn vouches(recorded: &Recorded, path: &Utf8Path, ownership: Ownership, held: &[u8]) -> bool {
    match ownership {
        Ownership::Managed => recorded
            .managed
            .iter()
            .find(|(name, _)| name == path.as_str())
            .is_some_and(|(_, digest)| digest == &Sha256::of(held)),
        // A marked region sits in a file the project owns, so what the
        // record vouches for is the region rather than the file. Every
        // byte outside it is the project's and survives either way.
        Ownership::Integration => region_vouched(path, held, recorded),
        // An adopted destination is the project's from the moment it
        // lands, and the projection already kept what is there.
        Ownership::Adopted => true,
    }
}

/// Whether the record vouches for the marked region a host file holds.
///
/// A host with no region yet is a first landing into a file the project
/// wrote, which the record cannot have an entry for and which the splice
/// leaves otherwise untouched.
fn region_vouched(path: &Utf8Path, held: &[u8], recorded: &Recorded) -> bool {
    use crate::domain::marker;

    let Ok(text) = std::str::from_utf8(held) else {
        return false;
    };
    let hash = if path == crate::domain::paths::HOOKS_CONFIG_PATH {
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
        .find(|(name, _)| name == path.as_str())
        .is_some_and(|(_, recorded)| recorded == &hash)
}

/// Remove the directories one removal emptied, inside the roots this tool
/// owns.
///
/// A release that stops landing a whole package would otherwise leave its
/// directory behind, and an empty directory reads as something the tool
/// still owns. A directory outside those roots is the project's, emptied
/// or not, so it stays.
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

/// Sort every managed destination the record holds that this release no
/// longer lands into the retired and the dropped.
///
/// Wherever it lies in the target. A file whose bytes are the ones the
/// record vouches for is retired, and the landing deletes it. Any other is
/// dropped: somebody edited it, so a deletion would be a silent loss, and
/// it stays as the project's own. A destination already gone is neither.
/// An adopted file a release stops seeding is never listed: the project
/// owns it from the moment it lands.
///
/// A destination reached through a link, or otherwise outside the target,
/// is dropped too: a deletion through it could remove somebody else's
/// file, so it is never followed and never deleted.
///
/// The one sort. The landing, the preview of `sdd upgrade`, `sdd stage`,
/// and `sdd stage compare` all read it, so each names what the apply does.
#[must_use]
pub fn departures(target: &Utf8Path, candidate: &Candidate, recorded: &Recorded) -> Departures {
    let landing: Vec<&str> = candidate
        .destinations
        .iter()
        .map(|destination| destination.path.as_str())
        .collect();
    departures_from(target, &landing, recorded)
}

/// [`departures`], where the caller holds the paths a candidate lands
/// rather than the candidate, as a stage does.
#[must_use]
pub fn departures_from(target: &Utf8Path, landing: &[&str], recorded: &Recorded) -> Departures {
    let mut departures = Departures::default();
    for (destination, digest) in &recorded.managed {
        if landing.contains(&destination.as_str()) {
            continue;
        }
        if escapes(target, Utf8Path::new(destination)).is_some() {
            departures.dropped.push(destination.clone());
            continue;
        }
        match std::fs::read(target.join(destination)) {
            Ok(held) if &Sha256::of(&held) == digest => {
                departures
                    .retired
                    .push((destination.clone(), digest.clone()));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => departures.dropped.push(destination.clone()),
        }
    }
    departures
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

    /// VERIFIES distribution:an-unattributed-skill-folder-is-never-replaced
    #[test]
    fn the_landing_names_every_unattributed_package_folder() {
        let candidate = candidate();
        let landed: Vec<&Destination> = candidate
            .destinations
            .iter()
            .filter(|destination| destination.path.starts_with(".claude/skills/sdd-setup"))
            .collect();
        let cases: [(&str, &str); 3] = [
            (".claude/skills/sdd-setup/SKILL.md", "full"),
            (".claude/skills/sdd-setup/extra.txt", "extra"),
            ("", "identical"),
        ];
        for (path, case) in cases {
            let dir = tempfile::tempdir().unwrap();
            let target = target(&dir);
            if path.is_empty() {
                for destination in &landed {
                    crate::adapters::fs::write_file(
                        &target.join(&destination.path),
                        &destination.bytes,
                    )
                    .unwrap();
                }
            } else {
                crate::adapters::fs::write_file(&target.join(path), b"somebody else's\n").unwrap();
            }
            let error = land(&target, &candidate, &Recorded::default()).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(".claude/skills/sdd-setup/ holds a skill package"),
                "{case}: {error}"
            );
            assert!(
                !target.join(crate::domain::manifest::MANIFEST_PATH).exists(),
                "{case}: the refusal wrote the record"
            );
        }
    }

    #[test]
    fn a_second_landing_writes_nothing_and_still_records() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let candidate = candidate();
        land(&target, &candidate, &Recorded::default()).unwrap();

        // The second landing reads the record the first one wrote, as every
        // caller does; that record attributes the landed skill packages.
        let recorded = crate::services::installer::recorded(&target);
        let outcome = land(&target, &candidate, &recorded).unwrap();
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
            replace: Vec::new(),
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
            replace: Vec::new(),
        };

        land(&target, &candidate, &recorded).unwrap();
        assert_eq!(
            std::fs::read(target.join(&managed.path)).unwrap(),
            managed.bytes
        );
    }

    /// VERIFIES staging:a-landing-retires-or-drops-what-it-stops-landing
    #[test]
    fn an_unedited_managed_file_this_release_stops_landing_is_retired() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let retired = ".spec-driven-docs/markdownlint/retired.jsonc";
        crate::adapters::fs::write_file(&target.join(retired), b"old").unwrap();
        let recorded = Recorded {
            managed: vec![(retired.to_string(), Sha256::of(b"old"))],
            integration: Vec::new(),
            replace: Vec::new(),
        };

        let outcome = land(&target, &candidate(), &recorded).unwrap();
        assert_eq!(outcome.removed, vec![retired.to_string()]);
        assert!(outcome.dropped.is_empty());
        assert!(!target.join(retired).exists());
        assert!(
            !target.join(".spec-driven-docs/markdownlint").exists(),
            "the emptied directory inside the instance stayed"
        );
    }

    /// VERIFIES staging:a-landing-retires-or-drops-what-it-stops-landing
    ///
    /// Wherever it lies: a retired file outside the roots this tool owns
    /// whole is deleted too, and the directory it leaves is the project's.
    #[test]
    fn an_unedited_retired_file_outside_the_owned_roots_is_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let retired = "docs/lint/retired.jsonc";
        crate::adapters::fs::write_file(&target.join(retired), b"old").unwrap();
        let recorded = Recorded {
            managed: vec![(retired.to_string(), Sha256::of(b"old"))],
            integration: Vec::new(),
            replace: Vec::new(),
        };

        let outcome = land(&target, &candidate(), &recorded).unwrap();
        assert_eq!(outcome.removed, vec![retired.to_string()]);
        assert!(!target.join(retired).exists());
        assert!(
            target.join("docs/lint").is_dir(),
            "a project directory went"
        );
    }

    /// VERIFIES staging:a-landing-retires-or-drops-what-it-stops-landing
    #[test]
    fn an_edited_file_this_release_stops_landing_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let inside = ".spec-driven-docs/markdownlint/retired.jsonc";
        let outside = "docs/lint/retired.jsonc";
        let gone = ".claude/skills/old/SKILL.md";
        for path in [inside, outside] {
            crate::adapters::fs::write_file(&target.join(path), b"edited since").unwrap();
        }
        let recorded = Recorded {
            managed: vec![
                (inside.to_string(), Sha256::of(b"old")),
                (outside.to_string(), Sha256::of(b"old")),
                (gone.to_string(), Sha256::of(b"old")),
            ],
            integration: Vec::new(),
            replace: Vec::new(),
        };

        let outcome = land(&target, &candidate(), &recorded).unwrap();
        assert!(outcome.removed.is_empty());
        assert_eq!(
            outcome.dropped,
            vec![inside.to_string(), outside.to_string()]
        );
        for path in [inside, outside] {
            assert_eq!(std::fs::read(target.join(path)).unwrap(), b"edited since");
        }
        let record = std::fs::read_to_string(target.join(MANIFEST_PATH)).unwrap();
        for path in [inside, outside, gone] {
            assert!(!record.contains(path), "{path} stayed in the record");
        }
    }

    #[test]
    fn a_named_destination_is_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let candidate = candidate();
        let managed = candidate
            .destinations
            .iter()
            .find(|destination| destination.ownership == Ownership::Managed)
            .unwrap()
            .clone();
        crate::adapters::fs::write_file(&target.join(&managed.path), b"edited since").unwrap();
        let recorded = Recorded {
            managed: vec![(managed.path.to_string(), Sha256::of(b"older"))],
            integration: Vec::new(),
            replace: vec![managed.path.to_string()],
        };
        assert!(!vouches(
            &recorded,
            &managed.path,
            Ownership::Managed,
            b"edited since"
        ));

        land(&target, &candidate, &recorded).unwrap();
        assert_eq!(
            std::fs::read(target.join(&managed.path)).unwrap(),
            managed.bytes
        );
    }

    #[test]
    fn a_destination_reached_through_a_link_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let target = target(&dir);
        let outside = target.join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(outside.as_std_path(), target.join("docs").as_std_path())
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
