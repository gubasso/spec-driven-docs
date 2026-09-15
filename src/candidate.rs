//! The candidate this binary renders for one target.
//!
//! One pure function owns every byte a landing would write. It reads the
//! sources compiled into this binary and the evidence the caller gathered
//! from the target, and it returns an ordered list of destinations plus the
//! record that describes them. It reads no filesystem, no environment, no
//! clock, and no network, so the same input renders the same bytes whether
//! a stage asked or a production landing did.
//!
//! What the caller observes about the target is a value in [`Evidence`].
//! Observation is the caller's job, because a projection that looked at the
//! world itself could not be replayed and could not be staged.

use std::collections::BTreeMap;

use camino::Utf8PathBuf;

use crate::domain::instance_config::{InstanceConfig, WritingStyle};
use crate::domain::manifest::{CANON_SOURCE, MANIFEST_PATH, Manifest, SCHEMA_VERSION};
use crate::domain::ownership::{AdoptedEntry, IntegrationBlock, ManagedEntry, Sha256};
use crate::domain::paths::{AGENTS_DIGEST_PATH, HOOKS_CONFIG_PATH};
use crate::domain::profile::{DocsRoot, ProfileId, resolve_destination};
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::services::hooks_render::{RenderOptions, render_block};

/// Who owns a destination's bytes after the landing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    /// This tool keeps owning the bytes and refreshes them.
    Managed,
    /// The project owns the bytes from the moment they land.
    Adopted,
    /// The project owns the file and this tool owns one region inside it.
    Integration,
}

/// How much of the destination the candidate carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Every byte of the file.
    WholeFile,
    /// One marked region, with every byte outside it preserved.
    MarkedRegion,
}

/// One destination of the candidate, and the bytes that would go there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    /// The path, relative to the target root.
    pub path: Utf8PathBuf,
    /// Every byte the file would hold after the landing.
    pub bytes: Vec<u8>,
    /// Who owns those bytes afterwards.
    pub ownership: Ownership,
    /// Whether the candidate carries the file or one region of it.
    pub placement: Placement,
    /// The payload path that produced the bytes, where one did.
    pub source: Option<String>,
}

/// The complete candidate for one target.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Every destination, in the order a landing writes them.
    pub destinations: Vec<Destination>,
    /// The record that describes the landing, written after all of them.
    pub manifest: Manifest,
    /// What the gates judge, as the candidate's own declaration states it.
    ///
    /// This is the resolved value the rendered bytes carry, not the flags
    /// the caller passed: a reader of the candidate needs what it says,
    /// and the two differ wherever the project already declared something.
    pub declaration: InstanceConfig,
    /// What the operator is told about what the projection chose.
    pub notes: Vec<String>,
}

impl Candidate {
    /// Every destination and its bytes, with the record written last.
    #[must_use]
    pub fn files(&self) -> Vec<(Utf8PathBuf, Vec<u8>)> {
        let mut files: Vec<(Utf8PathBuf, Vec<u8>)> = self
            .destinations
            .iter()
            .map(|destination| (destination.path.clone(), destination.bytes.clone()))
            .collect();
        files.push((
            Utf8PathBuf::from(MANIFEST_PATH),
            self.manifest.to_json().into_bytes(),
        ));
        files
    }
}

/// What the caller observed about the target before projecting.
///
/// Every field is a value the caller read once. The projection never looks
/// at the target itself, so a stage and a production run that observed the
/// same target project the same bytes.
#[derive(Debug, Clone, Default)]
pub struct Evidence {
    /// The bytes each existing destination holds, by target-relative path.
    pub existing: BTreeMap<Utf8PathBuf, Vec<u8>>,
    /// The destinations the instance record already calls adopted.
    pub recorded_adopted: Vec<String>,
    /// The hook configuration the target holds, or the empty default.
    pub hooks_host: String,
    /// The root author-instructions file the target holds, or nothing.
    pub agents_host: String,
}

/// What a landing was asked to render.
#[derive(Debug, Clone)]
pub struct Input {
    /// The profile to project.
    pub profile: ProfileId,
    /// The version of the binary doing the rendering.
    pub version: CanonVersion,
    /// The install timestamp to record, carried forward where one exists.
    pub installed_at: String,
    /// The documentation scratch to record.
    pub docs_scratch: Option<Utf8PathBuf>,
    /// Paths to record under `reserved:` in the instance declaration.
    pub reserve: Vec<String>,
    /// The writing-style selection to record in the declaration.
    pub writing_style: Option<WritingStyle>,
    /// What the caller observed about the target.
    pub evidence: Evidence,
}

/// Render the candidate this binary carries for one target.
///
/// # Errors
///
/// [`AppError::Refused`] where this release declares no such profile, where
/// a marked region cannot be read, or where two sources project onto one
/// destination.
#[allow(
    clippy::too_many_lines,
    reason = "the candidate is one ordered pass, and splitting it would hide the order it defines"
)]
pub fn project(input: &Input) -> Result<Candidate, AppError> {
    let declared = crate::domain::profile::DECLARATION
        .profile(input.profile)
        .ok_or_else(|| {
            AppError::Refused(format!(
                "this release declares no {} profile, so it cannot land one",
                input.profile
            ))
        })?;
    let docs_root = declared.docs_root;
    let mut destinations: Vec<Destination> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut managed_entries = Vec::new();
    let mut adopted_entries = Vec::new();

    for projection in declared.managed {
        let bytes = source_bytes(&projection.source)?;
        let path = Utf8PathBuf::from(&projection.destination);
        managed_entries.push(ManagedEntry {
            source: projection.source.clone().into(),
            destination: path.clone(),
            sha256: Sha256::of(&bytes),
        });
        destinations.push(Destination {
            path,
            bytes,
            ownership: Ownership::Managed,
            placement: Placement::WholeFile,
            source: Some(projection.source.clone()),
        });
    }

    for projection in declared.adopted {
        let seed = source_bytes(&projection.source)?;
        let path = resolve_destination(&projection.destination, docs_root);
        let held = input.evidence.existing.get(&path);
        if let Some(held) = held
            && held != &seed
            && !input
                .evidence
                .recorded_adopted
                .iter()
                .any(|recorded| recorded == path.as_str())
        {
            notes.push(format!(
                "note: {path} already exists and is kept; the seed was not written, so read it with 'sdd spec' and reconcile by hand"
            ));
        }
        let mut bytes = held.cloned().unwrap_or_else(|| seed.clone());
        // `--reserve` and `--writing-style` record into the declaration,
        // keeping its comments and whatever the project already wrote there.
        if path == crate::domain::instance_config::CONFIG_PATH
            && let Ok(text) = std::str::from_utf8(&bytes)
        {
            let mut text = text.to_string();
            if !input.reserve.is_empty() {
                text = crate::domain::instance_config::with_reserved(&text, &input.reserve);
            }
            if let Some(selection) = &input.writing_style {
                text = crate::domain::instance_config::with_writing_style(&text, selection);
            }
            bytes = text.into_bytes();
        }
        adopted_entries.push(AdoptedEntry {
            source: projection.source.clone().into(),
            destination: path.clone(),
            sha256: Sha256::of(&bytes),
            baseline_sha256: Sha256::of(&seed),
        });
        destinations.push(Destination {
            path,
            bytes,
            ownership: Ownership::Adopted,
            placement: Placement::WholeFile,
            source: Some(projection.source.clone()),
        });
    }

    let host = if input.evidence.hooks_host.is_empty() {
        "repos:\n".to_string()
    } else {
        input.evidence.hooks_host.clone()
    };
    let (base, _) = crate::domain::marker::split_block(&host)?;
    let indent = crate::domain::marker::splice_indent(&base)?;
    // Render from the declaration this landing is writing, not from the one
    // on disk. With `--reserve` they differ, and a block rendered from the
    // old one would disagree with the file the same landing writes.
    // A declaration that cannot be read is a refusal, never a default. The
    // landing would otherwise keep the project's bytes and wire the block
    // from something else, and the two would disagree from the first
    // commit onward.
    let declared = destinations
        .iter()
        .find(|destination| destination.path == crate::domain::instance_config::CONFIG_PATH);
    let declaration = match declared {
        Some(destination) => {
            let text = std::str::from_utf8(&destination.bytes).map_err(|source| {
                AppError::Refused(format!(
                    "{} is not UTF-8, so what the gates judge cannot be read: {source}",
                    destination.path
                ))
            })?;
            InstanceConfig::parse(text).map_err(|error| {
                AppError::Refused(format!("{} does not parse: {error}", destination.path))
            })?
        }
        None => InstanceConfig::default(),
    };
    let writing_style = declaration.writing_style.clone();
    let block = render_block(&RenderOptions {
        docs_root: docs_root.to_string(),
        indent,
        declaration: declaration.clone(),
        ..RenderOptions::default()
    });
    let spliced = crate::domain::marker::splice(&base, &block)?;
    let marker_hash = crate::domain::marker::block_hash(&spliced)
        .ok_or_else(|| anyhow::anyhow!("the rendered block lost its markers"))?;
    destinations.push(Destination {
        path: Utf8PathBuf::from(HOOKS_CONFIG_PATH),
        bytes: spliced.into_bytes(),
        ownership: Ownership::Integration,
        placement: Placement::MarkedRegion,
        source: None,
    });
    let mut integration_blocks = vec![IntegrationBlock {
        path: HOOKS_CONFIG_PATH.into(),
        marker_hash,
    }];

    let agents_block =
        crate::services::agents_render::render_block(&docs_root.to_string(), &writing_style);
    let agents =
        crate::domain::marker::place_agents_block(&input.evidence.agents_host, &agents_block)?;
    let agents_hash = crate::domain::marker::block_hash_with(
        &agents,
        crate::domain::marker::AGENTS_BEGIN,
        crate::domain::marker::AGENTS_END,
    )
    .ok_or_else(|| anyhow::anyhow!("the rendered AGENTS.md block lost its markers"))?;
    // An old unmarked documentation section is preserved, never deleted; the
    // note tells the operator to remove the duplicate by hand.
    if input.evidence.agents_host.contains("## Documentation")
        && crate::domain::marker::block_region_with(
            &input.evidence.agents_host,
            crate::domain::marker::AGENTS_BEGIN,
            crate::domain::marker::AGENTS_END,
        )
        .is_none()
    {
        notes.push(
            "note: AGENTS.md carries an unmarked '## Documentation' section; the managed block was appended and the old section left in place — remove it by hand".to_string(),
        );
    }
    destinations.push(Destination {
        path: Utf8PathBuf::from(AGENTS_DIGEST_PATH),
        bytes: agents.into_bytes(),
        ownership: Ownership::Integration,
        placement: Placement::MarkedRegion,
        source: None,
    });
    integration_blocks.push(IntegrationBlock {
        path: AGENTS_DIGEST_PATH.into(),
        marker_hash: agents_hash,
    });

    one_destination_each(&destinations)?;

    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        canon_version: input.version,
        canon_source: CANON_SOURCE.to_string(),
        profile: input.profile,
        docs_root,
        installed_at: input.installed_at.clone(),
        docs_scratch: input.docs_scratch.clone(),
        managed_files: managed_entries,
        adopted_files: adopted_entries,
        integration_blocks,
    };

    Ok(Candidate {
        destinations,
        manifest,
        declaration,
        notes,
    })
}

/// The documentation root one profile of this release lands into.
///
/// # Errors
///
/// [`AppError::Refused`] where this release declares no such profile.
pub fn docs_root_of(profile: ProfileId) -> Result<DocsRoot, AppError> {
    crate::domain::profile::DECLARATION
        .docs_root(profile)
        .ok_or_else(|| {
            AppError::Refused(format!(
                "this release declares no {profile} profile, so it cannot land one"
            ))
        })
}

/// The bytes one payload source carries, from this binary's own sources.
///
/// # Errors
///
/// [`AppError::Refused`] where this release does not carry the source.
pub fn source_bytes(source: &str) -> Result<Vec<u8>, AppError> {
    crate::embedded::asset(source)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| {
            AppError::Refused(format!(
                "this release projects {source}, and its own payload does not carry it"
            ))
        })
}

/// Refuse two sources aimed at one destination before any writer runs.
fn one_destination_each(destinations: &[Destination]) -> Result<(), AppError> {
    let mut seen: Vec<&Utf8PathBuf> = Vec::with_capacity(destinations.len());
    for destination in destinations {
        if seen.contains(&&destination.path) {
            return Err(AppError::Refused(format!(
                "{} is projected twice, so the candidate does not describe one file",
                destination.path
            )));
        }
        seen.push(&destination.path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn input(profile: ProfileId) -> Input {
        Input {
            profile,
            version: CanonVersion::current(),
            installed_at: "2026-01-01T00:00:00Z".to_string(),
            docs_scratch: None,
            reserve: Vec::new(),
            writing_style: None,
            evidence: Evidence::default(),
        }
    }

    #[test]
    fn equal_inputs_render_byte_identical_candidates() {
        let first = project(&input(ProfileId::Codebase)).unwrap();
        let second = project(&input(ProfileId::Codebase)).unwrap();
        assert_eq!(first.destinations, second.destinations);
        assert_eq!(first.manifest.to_json(), second.manifest.to_json());
    }

    #[test]
    fn every_profile_lands_its_own_root() {
        for profile in ProfileId::every() {
            let candidate = project(&input(profile)).unwrap();
            assert_eq!(candidate.manifest.profile, profile);
            assert_eq!(candidate.manifest.docs_root, docs_root_of(profile).unwrap());
        }
    }

    #[test]
    fn the_record_is_the_last_file_a_landing_writes() {
        let candidate = project(&input(ProfileId::KnowledgeBase)).unwrap();
        let files = candidate.files();
        assert_eq!(files.last().unwrap().0, Utf8PathBuf::from(MANIFEST_PATH));
        assert_eq!(files.len(), candidate.destinations.len() + 1);
    }

    #[test]
    fn one_documentation_root_serves_the_whole_candidate() {
        let candidate = project(&input(ProfileId::KnowledgeBase)).unwrap();
        let root = candidate.manifest.docs_root;
        for entry in &candidate.manifest.adopted_files {
            if entry.destination == crate::domain::paths::CONFIG_PATH {
                continue;
            }
            assert!(
                entry.destination.as_str().starts_with(&format!("{root}/")),
                "{} is outside the recorded root {root}",
                entry.destination
            );
        }
    }

    #[test]
    fn an_adopted_destination_the_project_wrote_is_kept_and_noted() {
        let mut held = input(ProfileId::KnowledgeBase);
        let candidate = project(&held).unwrap();
        let adopted = candidate
            .destinations
            .iter()
            .find(|destination| {
                destination.ownership == Ownership::Adopted
                    && destination.path != crate::domain::paths::CONFIG_PATH
            })
            .unwrap()
            .clone();
        held.evidence
            .existing
            .insert(adopted.path.clone(), b"the project wrote this".to_vec());

        let second = project(&held).unwrap();
        let kept = second
            .destinations
            .iter()
            .find(|destination| destination.path == adopted.path)
            .unwrap();
        assert_eq!(kept.bytes, b"the project wrote this");
        assert!(
            second
                .notes
                .iter()
                .any(|note| note.contains(adopted.path.as_str()))
        );
    }

    #[test]
    fn a_recorded_adopted_destination_is_kept_without_a_note() {
        let mut held = input(ProfileId::KnowledgeBase);
        let candidate = project(&held).unwrap();
        let adopted = candidate
            .destinations
            .iter()
            .find(|destination| {
                destination.ownership == Ownership::Adopted
                    && destination.path != crate::domain::paths::CONFIG_PATH
            })
            .unwrap()
            .clone();
        held.evidence
            .existing
            .insert(adopted.path.clone(), b"the project wrote this".to_vec());
        held.evidence
            .recorded_adopted
            .push(adopted.path.to_string());

        let second = project(&held).unwrap();
        assert!(second.notes.is_empty(), "{:?}", second.notes);
    }

    #[test]
    fn a_marked_region_preserves_every_byte_outside_it() {
        let mut held = input(ProfileId::Codebase);
        held.evidence.agents_host = "# Project\n\nOur own paragraph.\n".to_string();
        let candidate = project(&held).unwrap();
        let agents = candidate
            .destinations
            .iter()
            .find(|destination| destination.path == AGENTS_DIGEST_PATH)
            .unwrap();
        let text = String::from_utf8(agents.bytes.clone()).unwrap();
        assert!(text.contains("Our own paragraph."));
        assert_eq!(agents.placement, Placement::MarkedRegion);
    }

    #[test]
    fn a_declaration_that_cannot_be_read_refuses_rather_than_defaulting() {
        let mut held = input(ProfileId::KnowledgeBase);
        held.evidence.existing.insert(
            Utf8PathBuf::from(crate::domain::paths::CONFIG_PATH),
            b"\xff\xfe not text".to_vec(),
        );
        let error = project(&held).unwrap_err();
        assert!(error.to_string().contains("not UTF-8"), "{error}");

        held.evidence.existing.insert(
            Utf8PathBuf::from(crate::domain::paths::CONFIG_PATH),
            b"reserved: [".to_vec(),
        );
        let error = project(&held).unwrap_err();
        assert!(error.to_string().contains("does not parse"), "{error}");
    }

    #[test]
    fn every_projected_source_comes_from_the_embedded_inventory() {
        let candidate = project(&input(ProfileId::Codebase)).unwrap();
        for destination in &candidate.destinations {
            let Some(source) = &destination.source else {
                continue;
            };
            assert!(
                crate::embedded::asset(source).is_some(),
                "{source} is projected and not embedded"
            );
        }
    }
}
