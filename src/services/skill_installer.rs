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

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::ownership::Sha256;
use crate::domain::skill_record::SkillRecord;
use crate::error::AppError;

/// The root holding what the skills share, relative to the home directory.
///
/// Home-relative rather than `XDG_STATE_HOME`-relative for the reason the
/// record states: the skills naming these artifacts live under
/// `$HOME/.agents` and `$HOME/.claude`, which no XDG variable moves, and a
/// shared file reachable under a different home than the skills reading it
/// would be worse than no shared file at all.
pub const SHARED_ROOT: &str = ".local/state/spec-driven-docs/skills/shared";

/// The skill root Claude Code reads, relative to the home directory.
pub const CLAUDE_ROOT: &str = ".claude/skills";

/// The skill root Codex, Gemini CLI, and Copilot read, relative to the home
/// directory.
pub const AGENTS_ROOT: &str = ".agents/skills";

/// The invoking user's home directory.
///
/// # Errors
///
/// [`AppError::Usage`] when `HOME` is unset or empty.
pub fn home() -> Result<Utf8PathBuf, AppError> {
    std::env::var("HOME")
        .ok()
        .filter(|home| !home.is_empty())
        .map(Utf8PathBuf::from)
        .ok_or_else(|| AppError::Usage("HOME is not set".to_string()))
}

/// One planned write: where, and which bytes.
struct Planned {
    destination: Utf8PathBuf,
    bytes: &'static [u8],
}

/// Where one run writes: the agent roots, the shared root, and the record.
///
/// The shared root is not an agent root and no `--agent` selects it. Every
/// skill names its shared artifacts by one absolute path, so one copy serves
/// both agent families, and an install writes it whichever family it was
/// asked for.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The agent skill roots this run was asked to touch.
    pub roots: Vec<Utf8PathBuf>,
    /// Every agent skill root, whichever this run selected.
    pub every_root: Vec<Utf8PathBuf>,
    /// The root holding what the skills share.
    pub shared: Utf8PathBuf,
    /// The user-scope digest record.
    pub record: Utf8PathBuf,
}

/// Every skill destination under `roots`, root by root and skill by skill.
fn plan_roots(roots: &[Utf8PathBuf]) -> Result<Vec<Planned>, AppError> {
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

/// Every shared destination under `shared`.
fn plan_shared(shared: &Utf8Path) -> Vec<Planned> {
    crate::embedded::shared_artifacts()
        .into_iter()
        .map(|(path, bytes)| Planned {
            destination: shared.join(path),
            bytes,
        })
        .collect()
}

/// Refuse a shared root reached through a symlink this tool would follow.
///
/// The chain from the state directory down to the shared root is this
/// tool's own — nothing here ever creates a symlink in it, so one found
/// there redirects every shared write and removal somewhere else, and
/// `check_destination` cannot see it: the final component it checks is not
/// itself a link. The directories above the state directory are the user's
/// layout and stay unjudged.
fn check_shared_root(shared: &Utf8Path, record: &Utf8Path) -> Result<(), AppError> {
    let Some(state_dir) = record.parent() else {
        return Ok(());
    };
    let mut current = Some(shared);
    while let Some(dir) = current {
        if !dir.starts_with(state_dir) {
            break;
        }
        if dir.is_symlink() {
            return Err(AppError::Refused(format!(
                "the shared root is reached through a symlink: {dir}"
            )));
        }
        current = dir.parent();
    }
    Ok(())
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

/// Recorded destinations under `roots` that this install no longer plans.
///
/// A skill the canon renamed, dropped, or moved to another root leaves its
/// file behind otherwise, and a stale name is not inert: an agent's picker
/// keys on the name, so the leftover shows up beside the skill that replaced
/// it. Only bytes the record still vouches for are swept — a leftover the
/// user has since edited is theirs, and a symlink is never followed.
fn leftovers(
    roots: &[Utf8PathBuf],
    record: &SkillRecord,
    keep: &[Utf8PathBuf],
) -> Vec<Utf8PathBuf> {
    let kept: BTreeSet<&Utf8Path> = keep.iter().map(Utf8PathBuf::as_path).collect();
    record
        .written
        .iter()
        .filter(|(destination, digest)| {
            !kept.contains(destination.as_path())
                && roots.iter().any(|root| destination.starts_with(root))
                && !destination.is_symlink()
                && destination.is_file()
                && std::fs::read(destination).is_ok_and(|found| Sha256::of(&found) == **digest)
        })
        .map(|(destination, _)| destination.clone())
        .collect()
}

/// Remove one installed destination, and its directory when nothing else is
/// left there.
fn remove_installed(destination: &Utf8Path, lines: &mut Vec<String>) -> Result<(), AppError> {
    std::fs::remove_file(destination)?;
    let directory = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("destination has no parent: {destination}"))?;
    if std::fs::read_dir(directory)?.next().is_none() {
        std::fs::remove_dir(directory)?;
    } else {
        lines.push(format!("kept (not empty): {directory}"));
    }
    Ok(())
}

/// Restore every backed-up destination, returning those that would not go back.
///
/// A destination already holding what it held is restored, whatever a write
/// to it would do. Nothing else distinguishes the two ways an apply reaches
/// here — the write loop stopped before this destination, or a read-only
/// root refused every write including this one — and reporting the second as
/// unrestored sends the operator to verify files no write ever reached.
fn rollback(backups: &BTreeMap<Utf8PathBuf, Option<Vec<u8>>>) -> Vec<Utf8PathBuf> {
    let mut unrestored = Vec::new();
    for (destination, previous) in backups {
        let restored = previous.as_ref().map_or_else(
            || !destination.exists() || std::fs::remove_file(destination).is_ok(),
            |bytes| {
                std::fs::read(destination).is_ok_and(|found| &found == bytes)
                    || crate::adapters::fs::write_file(destination, bytes).is_ok()
            },
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
pub fn install(layout: &Layout, apply: bool, force: bool) -> Result<Vec<String>, AppError> {
    let record_path = layout.record.as_path();
    check_shared_root(&layout.shared, &layout.record)?;
    let mut planned = plan_roots(&layout.roots)?;
    planned.extend(plan_shared(&layout.shared));
    let mut lines: Vec<String> = Vec::new();
    for entry in &planned {
        check_destination(&entry.destination)?;
        lines.push(entry.destination.to_string());
    }
    let mut record = SkillRecord::load(record_path);
    let kept: Vec<Utf8PathBuf> = planned
        .iter()
        .map(|entry| entry.destination.clone())
        .collect();
    let mut scanned = layout.roots.clone();
    scanned.push(layout.shared.clone());
    let stale = leftovers(&scanned, &record, &kept);
    for destination in &stale {
        lines.push(format!("sweep (no longer in the payload): {destination}"));
    }
    if !apply {
        lines.push("DRY RUN: no files written".to_string());
        return Ok(lines);
    }

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

    // Sweep after the writes, never before: a refusal must leave the home
    // exactly as it found it, and a leftover is harmless until the install
    // that supersedes it has actually landed.
    for destination in &stale {
        if let Err(source) = remove_installed(destination, &mut lines) {
            lines.push(format!(
                "could not remove {destination}; remove it by hand: {source}"
            ));
            continue;
        }
        record.written.remove(destination);
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
pub fn uninstall(layout: &Layout, apply: bool) -> Result<Vec<String>, AppError> {
    let record_path = layout.record.as_path();
    let mut lines: Vec<String> = Vec::new();
    let mut removable: Vec<Utf8PathBuf> = Vec::new();
    for root in &layout.roots {
        for name in crate::embedded::skill_names() {
            let destination = root.join(name).join("SKILL.md");
            check_destination(&destination)?;
            if destination.is_file() {
                lines.push(destination.to_string());
                removable.push(destination);
            }
        }
    }
    // The shared artifacts serve every agent root, so they go only once no
    // root still holds an installed skill that reads them. Taking them
    // during a one-agent uninstall would leave the other family's skills
    // naming a file that is no longer there.
    let going: BTreeSet<&Utf8Path> = removable.iter().map(Utf8PathBuf::as_path).collect();
    let retained = plan_roots(&layout.every_root)?
        .iter()
        .any(|entry| !going.contains(entry.destination.as_path()) && entry.destination.is_file());
    let mut scanned = layout.roots.clone();
    if !retained {
        check_shared_root(&layout.shared, &layout.record)?;
        for entry in plan_shared(&layout.shared) {
            check_destination(&entry.destination)?;
            if entry.destination.is_file() {
                lines.push(entry.destination.to_string());
                removable.push(entry.destination);
            }
        }
        scanned.push(layout.shared.clone());
    }
    let mut record = SkillRecord::load(record_path);
    // A skill the payload has since dropped is still ours to take back, and
    // an uninstall that leaves it behind is the leftover an agent's picker
    // keeps offering. The record is what names it; the payload cannot.
    for destination in leftovers(&scanned, &record, &removable) {
        lines.push(format!("sweep (no longer in the payload): {destination}"));
        removable.push(destination);
    }
    if !apply {
        lines.push("DRY RUN: no files removed".to_string());
        return Ok(lines);
    }
    for destination in &removable {
        remove_installed(destination, &mut lines)?;
    }

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

    /// The layout a home directory implies, selecting both agent roots.
    fn home(dir: &tempfile::TempDir) -> Layout {
        let home = root(dir);
        let roots = vec![home.join(".agents/skills"), home.join(".claude/skills")];
        Layout {
            roots: roots.clone(),
            every_root: roots,
            shared: home.join(".local/state/spec-driven-docs/skills/shared"),
            record: home.join(crate::domain::skill_record::RECORD_PATH),
        }
    }

    /// The same layout narrowed to one selected root.
    fn select(layout: &Layout, index: usize) -> Layout {
        Layout {
            roots: vec![layout.roots[index].clone()],
            ..layout.clone()
        }
    }

    #[test]
    fn a_preview_lists_every_destination_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let (roots, record) = (layout.roots.clone(), layout.record.clone());
        let lines = install(&layout, false, false).unwrap();
        assert_eq!(lines.last().unwrap(), "DRY RUN: no files written");
        assert_eq!(
            lines.len(),
            crate::embedded::skill_names().len() * 2
                + crate::embedded::shared_artifacts().len()
                + 1
        );
        assert!(!layout.shared.exists());
        assert!(!roots[0].exists());
        assert!(!record.exists());
    }

    #[test]
    fn an_apply_is_idempotent_and_a_conflict_refuses_with_every_path() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let roots = layout.roots.clone();
        install(&layout, true, false).unwrap();
        install(&layout, true, false).unwrap();
        for name in crate::embedded::skill_names() {
            std::fs::write(roots[0].join(name).join("SKILL.md"), "edited").unwrap();
        }
        let error = install(&layout, true, false).unwrap_err();
        let message = error.to_string();
        for name in crate::embedded::skill_names() {
            assert!(message.contains(name), "{message} misses {name}");
        }
        install(&layout, true, true).unwrap();
        let text = std::fs::read_to_string(roots[0].join("sdd-setup/SKILL.md")).unwrap();
        assert!(text.contains("name: sdd-setup"));
    }

    /// The defect this record exists for: bytes a previous release wrote are
    /// not the user's, and refusing on them makes every skill-touching
    /// release break the install recipe.
    #[test]
    fn a_copy_a_previous_release_wrote_is_replaced_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let (roots, record) = (layout.roots.clone(), layout.record.clone());
        install(&layout, true, false).unwrap();

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

        install(&layout, true, false).unwrap();
        let text = std::fs::read_to_string(roots[1].join("sdd-setup/SKILL.md")).unwrap();
        assert!(text.contains("name: sdd-setup"));
    }

    /// A record vouching for one destination says nothing about the others.
    #[test]
    fn an_edit_still_refuses_when_a_sibling_is_merely_stale() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let (roots, record) = (layout.roots.clone(), layout.record.clone());
        install(&layout, true, false).unwrap();

        let stale_path = roots[0].join("sdd-setup/SKILL.md");
        let edited_path = roots[1].join("sdd-setup/SKILL.md");
        let mut stale = SkillRecord::load(&record);
        std::fs::write(&stale_path, "older canon bytes\n").unwrap();
        stale
            .written
            .insert(stale_path, Sha256::of(b"older canon bytes\n"));
        crate::adapters::fs::write_file(&record, stale.to_json().as_bytes()).unwrap();
        std::fs::write(&edited_path, "mine\n").unwrap();

        let message = install(&layout, true, false).unwrap_err().to_string();
        assert!(message.contains(edited_path.as_str()), "{message}");
        assert!(!message.contains("older canon"), "{message}");
        assert_eq!(std::fs::read_to_string(&edited_path).unwrap(), "mine\n");
    }

    /// Without a readable record every destination is the user's, which is
    /// the conservative half of the old behaviour and must survive.
    #[test]
    fn a_missing_record_treats_unknown_bytes_as_the_users() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let (roots, record) = (layout.roots.clone(), layout.record.clone());
        install(&layout, true, false).unwrap();
        std::fs::remove_file(&record).unwrap();
        std::fs::write(roots[0].join("sdd-setup/SKILL.md"), "older canon bytes\n").unwrap();
        let error = install(&layout, true, false).unwrap_err();
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
        let layout = home(&dir);
        let roots = layout.roots.clone();
        install(&layout, true, false).unwrap();

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

        let message = install(&layout, true, true).unwrap_err().to_string();
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

    /// A destination the record vouches for but the payload dropped is a
    /// leftover, not a skill: an agent still offers it under the name the
    /// canon renamed away from.
    #[test]
    fn a_recorded_destination_the_payload_dropped_is_swept_by_both_verbs() {
        for sweep_with_uninstall in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let layout = home(&dir);
            let (roots, record_path) = (layout.roots.clone(), layout.record.clone());
            install(&layout, true, false).unwrap();

            let dropped = roots[0].join("sdd-old-name/SKILL.md");
            crate::adapters::fs::write_file(&dropped, b"older\n").unwrap();
            let mut record = SkillRecord::load(&record_path);
            record
                .written
                .insert(dropped.clone(), Sha256::of(b"older\n"));
            crate::adapters::fs::write_file(&record_path, record.to_json().as_bytes()).unwrap();

            if sweep_with_uninstall {
                uninstall(&layout, true).unwrap();
            } else {
                install(&layout, true, false).unwrap();
            }
            assert!(!dropped.exists(), "the leftover file survived");
            assert!(
                !roots[0].join("sdd-old-name").exists(),
                "the leftover directory survived"
            );
            assert!(
                !SkillRecord::load(&record_path)
                    .written
                    .contains_key(&dropped)
            );
        }
    }

    /// The record vouches for bytes, so a leftover the user rewrote is
    /// theirs and no sweep may take it.
    #[test]
    fn an_edited_leftover_is_left_where_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let (roots, record_path) = (layout.roots.clone(), layout.record.clone());
        install(&layout, true, false).unwrap();

        let dropped = roots[0].join("sdd-old-name/SKILL.md");
        crate::adapters::fs::write_file(&dropped, b"older\n").unwrap();
        let mut record = SkillRecord::load(&record_path);
        record
            .written
            .insert(dropped.clone(), Sha256::of(b"older\n"));
        crate::adapters::fs::write_file(&record_path, record.to_json().as_bytes()).unwrap();
        crate::adapters::fs::write_file(&dropped, b"mine\n").unwrap();

        install(&layout, true, false).unwrap();
        assert_eq!(std::fs::read(&dropped).unwrap(), b"mine\n");
        uninstall(&layout, true).unwrap();
        assert_eq!(std::fs::read(&dropped).unwrap(), b"mine\n");
    }

    /// A root no write can reach leaves every destination as it was, so the
    /// refusal must say the destinations were restored rather than sending
    /// the operator to verify files nothing touched.
    #[test]
    fn a_root_that_refuses_every_write_reports_a_clean_restore() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let roots = layout.roots.clone();
        install(&layout, true, false).unwrap();

        // `.agents` is the first root, so its refusal lands before any
        // destination has been rewritten.
        let mut locked = Vec::new();
        for name in crate::embedded::skill_names() {
            let destination = roots[0].join(name).join("SKILL.md");
            let mut permissions = std::fs::metadata(&destination).unwrap().permissions();
            std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o444);
            std::fs::set_permissions(&destination, permissions).unwrap();
            locked.push(destination);
        }

        let message = install(&layout, true, true).unwrap_err().to_string();

        for destination in &locked {
            let mut permissions = std::fs::metadata(destination).unwrap().permissions();
            std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o644);
            std::fs::set_permissions(destination, permissions).unwrap();
        }

        assert!(
            message.contains("the destinations were restored"),
            "{message}"
        );
        assert!(!message.contains("restoration is incomplete"), "{message}");
    }

    #[test]
    fn an_uninstall_removes_only_payload_files_and_keeps_foreign_ones() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let (roots, record) = (layout.roots.clone(), layout.record.clone());
        install(&layout, true, false).unwrap();
        std::fs::write(roots[1].join("sdd-setup/notes.md"), "mine").unwrap();

        let preview = uninstall(&layout, false).unwrap();
        assert_eq!(preview.last().unwrap(), "DRY RUN: no files removed");
        assert!(roots[1].join("sdd-setup/SKILL.md").is_file());

        let lines = uninstall(&layout, true).unwrap();
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
        uninstall(&layout, true).unwrap();
    }

    /// Uninstalling one root leaves the other's entries intact, so the next
    /// install still recognises what it wrote there.
    #[test]
    fn an_uninstall_of_one_root_keeps_the_others_entries() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        let (roots, record) = (layout.roots.clone(), layout.record.clone());
        install(&layout, true, false).unwrap();
        uninstall(&select(&layout, 1), true).unwrap();
        let kept = SkillRecord::load(&record);
        assert!(
            kept.written
                .keys()
                .all(|path| path.starts_with(&roots[0]) || path.starts_with(&layout.shared))
        );
        assert!(kept.written.keys().any(|path| path.starts_with(&roots[0])));
        assert!(
            kept.written
                .keys()
                .any(|path| path.starts_with(&layout.shared))
        );
    }

    /// The shared artifacts serve both agent families, so either family's
    /// install lands them alone.
    #[test]
    fn either_agent_alone_still_lands_the_shared_artifacts() {
        for index in 0..2 {
            let dir = tempfile::tempdir().unwrap();
            let layout = home(&dir);
            install(&select(&layout, index), true, false).unwrap();
            assert!(layout.shared.join("plan-gate.md").is_file());
            assert!(layout.shared.join("pre-flight-gate.md").is_file());
        }
    }

    /// A one-family uninstall keeps the shared artifacts while the other
    /// family's skills still name them; the last one takes them along.
    #[test]
    fn the_shared_artifacts_stay_while_another_root_still_holds_skills() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        uninstall(&select(&layout, 1), true).unwrap();
        assert!(layout.shared.join("plan-gate.md").is_file());
        assert!(layout.shared.join("pre-flight-gate.md").is_file());
        uninstall(&select(&layout, 0), true).unwrap();
        assert!(!layout.shared.exists());
        assert!(!layout.record.exists());
    }

    /// The uninstall that takes the last skills previews the shared
    /// artifacts with them, and removes nothing.
    #[test]
    fn the_last_uninstall_previews_the_shared_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        let lines = uninstall(&layout, false).unwrap();
        for artifact in ["plan-gate.md", "pre-flight-gate.md"] {
            let shared = layout.shared.join(artifact);
            assert!(lines.iter().any(|line| line == shared.as_str()));
            assert!(shared.is_file());
        }
    }

    /// An edited shared artifact is a conflict like an edited skill: the
    /// install refuses naming it, and `--force` is the override.
    #[test]
    fn an_edited_shared_artifact_refuses_an_install() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        install(&layout, true, false).unwrap();
        let shared = layout.shared.join("plan-gate.md");
        std::fs::write(&shared, "mine\n").unwrap();
        let message = install(&layout, true, false).unwrap_err().to_string();
        assert!(message.contains(shared.as_str()), "{message}");
        install(&layout, true, true).unwrap();
        assert!(
            std::fs::read_to_string(&shared)
                .unwrap()
                .contains("# The plan gate")
        );
    }

    /// A shared root reached through a symlinked directory refuses both
    /// verbs: the write would land, and the removal would delete, wherever
    /// the link points.
    #[test]
    fn a_symlinked_shared_root_refuses_install_and_uninstall() {
        for symlinked in ["skills", "skills/shared"] {
            let dir = tempfile::tempdir().unwrap();
            let layout = home(&dir);
            let elsewhere = root(&dir).join("elsewhere");
            std::fs::create_dir_all(&elsewhere).unwrap();
            let state_dir = layout.record.parent().unwrap();
            let linked = state_dir.join(symlinked);
            std::fs::create_dir_all(linked.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(&elsewhere, &linked).unwrap();

            let message = install(&layout, true, false).unwrap_err().to_string();
            assert!(message.contains("symlink"), "{message}");
            assert!(!elsewhere.join("plan-gate.md").exists());
            assert!(!layout.roots[0].exists());

            std::fs::write(elsewhere.join("plan-gate.md"), "theirs\n").unwrap();
            let message = uninstall(&layout, true).unwrap_err().to_string();
            assert!(message.contains("symlink"), "{message}");
            assert!(elsewhere.join("plan-gate.md").exists());
        }
    }

    /// A symlinked shared destination refuses before anything is written.
    #[test]
    fn a_symlinked_shared_destination_refuses_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let layout = home(&dir);
        std::fs::create_dir_all(&layout.shared).unwrap();
        let target = root(&dir).join("elsewhere.md");
        std::fs::write(&target, "x").unwrap();
        std::os::unix::fs::symlink(&target, layout.shared.join("plan-gate.md")).unwrap();
        let message = install(&layout, true, false).unwrap_err().to_string();
        assert!(message.contains("symlink"), "{message}");
        assert!(!layout.roots[0].exists());
    }
}
