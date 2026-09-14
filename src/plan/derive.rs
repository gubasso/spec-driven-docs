//! Turn one computed target state into the operations that reach it.
//!
//! What a release lands into a target is one computation, in the
//! installer. Deriving it a second time here would be two places for one
//! rule to drift, so the planner takes the installer's answer and says
//! what kind of write each destination is.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::ownership::Sha256;
use crate::domain::paths::{AGENTS_DIGEST_PATH, HOOKS_CONFIG_PATH, MANIFEST_PATH, PRUNABLE_ROOTS};
use crate::domain::profile::{ProfileId, resolve_destination};
use crate::domain::projection::Declaration;
use crate::error::AppError;
use crate::plan::operation::{Class, Operation, TargetPath};

/// Every write one target state implies, with the bytes each one lands.
pub type Derived = (Vec<Operation>, BTreeMap<Sha256, Vec<u8>>);

/// Every write one target state implies, and the bytes each one lands.
///
/// A destination that already holds the bytes is not an operation: a plan
/// that listed it would tell an operator something would change when
/// nothing would.
///
/// # Errors
///
/// [`AppError::Refused`] for a destination no operation may name, and I/O
/// errors reading what the target holds now.
pub fn operations_for(
    target: &Utf8Path,
    files: &[(Utf8PathBuf, Vec<u8>)],
    declaration: &Declaration,
    profile: ProfileId,
    recorded_managed: &[(String, Sha256)],
) -> Result<Derived, AppError> {
    let adopted = adopted_destinations(declaration, profile);
    let mut operations = Vec::new();
    let mut blobs = BTreeMap::new();
    for (destination, bytes) in files {
        let path = TargetPath::new(destination.as_str())
            .map_err(|error| AppError::Refused(error.to_string()))?;
        let after = Sha256::of(bytes);
        let before = std::fs::read(target.join(destination))
            .ok()
            .map(|held| Sha256::of(&held));
        if before.as_ref() == Some(&after) {
            continue;
        }
        blobs.insert(after.clone(), bytes.clone());
        operations.push(match destination.as_str() {
            MANIFEST_PATH => Operation::WriteRecord {
                path,
                before,
                after,
            },
            // A marked region is spliced into a file the project owns, so
            // the operation says which region rather than claiming the
            // whole file.
            HOOKS_CONFIG_PATH | AGENTS_DIGEST_PATH => Operation::SpliceBlock {
                marker: destination.to_string(),
                path,
                before,
                after,
            },
            held if adopted.contains(&held.to_string()) => Operation::WriteFile {
                path,
                class: Class::Adopted,
                before,
                after,
            },
            _ => Operation::WriteFile {
                path,
                class: Class::Managed,
                before,
                after,
            },
        });
    }
    operations.extend(removals(target, files, recorded_managed)?);
    Ok((operations, blobs))
}

/// The debt file the operator asked the landing to record.
///
/// The same bytes `sdd debt baseline --apply` writes, from the same
/// measurements, so the two can never disagree about what a ceiling is.
/// A target that already carries a debt file keeps it: a baseline never
/// widens one, and a landing is not the place to argue with that.
///
/// # Errors
///
/// [`AppError::Refused`] for a destination no operation may name.
pub fn debt_operation(
    target: &Utf8Path,
    measured: &[crate::domain::debt::Measurement],
) -> Result<Option<(Operation, Vec<u8>)>, AppError> {
    use crate::domain::paths::DEBT_PATH;

    if target.join(DEBT_PATH).exists() {
        return Ok(None);
    }
    let debt = crate::domain::debt::Debt::baseline(measured);
    if debt.is_empty() {
        return Ok(None);
    }
    let bytes = debt.render().into_bytes();
    let path = TargetPath::new(DEBT_PATH).map_err(|error| AppError::Refused(error.to_string()))?;
    Ok(Some((
        Operation::WriteDebt {
            path,
            before: None,
            after: Sha256::of(&bytes),
        },
        bytes,
    )))
}

/// Every managed file the record holds that this release no longer lands.
///
/// Only managed, and only under a root the canon owns. An adopted file the
/// release stops seeding stays: the project owns it from the moment it
/// lands, and a version moving is not permission to take it back. A file
/// whose bytes are not the ones the record vouches for stays too, because
/// the operator edited it and a removal would be a silent loss.
fn removals(
    target: &Utf8Path,
    files: &[(Utf8PathBuf, Vec<u8>)],
    recorded_managed: &[(String, Sha256)],
) -> Result<Vec<Operation>, AppError> {
    let landing: Vec<&str> = files
        .iter()
        .map(|(destination, _)| destination.as_str())
        .collect();
    let mut removals = Vec::new();
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
        let path =
            TargetPath::new(destination).map_err(|error| AppError::Refused(error.to_string()))?;
        let Ok(held) = std::fs::read(target.join(destination)) else {
            continue;
        };
        let found = Sha256::of(&held);
        if &found != recorded {
            continue;
        }
        removals.push(Operation::RemoveOwnedFile {
            path,
            before: found,
        });
    }
    Ok(removals)
}

/// Every destination one profile adopts, resolved.
fn adopted_destinations(declaration: &Declaration, profile: ProfileId) -> Vec<String> {
    let Some(docs_root) = declaration.docs_root(profile) else {
        return Vec::new();
    };
    declaration
        .adopted
        .iter()
        .map(|projection| resolve_destination(&projection.destination, docs_root).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    #[test]
    fn each_destination_takes_the_operation_its_kind_implies() {
        let dir = tempfile::tempdir().unwrap();
        let target = Utf8PathBuf::from(dir.path().to_str().unwrap());
        let declaration = &crate::domain::profile::DECLARATION;
        let files = vec![
            (Utf8PathBuf::from(MANIFEST_PATH), b"record".to_vec()),
            (Utf8PathBuf::from(HOOKS_CONFIG_PATH), b"hooks".to_vec()),
            (Utf8PathBuf::from(AGENTS_DIGEST_PATH), b"digest".to_vec()),
            (
                Utf8PathBuf::from("docs/specs/SPEC-instance.md"),
                b"seed".to_vec(),
            ),
            (
                Utf8PathBuf::from(".spec-driven-docs/markdownlint/adr.markdownlint-cli2.jsonc"),
                b"config".to_vec(),
            ),
        ];
        let (operations, blobs) =
            operations_for(&target, &files, declaration, ProfileId::Codebase, &[]).unwrap();
        let kinds: Vec<&str> = operations.iter().map(Operation::kind).collect();
        assert_eq!(
            kinds,
            [
                "write-record",
                "splice-block",
                "splice-block",
                "write-file",
                "write-file"
            ]
        );
        assert!(matches!(
            operations[3],
            Operation::WriteFile {
                class: Class::Adopted,
                ..
            }
        ));
        assert!(matches!(
            operations[4],
            Operation::WriteFile {
                class: Class::Managed,
                ..
            }
        ));
        assert_eq!(blobs.len(), 5);
    }

    #[test]
    fn a_destination_that_already_holds_the_bytes_is_no_operation() {
        let dir = tempfile::tempdir().unwrap();
        let target = Utf8PathBuf::from(dir.path().to_str().unwrap());
        crate::adapters::fs::write_file(&target.join("a.md"), b"same").unwrap();
        let files = vec![(Utf8PathBuf::from("a.md"), b"same".to_vec())];
        let (operations, blobs) = operations_for(
            &target,
            &files,
            &crate::domain::profile::DECLARATION,
            ProfileId::Codebase,
            &[],
        )
        .unwrap();
        assert!(operations.is_empty());
        assert!(blobs.is_empty());
    }

    #[test]
    fn a_managed_file_the_release_dropped_is_taken_back() {
        let dir = tempfile::tempdir().unwrap();
        let target = Utf8PathBuf::from(dir.path().to_str().unwrap());
        let dropped = ".spec-driven-docs/markdownlint/retired.jsonc";
        crate::adapters::fs::write_file(&target.join(dropped), b"old").unwrap();
        let recorded = vec![(dropped.to_string(), Sha256::of(b"old"))];
        let (operations, _) = operations_for(
            &target,
            &[],
            &crate::domain::profile::DECLARATION,
            ProfileId::Codebase,
            &recorded,
        )
        .unwrap();
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind(), "remove-owned-file");
        assert_eq!(operations[0].path().as_str(), dropped);
    }

    #[test]
    fn an_edited_dropped_file_and_one_outside_the_owned_roots_both_stay() {
        let dir = tempfile::tempdir().unwrap();
        let target = Utf8PathBuf::from(dir.path().to_str().unwrap());
        let edited = ".spec-driven-docs/markdownlint/retired.jsonc";
        let adopted = "docs/specs/SPEC-retired.md";
        crate::adapters::fs::write_file(&target.join(edited), b"mine now").unwrap();
        crate::adapters::fs::write_file(&target.join(adopted), b"seed").unwrap();
        let recorded = vec![
            (edited.to_string(), Sha256::of(b"old")),
            (adopted.to_string(), Sha256::of(b"seed")),
        ];
        let (operations, _) = operations_for(
            &target,
            &[],
            &crate::domain::profile::DECLARATION,
            ProfileId::Codebase,
            &recorded,
        )
        .unwrap();
        assert!(
            operations.is_empty(),
            "an edited file or one the project owns was taken back: {operations:?}"
        );
    }

    #[test]
    fn a_destination_no_operation_may_name_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let target = Utf8PathBuf::from(dir.path().to_str().unwrap());
        let files = vec![(Utf8PathBuf::from("../escape.md"), b"x".to_vec())];
        assert!(
            operations_for(
                &target,
                &files,
                &crate::domain::profile::DECLARATION,
                ProfileId::Codebase,
                &[],
            )
            .is_err()
        );
    }
}
