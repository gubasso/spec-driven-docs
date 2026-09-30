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
use crate::domain::profile::{DocsRoot, ProfileId, render_root, resolve_destination};
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::services::hooks_render::{RenderOptions, render_block};

/// The title a root `AGENTS.md` this landing creates opens with.
pub const AGENTS_TITLE: &str = "# AGENTS";

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
    /// Whether the root author-instructions file exists at all.
    ///
    /// An absent file and an existing empty one both leave `agents_host`
    /// empty, and only the absent one is created with a title.
    pub agents_exists: bool,
    /// Which names of every declared yield set exist at the target root.
    pub present_at_root: Vec<String>,
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
        let path = resolve_destination(&projection.destination, docs_root);
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
        let seed = rendered_seed(&projection.source, docs_root)?;
        let path = resolve_destination(&projection.destination, docs_root);
        let recorded = input
            .evidence
            .recorded_adopted
            .iter()
            .any(|recorded| recorded == path.as_str());
        // A seed the record already attributes keeps its entry, its held
        // bytes, and its baseline, like any other adopted file. The yield
        // decides only whether to introduce one, and the seed's own name is
        // in the set it yields to, so asking first would stop owning it at
        // the second landing.
        if !recorded
            && let Some(cause) = projection.yield_set().and_then(|set| {
                set.names().find(|name| {
                    input
                        .evidence
                        .present_at_root
                        .iter()
                        .any(|held| held == name)
                })
            })
        {
            notes.push(format!(
                "note: {path} was not seeded, because the target root already holds {cause}; that configuration stays the project's, and 'sdd method gates' states what it needs to load the relative-links rule"
            ));
            continue;
        }
        let held = input.evidence.existing.get(&path);
        if let Some(held) = held
            && held != &seed
            && !recorded
        {
            notes.push(format!(
                "note: {path} already exists and is kept; the seed was not written, so read it with 'sdd spec' and reconcile by hand"
            ));
        }
        let mut bytes = held.cloned().unwrap_or_else(|| seed.clone());
        // `--reserve` and `--writing-style` record into the declaration,
        // keeping its comments and whatever the project already wrote there.
        // What is already there is read first: a transformation applied to
        // bytes nobody validated would repair a malformed declaration into
        // a valid one and overwrite the file the operator has to fix.
        if path == crate::domain::instance_config::CONFIG_PATH {
            let text = readable(&path, &bytes)?;
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
    // The declaration the block is rendered from is the one this landing
    // writes, already validated above.
    let declared = destinations
        .iter()
        .find(|destination| destination.path == crate::domain::instance_config::CONFIG_PATH);
    let declaration = match declared {
        Some(destination) => {
            let text = readable(&destination.path, &destination.bytes)?;
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
    let mut agents =
        crate::domain::marker::place_agents_block(&input.evidence.agents_host, &agents_block)?;
    // A file this landing creates opens with a title, because a Markdown
    // file whose first line is not one fails the linter's default. A host
    // that exists, even an empty one, is the project's, and gets none.
    if !input.evidence.agents_exists {
        agents = format!("{AGENTS_TITLE}\n\n{agents}");
    }
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

/// One declaration's text, refusing bytes no reader can take.
///
/// A declaration that cannot be read is a refusal, never a default and
/// never something a flag repairs on the way past: the landing would
/// otherwise keep the project's bytes and wire the block from something
/// else, and the two would disagree from the first commit onward.
fn readable<'a>(path: &Utf8PathBuf, bytes: &'a [u8]) -> Result<&'a str, AppError> {
    let text = std::str::from_utf8(bytes).map_err(|source| {
        AppError::Refused(format!(
            "{path} is not UTF-8, so what the gates judge cannot be read: {source}"
        ))
    })?;
    InstanceConfig::parse(text)
        .map_err(|error| AppError::Refused(format!("{path} does not parse: {error}")))?;
    Ok(text)
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

/// One adopted seed as it lands for a documentation root.
///
/// A seed names the instance's root by the placeholder, and the landing is
/// the one moment that knows the root, so the landed bytes, the held-bytes
/// comparison, and the recorded baseline all use this rendering. A seed
/// that is not text carries no placeholder and lands as it is.
///
/// SATISFIES staging:a-seed-lands-rendered-for-its-root
///
/// # Errors
///
/// [`AppError::Refused`] where this release does not carry the source.
pub fn rendered_seed(source: &str, docs_root: DocsRoot) -> Result<Vec<u8>, AppError> {
    let bytes = source_bytes(source)?;
    let rendered = std::str::from_utf8(&bytes)
        .ok()
        .map(|text| render_root(text, docs_root.as_str()).into_bytes());
    Ok(rendered.unwrap_or(bytes))
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
        let root_seeds: Vec<&str> = crate::domain::profile::DECLARATION
            .adopted
            .iter()
            .filter(|projection| projection.yield_set().is_some())
            .map(|projection| projection.destination.as_str())
            .collect();
        assert_eq!(root_seeds, [".markdownlint-cli2.jsonc"]);
        for entry in candidate
            .manifest
            .managed_files
            .iter()
            .map(|entry| &entry.destination)
            .chain(
                candidate
                    .manifest
                    .adopted_files
                    .iter()
                    .map(|entry| &entry.destination),
            )
        {
            if entry == crate::domain::paths::CONFIG_PATH || root_seeds.contains(&entry.as_str()) {
                continue;
            }
            assert!(
                entry.as_str().starts_with(&format!("{root}/")),
                "{entry} is outside the recorded root {root}"
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
        held.evidence.agents_exists = true;
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

        // And a flag does not repair it on the way past. A transformation
        // over bytes nobody validated would rewrite the file the operator
        // has to fix and call the result a landing.
        held.reserve = vec!["vendor/**".to_string()];
        let error = project(&held).unwrap_err();
        assert!(error.to_string().contains("does not parse"), "{error}");
    }

    const ROOT_SEED: &str = ".markdownlint-cli2.jsonc";

    #[test]
    fn a_yielding_seed_the_record_does_not_name_is_not_introduced() {
        let mut held = input(ProfileId::Codebase);
        held.evidence.present_at_root = vec![".markdownlint.yaml".to_string()];
        let candidate = project(&held).unwrap();
        assert!(
            !candidate
                .destinations
                .iter()
                .any(|destination| destination.path == ROOT_SEED)
        );
        assert!(
            !candidate
                .manifest
                .adopted_files
                .iter()
                .any(|entry| entry.destination == ROOT_SEED)
        );
        assert!(
            candidate
                .notes
                .iter()
                .any(|note| note.contains(&format!("{ROOT_SEED} was not seeded"))
                    && note.contains("already holds .markdownlint.yaml")),
            "{:?}",
            candidate.notes
        );
    }

    #[test]
    fn a_yielding_seed_the_record_names_stays_adopted() {
        let mut held = input(ProfileId::Codebase);
        let seed = rendered_seed("instance/seeds/markdownlint-cli2.jsonc", DocsRoot::Docs).unwrap();
        // The seed's own name is in the set it yields to, which is why the
        // record is asked first.
        held.evidence.present_at_root = vec![ROOT_SEED.to_string()];
        held.evidence.existing.insert(
            Utf8PathBuf::from(ROOT_SEED),
            b"{ \"edited\": true }\n".to_vec(),
        );
        held.evidence.recorded_adopted.push(ROOT_SEED.to_string());

        let candidate = project(&held).unwrap();
        let kept = candidate
            .destinations
            .iter()
            .find(|destination| destination.path == ROOT_SEED)
            .unwrap();
        assert_eq!(kept.bytes, b"{ \"edited\": true }\n");
        assert_eq!(kept.ownership, Ownership::Adopted);
        let entry = candidate
            .manifest
            .adopted_files
            .iter()
            .find(|entry| entry.destination == ROOT_SEED)
            .unwrap();
        assert_eq!(entry.baseline_sha256, Sha256::of(&seed));
        assert!(candidate.notes.is_empty(), "{:?}", candidate.notes);
    }

    fn agents_bytes(candidate: &Candidate) -> String {
        let agents = candidate
            .destinations
            .iter()
            .find(|destination| destination.path == AGENTS_DIGEST_PATH)
            .unwrap();
        String::from_utf8(agents.bytes.clone()).unwrap()
    }

    #[test]
    fn only_an_absent_host_receives_the_title() {
        let absent = project(&input(ProfileId::Codebase)).unwrap();
        let text = agents_bytes(&absent);
        assert!(
            text.starts_with(&format!("{AGENTS_TITLE}\n\n<!-- BEGIN")),
            "{text}"
        );

        let mut empty = input(ProfileId::Codebase);
        empty.evidence.agents_exists = true;
        let text = agents_bytes(&project(&empty).unwrap());
        assert!(text.starts_with("<!-- BEGIN"), "{text}");
        assert!(!text.contains(AGENTS_TITLE), "{text}");
        assert_ne!(agents_bytes(&absent), text);
    }

    #[test]
    fn an_unmarked_documentation_section_is_kept_and_noted_without_a_title() {
        let mut held = input(ProfileId::Codebase);
        held.evidence.agents_exists = true;
        held.evidence.agents_host = "## Documentation\n\n- Our own routing.\n".to_string();
        let candidate = project(&held).unwrap();
        let text = agents_bytes(&candidate);
        assert!(
            text.starts_with("## Documentation\n\n- Our own routing.\n\n<!-- BEGIN"),
            "{text}"
        );
        assert!(!text.contains(AGENTS_TITLE), "{text}");
        assert!(
            candidate
                .notes
                .iter()
                .any(|note| note.contains("unmarked '## Documentation' section")),
            "{:?}",
            candidate.notes
        );
    }

    #[test]
    fn a_seed_lands_rendered_for_its_root() {
        let candidate = project(&input(ProfileId::Codebase)).unwrap();
        let registry = candidate
            .destinations
            .iter()
            .find(|destination| destination.path == "docs/reference/tracking.yaml")
            .unwrap();
        let text = String::from_utf8(registry.bytes.clone()).unwrap();
        assert!(
            text.contains("path: docs/reference/model-pricing.md"),
            "{text}"
        );
        assert!(
            !text.contains(crate::domain::profile::DOCS_ROOT_PLACEHOLDER),
            "{text}"
        );
        let recorded = candidate
            .manifest
            .adopted_files
            .iter()
            .find(|entry| entry.destination == "docs/reference/tracking.yaml")
            .unwrap();
        assert_eq!(recorded.baseline_sha256, Sha256::of(&registry.bytes));
        assert_ne!(
            recorded.baseline_sha256,
            Sha256::of(&source_bytes("templates/TEMPLATE-tracking.yaml").unwrap())
        );
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
