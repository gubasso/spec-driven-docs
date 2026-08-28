//! Install the embedded skills into agent skill directories.
//!
//! The destinations live outside any instance — under the invoking user's
//! home — so nothing here touches an instance manifest. Two references
//! decide what a destination holds: the embedded payload, and the record of
//! what a previous apply wrote there. Bytes matching either are the tool's
//! own and may be replaced; anything else is the user's and refuses.
//!
//! The preview-by-default, list-every-conflict, and restore-on-failure
//! conventions mirror the instance installer, and for the same reason: a
//! partial apply across two roots leaves an agent reading one version of a
//! skill and another agent reading a different one.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::ownership::Sha256;
use crate::domain::skill_record::SkillRecord;
use crate::error::AppError;

/// One planned write: where, and which bytes.
struct Planned {
    destination: Utf8PathBuf,
    bytes: &'static [u8],
}

fn plan(roots: &[Utf8PathBuf]) -> Result<Vec<Planned>, AppError> {
    let mut planned = Vec::new();
    for root in roots {
        for name in crate::embedded::skill_names() {
            let text = crate::embedded::skill(name)
                .ok_or_else(|| anyhow::anyhow!("payload skill missing: {name}"))?;
            planned.push(Planned {
                destination: root.join(name).join("SKILL.md"),
                bytes: text.as_bytes(),
            });
        }
    }
    Ok(planned)
}

fn check_destination(destination: &Utf8Path) -> Result<(), AppError> {
    if destination.is_symlink() {
        return Err(AppError::Refused(format!(
            "destination is a symlink: {destination}"
        )));
    }
    if destination.exists() && !destination.is_file() {
        return Err(AppError::Refused(format!(
            "destination exists and is not a regular file: {destination}"
        )));
    }
    Ok(())
}

/// Destinations holding bytes neither the payload nor the record accounts for.
///
/// A destination the record vouches for carries a copy this tool wrote and a
/// later release has since changed. That is an upgrade, not a conflict, and
/// naming it one would make every skill-touching release refuse on files
/// nobody edited.
fn conflicts(planned: &[Planned], record: &SkillRecord) -> Result<Vec<String>, AppError> {
    let mut conflicts = Vec::new();
    for entry in planned {
        if !entry.destination.is_file() {
            continue;
        }
        // An unreadable destination raises instead of passing as clean: a
        // comparison that cannot run must never license an overwrite.
        let found = std::fs::read(&entry.destination)?;
        if found == entry.bytes {
            continue;
        }
        if record.wrote(&entry.destination, &Sha256::of(&found)) {
            continue;
        }
        conflicts.push(entry.destination.to_string());
    }
    Ok(conflicts)
}

/// Restore every backed-up destination, returning those that would not go back.
fn rollback(backups: &BTreeMap<Utf8PathBuf, Option<Vec<u8>>>) -> Vec<Utf8PathBuf> {
    let mut unrestored = Vec::new();
    for (destination, previous) in backups {
        let restored = previous.as_ref().map_or_else(
            || std::fs::remove_file(destination).is_ok() || !destination.exists(),
            |bytes| crate::adapters::fs::write_file(destination, bytes).is_ok(),
        );
        if !restored {
            unrestored.push(destination.clone());
        }
    }
    unrestored
}

/// The refusal a failed apply carries, naming the cause and what it restored.
fn abort(unrestored: &[Utf8PathBuf], cause: &str) -> AppError {
    if unrestored.is_empty() {
        AppError::Refused(format!(
            "skill install aborted; the destinations were restored: {cause}"
        ))
    } else {
        let paths: Vec<&str> = unrestored.iter().map(|p| p.as_str()).collect();
        AppError::Refused(format!(
            "skill install aborted and restoration is incomplete; verify by hand: {}: {cause}",
            paths.join(" ")
        ))
    }
}

/// Install every embedded skill under each root, previewing by default.
///
/// `record` is the user-scope digest record: read to tell a stale copy this
/// tool wrote from a file the user edited, and rewritten after a successful
/// apply. A failure to write the record is not a failure of the install —
/// the files landed — so it costs only the benefit of the doubt next time.
///
/// # Errors
///
/// [`AppError::Refused`] when a destination cannot be touched, when one
/// holds bytes neither reference accounts for and `force` is not set, or
/// when a write fails partway; the destinations are restored before that
/// last one returns. I/O errors when a destination cannot be read.
pub fn install(
    roots: &[Utf8PathBuf],
    record_path: &Utf8Path,
    apply: bool,
    force: bool,
) -> Result<Vec<String>, AppError> {
    let planned = plan(roots)?;
    let mut lines: Vec<String> = Vec::new();
    for entry in &planned {
        check_destination(&entry.destination)?;
        lines.push(entry.destination.to_string());
    }
    if !apply {
        lines.push("DRY RUN: no files written".to_string());
        return Ok(lines);
    }

    let mut record = SkillRecord::load(record_path);
    if !force {
        let conflicts = conflicts(&planned, &record)?;
        if !conflicts.is_empty() {
            return Err(AppError::Refused(format!(
                "destinations hold bytes this tool did not write: {}; re-run with --force to overwrite",
                conflicts.join(", ")
            )));
        }
    }

    // Back up every destination before the first write, so a failure on the
    // second root cannot leave the first one upgraded.
    let mut backups: BTreeMap<Utf8PathBuf, Option<Vec<u8>>> = BTreeMap::new();
    for entry in &planned {
        let previous = if entry.destination.is_file() {
            Some(std::fs::read(&entry.destination).map_err(|source| {
                AppError::Refused(format!("cannot back up {}: {source}", entry.destination))
            })?)
        } else {
            None
        };
        backups.insert(entry.destination.clone(), previous);
    }

    for entry in &planned {
        if let Err(source) = crate::adapters::fs::write_file(&entry.destination, entry.bytes) {
            return Err(abort(
                &rollback(&backups),
                &format!("writing {} failed: {source}", entry.destination),
            ));
        }
    }

    for entry in &planned {
        record
            .written
            .insert(entry.destination.clone(), Sha256::of(entry.bytes));
    }
    if crate::adapters::fs::write_file(record_path, record.to_json().as_bytes()).is_err() {
        lines.push(format!(
            "note: could not record the installed digests at {record_path}; a later install may ask for --force"
        ));
    }
    Ok(lines)
}

/// Remove every installed skill under each root, previewing by default.
///
/// Only the payload's own files go: each skill's `SKILL.md`, and its
/// directory when nothing else lives there. An absent destination is a
/// no-op, so a re-run succeeds. Removed destinations leave the record too;
/// what is gone cannot be vouched for.
///
/// # Errors
///
/// [`AppError::Refused`] when a destination is a symlink or not a regular
/// file, and I/O errors when a removal fails.
pub fn uninstall(
    roots: &[Utf8PathBuf],
    record_path: &Utf8Path,
    apply: bool,
) -> Result<Vec<String>, AppError> {
    let mut lines: Vec<String> = Vec::new();
    let mut removable: Vec<Utf8PathBuf> = Vec::new();
    for root in roots {
        for name in crate::embedded::skill_names() {
            let destination = root.join(name).join("SKILL.md");
            check_destination(&destination)?;
            if destination.is_file() {
                lines.push(destination.to_string());
                removable.push(destination);
            }
        }
    }
    if !apply {
        lines.push("DRY RUN: no files removed".to_string());
        return Ok(lines);
    }
    for destination in &removable {
        std::fs::remove_file(destination)?;
        let directory = destination
            .parent()
            .ok_or_else(|| anyhow::anyhow!("destination has no parent: {destination}"))?;
        if std::fs::read_dir(directory)?.next().is_none() {
            std::fs::remove_dir(directory)?;
        } else {
            lines.push(format!("kept (not empty): {directory}"));
        }
    }

    let mut record = SkillRecord::load(record_path);
    for destination in &removable {
        record.written.remove(destination);
    }
    let _ = if record.written.is_empty() {
        std::fs::remove_file(record_path).map_err(|_| ())
    } else {
        crate::adapters::fs::write_file(record_path, record.to_json().as_bytes()).map_err(|_| ())
    };
    Ok(lines)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn root(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap())
    }

    /// The skill roots and the record path a home directory implies.
    fn home(dir: &tempfile::TempDir) -> (Vec<Utf8PathBuf>, Utf8PathBuf) {
        let home = root(dir);
        (
            vec![home.join(".agents/skills"), home.join(".claude/skills")],
            home.join(crate::domain::skill_record::RECORD_PATH),
        )
    }

    #[test]
    fn a_preview_lists_every_destination_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (roots, record) = home(&dir);
        let lines = install(&roots, &record, false, false).unwrap();
        assert_eq!(lines.last().unwrap(), "DRY RUN: no files written");
        assert_eq!(lines.len(), crate::embedded::skill_names().len() * 2 + 1);
        assert!(!roots[0].exists());
        assert!(!record.exists());
    }

    #[test]
    fn an_apply_is_idempotent_and_a_conflict_refuses_with_every_path() {
        let dir = tempfile::tempdir().unwrap();
        let (roots, record) = home(&dir);
        install(&roots, &record, true, false).unwrap();
        install(&roots, &record, true, false).unwrap();
        for name in crate::embedded::skill_names() {
            std::fs::write(roots[0].join(name).join("SKILL.md"), "edited").unwrap();
        }
        let error = install(&roots, &record, true, false).unwrap_err();
        let message = error.to_string();
        for name in crate::embedded::skill_names() {
            assert!(message.contains(name), "{message} misses {name}");
        }
        install(&roots, &record, true, true).unwrap();
        let text = std::fs::read_to_string(roots[0].join("sdd-setup/SKILL.md")).unwrap();
        assert!(text.contains("name: sdd-setup"));
    }

    /// The defect this record exists for: bytes a previous release wrote are
    /// not the user's, and refusing on them makes every skill-touching
    /// release break the install recipe.
    #[test]
    fn a_copy_a_previous_release_wrote_is_replaced_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let (roots, record) = home(&dir);
        install(&roots, &record, true, false).unwrap();

        // Stand in for an older release: rewrite each destination and record
        // the digest, exactly as that release's apply would have left it.
        let mut stale = SkillRecord::load(&record);
        for root in &roots {
            for name in crate::embedded::skill_names() {
                let destination = root.join(name).join("SKILL.md");
                std::fs::write(&destination, "older canon bytes\n").unwrap();
                stale
                    .written
                    .insert(destination, Sha256::of(b"older canon bytes\n"));
            }
        }
        crate::adapters::fs::write_file(&record, stale.to_json().as_bytes()).unwrap();

        install(&roots, &record, true, false).unwrap();
        let text = std::fs::read_to_string(roots[1].join("sdd-setup/SKILL.md")).unwrap();
        assert!(text.contains("name: sdd-setup"));
    }

    /// A record vouching for one destination says nothing about the others.
    #[test]
    fn an_edit_still_refuses_when_a_sibling_is_merely_stale() {
        let dir = tempfile::tempdir().unwrap();
        let (roots, record) = home(&dir);
        install(&roots, &record, true, false).unwrap();

        let stale_path = roots[0].join("sdd-setup/SKILL.md");
        let edited_path = roots[1].join("sdd-setup/SKILL.md");
        let mut stale = SkillRecord::load(&record);
        std::fs::write(&stale_path, "older canon bytes\n").unwrap();
        stale
            .written
            .insert(stale_path, Sha256::of(b"older canon bytes\n"));
        crate::adapters::fs::write_file(&record, stale.to_json().as_bytes()).unwrap();
        std::fs::write(&edited_path, "mine\n").unwrap();

        let message = install(&roots, &record, true, false)
            .unwrap_err()
            .to_string();
        assert!(message.contains(edited_path.as_str()), "{message}");
        assert!(!message.contains("older canon"), "{message}");
        assert_eq!(std::fs::read_to_string(&edited_path).unwrap(), "mine\n");
    }

    /// Without a readable record every destination is the user's, which is
    /// the conservative half of the old behaviour and must survive.
    #[test]
    fn a_missing_record_treats_unknown_bytes_as_the_users() {
        let dir = tempfile::tempdir().unwrap();
        let (roots, record) = home(&dir);
        install(&roots, &record, true, false).unwrap();
        std::fs::remove_file(&record).unwrap();
        std::fs::write(roots[0].join("sdd-setup/SKILL.md"), "older canon bytes\n").unwrap();
        let error = install(&roots, &record, true, false).unwrap_err();
        assert!(error.to_string().contains("sdd-setup"));
    }

    /// A destination that cannot be written must not leave the roots split.
    ///
    /// This is the shape the defect took in the field: two roots, the first
    /// written, the second refused, and no rollback — leaving one agent on
    /// the new skill and the other on the old one.
    #[test]
    fn a_write_that_fails_partway_restores_every_destination() {
        let dir = tempfile::tempdir().unwrap();
        let (roots, record) = home(&dir);
        install(&roots, &record, true, false).unwrap();

        // Every destination carries recognisable bytes, then the second
        // root's first destination is made uncreatable: its file is gone and
        // its directory denies writes, so the failure lands after the first
        // root is already rewritten.
        let mut before: Vec<(Utf8PathBuf, Vec<u8>)> = Vec::new();
        for root in &roots {
            for name in crate::embedded::skill_names() {
                let path = root.join(name).join("SKILL.md");
                std::fs::write(&path, format!("previous {name}\n")).unwrap();
                before.push((path.clone(), std::fs::read(&path).unwrap()));
            }
        }
        let blocked = roots[1].join("sdd-setup");
        std::fs::remove_file(blocked.join("SKILL.md")).unwrap();
        before.retain(|(path, _)| path.parent() != Some(blocked.as_path()));
        let mut permissions = std::fs::metadata(&blocked).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o500);
        std::fs::set_permissions(&blocked, permissions.clone()).unwrap();

        let message = install(&roots, &record, true, true)
            .unwrap_err()
            .to_string();
        assert!(message.contains("skill install aborted"), "{message}");
        assert!(
            message.contains("the destinations were restored"),
            "{message}"
        );
        assert!(message.contains(blocked.as_str()), "{message}");

        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o700);
        std::fs::set_permissions(&blocked, permissions).unwrap();
        for (path, bytes) in &before {
            assert_eq!(
                &std::fs::read(path).unwrap(),
                bytes,
                "{path} was not restored"
            );
        }
        assert!(
            !blocked.join("SKILL.md").exists(),
            "a destination that did not exist before was left behind"
        );
    }

    #[test]
    fn an_uninstall_removes_only_payload_files_and_keeps_foreign_ones() {
        let dir = tempfile::tempdir().unwrap();
        let (roots, record) = home(&dir);
        install(&roots, &record, true, false).unwrap();
        std::fs::write(roots[1].join("sdd-setup/notes.md"), "mine").unwrap();

        let preview = uninstall(&roots, &record, false).unwrap();
        assert_eq!(preview.last().unwrap(), "DRY RUN: no files removed");
        assert!(roots[1].join("sdd-setup/SKILL.md").is_file());

        let lines = uninstall(&roots, &record, true).unwrap();
        assert!(!roots[1].join("sdd-setup/SKILL.md").exists());
        assert!(!roots[1].join("sdd-write-docs").exists());
        assert_eq!(
            std::fs::read_to_string(roots[1].join("sdd-setup/notes.md")).unwrap(),
            "mine"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("kept (not empty):"))
        );
        assert!(
            !record.exists(),
            "the record outlived every file it vouched for"
        );

        // A re-run on the emptied roots is a no-op, not an error.
        uninstall(&roots, &record, true).unwrap();
    }

    /// Uninstalling one root leaves the other's entries intact, so the next
    /// install still recognises what it wrote there.
    #[test]
    fn an_uninstall_of_one_root_keeps_the_others_entries() {
        let dir = tempfile::tempdir().unwrap();
        let (roots, record) = home(&dir);
        install(&roots, &record, true, false).unwrap();
        uninstall(std::slice::from_ref(&roots[1]), &record, true).unwrap();
        let kept = SkillRecord::load(&record);
        assert!(kept.written.keys().all(|path| path.starts_with(&roots[0])));
        assert!(!kept.written.is_empty());
    }
}
