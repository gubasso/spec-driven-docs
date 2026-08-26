//! Install the embedded skills into agent skill directories.
//!
//! The destinations live outside any instance — under the invoking user's
//! home — so nothing here touches an instance manifest; the embedded
//! payload is the reference every destination is compared against. The
//! preview-by-default and list-every-conflict conventions mirror the
//! instance installer.

use camino::{Utf8Path, Utf8PathBuf};

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

/// Install every embedded skill under each root, previewing by default.
///
/// # Errors
///
/// [`AppError::Refused`] when a destination cannot be touched or holds
/// bytes that differ from the payload and `force` is not set, and I/O
/// errors when a write fails.
pub fn install(roots: &[Utf8PathBuf], apply: bool, force: bool) -> Result<Vec<String>, AppError> {
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
    if !force {
        let mut conflicts: Vec<String> = Vec::new();
        for entry in &planned {
            // An unreadable destination raises instead of passing as clean:
            // a comparison that cannot run must never license an overwrite.
            if entry.destination.is_file() && std::fs::read(&entry.destination)? != entry.bytes {
                conflicts.push(entry.destination.to_string());
            }
        }
        if !conflicts.is_empty() {
            return Err(AppError::Refused(format!(
                "destinations hold locally changed bytes: {}; re-run with --force to overwrite",
                conflicts.join(", ")
            )));
        }
    }
    for entry in &planned {
        crate::adapters::fs::write_file(&entry.destination, entry.bytes)?;
    }
    Ok(lines)
}

/// Remove every installed skill under each root, previewing by default.
///
/// Only the payload's own files go: each skill's `SKILL.md`, and its
/// directory when nothing else lives there. An absent destination is a
/// no-op, so a re-run succeeds.
///
/// # Errors
///
/// [`AppError::Refused`] when a destination is a symlink or not a regular
/// file, and I/O errors when a removal fails.
pub fn uninstall(roots: &[Utf8PathBuf], apply: bool) -> Result<Vec<String>, AppError> {
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
    Ok(lines)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn root(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap())
    }

    #[test]
    fn a_preview_lists_every_destination_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let skills = root(&dir).join(".claude/skills");
        let lines = install(std::slice::from_ref(&skills), false, false).unwrap();
        assert_eq!(lines.last().unwrap(), "DRY RUN: no files written");
        assert_eq!(lines.len(), crate::embedded::skill_names().len() + 1);
        assert!(!skills.exists());
    }

    #[test]
    fn an_apply_is_idempotent_and_a_conflict_refuses_with_every_path() {
        let dir = tempfile::tempdir().unwrap();
        let skills = root(&dir).join(".agents/skills");
        install(std::slice::from_ref(&skills), true, false).unwrap();
        install(std::slice::from_ref(&skills), true, false).unwrap();
        for name in crate::embedded::skill_names() {
            std::fs::write(skills.join(name).join("SKILL.md"), "edited").unwrap();
        }
        let error = install(std::slice::from_ref(&skills), true, false).unwrap_err();
        let message = error.to_string();
        for name in crate::embedded::skill_names() {
            assert!(message.contains(name), "{message} misses {name}");
        }
        install(std::slice::from_ref(&skills), true, true).unwrap();
        let text = std::fs::read_to_string(skills.join("sdd-setup/SKILL.md")).unwrap();
        assert!(text.contains("name: sdd-setup"));
    }

    #[test]
    fn an_uninstall_removes_only_payload_files_and_keeps_foreign_ones() {
        let dir = tempfile::tempdir().unwrap();
        let skills = root(&dir).join(".claude/skills");
        install(std::slice::from_ref(&skills), true, false).unwrap();
        std::fs::write(skills.join("sdd-setup/notes.md"), "mine").unwrap();

        let preview = uninstall(std::slice::from_ref(&skills), false).unwrap();
        assert_eq!(preview.last().unwrap(), "DRY RUN: no files removed");
        assert!(skills.join("sdd-setup/SKILL.md").is_file());

        let lines = uninstall(std::slice::from_ref(&skills), true).unwrap();
        assert!(!skills.join("sdd-setup/SKILL.md").exists());
        assert!(!skills.join("sdd-write-docs").exists());
        assert_eq!(
            std::fs::read_to_string(skills.join("sdd-setup/notes.md")).unwrap(),
            "mine"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("kept (not empty):"))
        );

        // A re-run on the emptied roots is a no-op, not an error.
        uninstall(std::slice::from_ref(&skills), true).unwrap();
    }
}
