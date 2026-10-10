//! A migration that archives the old corpus and rebuilds it from there.
//!
//! The verbs behind `sdd migrate`. `inventory` lists every document the
//! target holds. `archive` moves each one but the root metadata into a
//! workspace under this tool's state root, writes the registry beside it,
//! and records the open migration in the declaration, so the repository
//! reads as greenfield and every clone sees that a migration is open.
//! `mark` moves a source or a group through its statuses, `status` reports
//! where each stands, and `close` ends the migration once every source is
//! verified or dropped.
//!
//! Three copies of every byte exist before a move: Git holds it at the base
//! commit, because the archive refuses a document whose bytes differ from
//! the trunk's copy, the archive holds it with its checksum, and
//! `--rebuild` recreates the archive from Git on any clone. The base commit
//! is the trunk's, so it outlives the branch the migration ran on.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::adapters::history::History;
use crate::domain::instance_config::{
    CONFIG_PATH, InstanceConfig, with_migration, without_migration,
};
use crate::domain::migration::{
    Event, Kind, Marking, Placement, REGISTRY_FILE, REGISTRY_SCHEMA, Registry, SOURCES_DIR, Source,
    Status, kind_of, slug_of,
};
use crate::domain::ownership::Sha256;
use crate::error::AppError;
use crate::landing::classify::Classification;

/// The machine schema every `sdd migrate --json` report declares.
pub const SCHEMA: &str = "sdd.migrate/1";

/// The directory every workspace sits under, inside the state root.
pub const MIGRATIONS_DIR: &str = "migrations";

/// Whether a document's home is one the convention or the corpus names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Home {
    /// Under a documentation root or a methodology marker.
    Recognized,
    /// Anywhere else.
    Unrecognized,
}

/// One document the inventory found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// Where it sits, relative to the target.
    pub path: Utf8PathBuf,
    /// What kind of document it is.
    pub kind: Kind,
    /// The SHA-256 of its bytes.
    pub sha256: String,
    /// Whether the archive moves it or it stays in place.
    pub placement: Placement,
    /// Whether its home is a recognized one.
    pub home: Home,
}

/// Every document a target holds, as `sdd migrate inventory` reports it.
#[derive(Debug, Serialize)]
pub struct Inventory {
    /// The machine schema this object declares.
    pub schema: &'static str,
    /// Every document, in path order.
    pub documents: Vec<Entry>,
}

/// What `sdd migrate archive` did, or would do.
#[derive(Debug, Serialize)]
pub struct ArchiveReport {
    /// The machine schema this object declares.
    pub schema: &'static str,
    /// The migration id the declaration records.
    pub id: String,
    /// Where the workspace is.
    pub workspace: Utf8PathBuf,
    /// The commit that holds every byte the archive moves.
    pub base_commit: String,
    /// Every path the archive moves out of the target.
    pub archived: Vec<Utf8PathBuf>,
    /// Every path that stays in place, registered as a source.
    pub in_place: Vec<Utf8PathBuf>,
    /// Whether anything was written.
    pub applied: bool,
}

/// What `sdd migrate status` reports for one source.
#[derive(Debug, Serialize)]
pub struct SourceState {
    /// The source id.
    pub id: String,
    /// Where it sat at the base commit.
    pub path: Utf8PathBuf,
    /// Where it stands.
    pub status: Status,
    /// The group it is filed in.
    pub group: Option<String>,
    /// Whether its bytes differ from the registry's checksum: the archived
    /// copy for an archived source, the file in place for root metadata.
    pub changed: bool,
}

/// The whole state of an open migration.
#[derive(Debug, Serialize)]
pub struct StatusReport {
    /// The machine schema this object declares.
    pub schema: &'static str,
    /// The migration id.
    pub id: String,
    /// Where the workspace is.
    pub workspace: Utf8PathBuf,
    /// The commit that held every source byte.
    pub base_commit: String,
    /// Every source, in path order.
    pub sources: Vec<SourceState>,
    /// Why the migration cannot close yet. Empty means it can.
    pub unfinished: Vec<String>,
}

/// What `sdd migrate mark` changed.
#[derive(Debug, Serialize)]
pub struct MarkReport {
    /// The machine schema this object declares.
    pub schema: &'static str,
    /// Every source id the mark moved.
    pub marked: Vec<String>,
    /// The status each one took.
    pub status: Status,
}

/// What `sdd migrate close` did, or would do.
#[derive(Debug, Serialize)]
pub struct CloseReport {
    /// The machine schema this object declares.
    pub schema: &'static str,
    /// The migration id.
    pub id: String,
    /// The workspace, which stays: the archive is the operator's to remove.
    pub workspace: Utf8PathBuf,
    /// Whether the declaration key was removed.
    pub applied: bool,
}

/// The documents `target` holds, classified, in path order.
///
/// SATISFIES setup:inventory-lists-every-document
///
/// # Errors
///
/// I/O errors when the tree or a document cannot be read.
pub fn inventory(target: &Utf8Path) -> Result<Inventory, AppError> {
    let corpus =
        crate::services::assess::corpus(target, crate::gates::paths::docs_scratch_variable())?;
    let homes: Vec<String> = corpus
        .doc_roots
        .iter()
        .chain(&corpus.methodology_markers)
        .cloned()
        .collect();
    let mut documents = Vec::new();
    for path in &corpus.documents {
        let bytes = std::fs::read(target.join(path))?;
        documents.push(entry(path, &bytes, &homes));
    }
    Ok(Inventory {
        schema: SCHEMA,
        documents,
    })
}

/// One inventory entry for a document and its bytes.
fn entry(path: &Utf8Path, bytes: &[u8], homes: &[String]) -> Entry {
    let metadata = crate::services::assess::is_root_metadata(path);
    let kind = kind_of(path, metadata);
    let recognized = metadata
        || matches!(kind, Kind::Decision | Kind::Spec)
        || homes.iter().any(|home| path.starts_with(home.as_str()));
    Entry {
        path: path.to_owned(),
        kind,
        sha256: Sha256::of(bytes).to_string(),
        placement: if metadata {
            Placement::InPlace
        } else {
            Placement::Archived
        },
        home: if recognized {
            Home::Recognized
        } else {
            Home::Unrecognized
        },
    }
}

/// The migration the declaration holds open, if any.
///
/// # Errors
///
/// [`AppError::Refused`] naming the declaration where it cannot be read.
pub fn open_migration(target: &Utf8Path) -> Result<Option<String>, AppError> {
    InstanceConfig::read(target)
        .map(|declaration| declaration.migration)
        .map_err(|error| AppError::Refused(error.to_string()))
}

/// Where one migration's workspace is.
///
/// # Errors
///
/// [`AppError::Refused`] where no state root resolves, because the home is
/// not set and no variable names one.
pub fn workspace(target: &Utf8Path, id: &str) -> Result<Utf8PathBuf, AppError> {
    let state = crate::domain::paths::UserEnv::from_process()
        .state_root()
        .ok_or_else(|| {
            AppError::Refused(
                "no state root resolves: set HOME or XDG_STATE_HOME so the archive has a home"
                    .to_string(),
            )
        })?;
    Ok(state
        .path
        .join(MIGRATIONS_DIR)
        .join(crate::stage::stage_name(target))
        .join(id))
}

/// The history the archive rests on, or the refusal that names why none is.
fn history(target: &Utf8Path) -> Result<History, AppError> {
    History::open(target).map_err(|missing| {
        AppError::Refused(format!(
            "{missing}; a migration archives only what Git holds, so commit the corpus in a repository first (setup:an-archive-moves-only-committed-documents)"
        ))
    })
}

/// The time a status change is recorded at, in UTC.
fn now() -> String {
    jiff::Timestamp::now()
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// Move every document but the root metadata into a new workspace.
///
/// Without `apply` this reports what would move and writes nothing.
///
/// SATISFIES setup:an-archive-moves-only-committed-documents
///
/// # Errors
///
/// [`AppError::Refused`] where a migration is already open, the target is
/// not a migration, no trunk resolves, a listed document differs from the
/// trunk's copy, or the instance directory cannot be written through.
/// Nothing is written when it refuses.
pub fn archive(
    target: &Utf8Path,
    trunk: Option<&str>,
    apply: bool,
) -> Result<ArchiveReport, AppError> {
    if let Some(id) = open_migration(target)? {
        return Err(AppError::Refused(format!(
            "the migration {id} is already open; read it with 'sdd migrate status'"
        )));
    }
    let found = crate::services::status::classify_target(target)?.classification;
    if found != Classification::Migration {
        return Err(AppError::Refused(format!(
            "sdd migrate archive serves a migration target, and this target is {found}"
        )));
    }
    let history = history(target)?;
    if history.head().is_none() {
        return Err(AppError::Refused(
            "the repository has no commit, so Git holds none of the corpus; commit it first"
                .to_string(),
        ));
    }
    let trunk = trunk
        .map(str::to_string)
        .or_else(|| history.default_trunk())
        .ok_or_else(|| {
            AppError::Refused(
                "the remote's HEAD names no local branch; name the trunk with --trunk <ref>"
                    .to_string(),
            )
        })?;
    let base_commit = history.commit_of(&trunk).ok_or_else(|| {
        AppError::Refused(format!(
            "the trunk {trunk} names no commit; name the trunk with --trunk <ref>"
        ))
    })?;
    let listed = inventory(target)?.documents;
    let paths: Vec<&Utf8Path> = listed.iter().map(|entry| entry.path.as_path()).collect();
    // A document the trunk holds and the target lacks was deleted off the
    // trunk: that is an edit too, and it integrates before the archive.
    let mut differing = history.differing(&base_commit, &paths)?;
    differing.extend(
        documents_at(target, &history, &base_commit)?
            .into_iter()
            .filter(|path| !paths.contains(&path.as_path())),
    );
    differing.sort();
    if !differing.is_empty() {
        let names: Vec<String> = differing.iter().map(ToString::to_string).collect();
        return Err(AppError::Refused(format!(
            "these documents differ from the trunk's copy on {trunk}: {}; integrate their edits onto the trunk, then run the archive again (setup:an-archive-moves-only-committed-documents)",
            names.join(", ")
        )));
    }
    for written in [CONFIG_PATH, crate::domain::manifest::MANIFEST_PATH] {
        crate::adapters::fs::check_destination(target, Utf8Path::new(written)).map_err(
            |refusal| {
                AppError::Refused(format!(
                    "{written}: {refusal}; the archive writes nothing through it"
                ))
            },
        )?;
    }
    let stamp = jiff::Timestamp::now()
        .strftime("%Y%m%dT%H%M%SZ")
        .to_string();
    // An apply claims its workspace exclusively, so two archives in one
    // second never share one; a preview only names where it would land.
    let planned = workspace(target, &stamp)?;
    let workspace = match planned.parent() {
        Some(runs) if apply => crate::adapters::fs::claim_run_dir(runs, &stamp)?,
        _ => planned,
    };
    let id = workspace.file_name().unwrap_or(stamp.as_str()).to_string();
    let (archived, in_place): (Vec<&Entry>, Vec<&Entry>) = listed
        .iter()
        .partition(|entry| entry.placement == Placement::Archived);
    let report = ArchiveReport {
        schema: SCHEMA,
        id: id.clone(),
        workspace: workspace.clone(),
        base_commit: base_commit.clone(),
        archived: archived.iter().map(|entry| entry.path.clone()).collect(),
        in_place: in_place.iter().map(|entry| entry.path.clone()).collect(),
        applied: apply,
    };
    if !apply {
        return Ok(report);
    }
    for entry in &archived {
        let bytes = std::fs::read(target.join(&entry.path))?;
        let copy = workspace.join(SOURCES_DIR).join(&entry.path);
        crate::adapters::fs::write_atomic(&copy, &bytes)?;
        if Sha256::of(&std::fs::read(&copy)?).to_string() != entry.sha256 {
            return Err(AppError::Refused(format!(
                "{copy} does not read back as the bytes it was copied from; nothing was moved"
            )));
        }
    }
    let registry = new_registry(&id, &base_commit, &listed);
    write_registry(&workspace, &registry)?;
    write_declaration(target, |text| with_migration(text, &id))?;
    for entry in &archived {
        crate::adapters::fs::remove_within(target, &entry.path)?;
        prune_empty_parents(target, &entry.path);
    }
    Ok(report)
}

/// A registry with every listed document pending.
///
/// Each source id is the slug of its path. Paths are taken in byte order,
/// so where two slugs collide the first path keeps the bare slug and each
/// later one takes the next free `-N`, the same on every rebuild.
fn new_registry(id: &str, base_commit: &str, listed: &[Entry]) -> Registry {
    let at = now();
    let mut ordered: Vec<&Entry> = listed.iter().collect();
    ordered.sort_by(|left, right| left.path.as_str().cmp(right.path.as_str()));
    let mut sources: Vec<Source> = Vec::new();
    for entry in ordered {
        let mut slug = slug_of(&entry.path);
        let mut suffix = 2;
        while sources.iter().any(|source| source.id == slug) {
            slug = format!("{}-{suffix}", slug_of(&entry.path));
            suffix += 1;
        }
        sources.push(Source {
            id: slug,
            path: entry.path.clone(),
            sha256: entry.sha256.clone(),
            kind: entry.kind,
            placement: entry.placement,
            tags: Vec::new(),
            group: None,
            status: Status::Pending,
            history: vec![Event {
                at: at.clone(),
                status: Status::Pending,
                reason: None,
            }],
        });
    }
    Registry {
        schema_version: REGISTRY_SCHEMA,
        id: id.to_string(),
        base_commit: base_commit.to_string(),
        sources,
        groups: Vec::new(),
    }
}

/// Rewrite the declaration through `change`, creating it where it is
/// absent, and keep the instance's record of it current.
fn write_declaration(target: &Utf8Path, change: impl Fn(&str) -> String) -> Result<(), AppError> {
    let path = target.join(CONFIG_PATH);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(AppError::Io(error)),
    };
    let changed = change(&text);
    crate::adapters::fs::write_within(target, Utf8Path::new(CONFIG_PATH), changed.as_bytes())?;
    rerecord_declaration(target, changed.as_bytes())
}

/// Update the instance's record of the declaration, where an instance is
/// landed, so a verb that wrote it leaves no drift behind.
fn rerecord_declaration(target: &Utf8Path, bytes: &[u8]) -> Result<(), AppError> {
    let manifest = target.join(crate::domain::manifest::MANIFEST_PATH);
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return Ok(());
    };
    let mut document: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;
    let recorded = document
        .get("adopted_files")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry.get("destination").and_then(serde_json::Value::as_str) == Some(CONFIG_PATH)
            })
        });
    if !recorded {
        return Ok(());
    }
    crate::services::policy::with_adopted_record(
        &mut document,
        "",
        Utf8Path::new(CONFIG_PATH),
        bytes,
        &[],
    )?;
    let rendered = serde_json::to_string_pretty(&document)
        .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;
    crate::adapters::fs::write_within(
        target,
        Utf8Path::new(crate::domain::manifest::MANIFEST_PATH),
        format!("{rendered}\n").as_bytes(),
    )?;
    Ok(())
}

/// Remove each parent directory of `path` the move left empty, up to the
/// target, so the archive leaves no hollow tree behind.
fn prune_empty_parents(target: &Utf8Path, path: &Utf8Path) {
    let mut parent = path.parent();
    while let Some(directory) = parent.filter(|directory| !directory.as_str().is_empty()) {
        if std::fs::remove_dir(target.join(directory)).is_err() {
            break;
        }
        parent = directory.parent();
    }
}

/// Write the registry into the workspace.
fn write_registry(workspace: &Utf8Path, registry: &Registry) -> Result<(), AppError> {
    let text = registry
        .render()
        .map_err(|error| AppError::Refused(error.to_string()))?;
    crate::adapters::fs::write_atomic(&workspace.join(REGISTRY_FILE), text.as_bytes())?;
    Ok(())
}

/// The open migration's id, workspace, and registry.
///
/// # Errors
///
/// [`AppError::Refused`] where no migration is open or the registry cannot
/// be read.
fn opened(target: &Utf8Path) -> Result<(String, Utf8PathBuf, Registry), AppError> {
    let id = open_migration(target)?.ok_or_else(|| {
        AppError::Refused("no migration is open here; 'sdd migrate archive' opens one".to_string())
    })?;
    let workspace = workspace(target, &id)?;
    let text = std::fs::read_to_string(workspace.join(REGISTRY_FILE)).map_err(|error| {
        AppError::Refused(format!(
            "the registry of {id} cannot be read under {workspace}: {error}; 'sdd migrate archive --rebuild --apply' recreates the archive from Git"
        ))
    })?;
    let registry = Registry::parse(&text).map_err(|error| AppError::Refused(error.to_string()))?;
    Ok((id, workspace, registry))
}

/// Recreate the open migration's archive from Git at its base commit.
///
/// The registry is read where the workspace holds one. On a clone that has
/// none, the base commit is the parent of the commit that recorded the
/// migration in the declaration, and the registry is rebuilt from the
/// documents that commit holds, every one pending again.
///
/// SATISFIES setup:an-archive-rebuilds-from-git
///
/// # Errors
///
/// [`AppError::Refused`] where no migration is open, the history does not
/// hold the base commit, or a recreated byte does not match its checksum.
pub fn rebuild(target: &Utf8Path, apply: bool) -> Result<ArchiveReport, AppError> {
    let id = open_migration(target)?.ok_or_else(|| {
        AppError::Refused("no migration is open here, so there is nothing to rebuild".to_string())
    })?;
    let workspace = workspace(target, &id)?;
    let history = history(target)?;
    let registry = if let Ok(text) = std::fs::read_to_string(workspace.join(REGISTRY_FILE)) {
        Registry::parse(&text).map_err(|error| AppError::Refused(error.to_string()))?
    } else {
        reconstructed(target, &history, &id)?
    };
    let archived: Vec<&Source> = registry
        .sources
        .iter()
        .filter(|source| source.placement == Placement::Archived)
        .collect();
    let mut recreated = Vec::new();
    for source in &archived {
        let bytes = history
            .show(&registry.base_commit, &source.path)
            .ok_or_else(|| {
                AppError::Refused(format!(
                    "{} holds no {}; fetch the history that holds it",
                    registry.base_commit, source.path
                ))
            })?;
        if Sha256::of(&bytes).to_string() != source.sha256 {
            return Err(AppError::Refused(format!(
                "{} at {} does not match its registered checksum",
                source.path, registry.base_commit
            )));
        }
        recreated.push((source.path.clone(), bytes));
    }
    if apply {
        for (path, bytes) in &recreated {
            crate::adapters::fs::write_atomic(&workspace.join(SOURCES_DIR).join(path), bytes)?;
        }
        if !workspace.join(REGISTRY_FILE).is_file() {
            write_registry(&workspace, &registry)?;
        }
    }
    Ok(ArchiveReport {
        schema: SCHEMA,
        id,
        workspace,
        base_commit: registry.base_commit.clone(),
        archived: archived.iter().map(|source| source.path.clone()).collect(),
        in_place: registry
            .sources
            .iter()
            .filter(|source| source.placement == Placement::InPlace)
            .map(|source| source.path.clone())
            .collect(),
        applied: apply,
    })
}

/// The documents `commit` holds, selected the way an inventory selects them
/// from a tree: regular files a walk would count, outside the docs scratch.
fn documents_at(
    target: &Utf8Path,
    history: &History,
    commit: &str,
) -> Result<Vec<Utf8PathBuf>, AppError> {
    let scratch = crate::services::assess::scratch_within(
        target,
        crate::gates::paths::docs_scratch_variable(),
    );
    Ok(history
        .files_at(commit)?
        .into_iter()
        .filter(|path| {
            crate::services::assess::is_document_path(path)
                && !scratch
                    .as_ref()
                    .is_some_and(|scratch| path.starts_with(scratch))
        })
        .collect())
}

/// The registry of a migration whose workspace this host does not hold,
/// rebuilt from the commit before the one that recorded it.
///
/// The sources are selected the way an inventory selects them: regular
/// files a walk would count, outside the docs scratch. The scratch is the
/// one that resolves today, because only the lost registry recorded which
/// documents the archive took.
fn reconstructed(target: &Utf8Path, history: &History, id: &str) -> Result<Registry, AppError> {
    let base = history
        .before_adding(
            Utf8Path::new(CONFIG_PATH),
            &crate::domain::instance_config::migration_line(id),
        )
        .ok_or_else(|| {
            AppError::Refused(format!(
                "no commit records the migration {id} in {CONFIG_PATH}, so the base commit cannot be found"
            ))
        })?;
    let mut listed = Vec::new();
    for path in documents_at(target, history, &base)? {
        if let Some(bytes) = history.show(&base, &path) {
            listed.push(entry(&path, &bytes, &[]));
        }
    }
    Ok(new_registry(id, &base, &listed))
}

/// Move a source, or every source of a group, to `status`.
///
/// SATISFIES setup:mark-appends-and-never-rewrites
///
/// # Errors
///
/// [`AppError::Usage`] where a destination leaves the target, and
/// [`AppError::Refused`] where no migration is open or the registry
/// refuses the change. The registry is unchanged when either refuses.
pub fn mark(
    target: &Utf8Path,
    name: &str,
    status: Status,
    marking: &Marking,
) -> Result<MarkReport, AppError> {
    // Close judges each destination under the target, so one that names
    // an absolute path or climbs out of it would judge a foreign file.
    for destination in &marking.destinations {
        let inside = !destination.as_str().is_empty()
            && destination.components().all(|part| {
                matches!(
                    part,
                    camino::Utf8Component::Normal(_) | camino::Utf8Component::CurDir
                )
            });
        if !inside {
            return Err(AppError::Usage(format!(
                "the destination {destination} leaves the target; name a path relative to the target root"
            )));
        }
    }
    let (_, workspace, mut registry) = opened(target)?;
    let marked = registry
        .mark(name, status, marking, &now())
        .map_err(|error| AppError::Refused(error.to_string()))?;
    write_registry(&workspace, &registry)?;
    Ok(MarkReport {
        schema: SCHEMA,
        marked,
        status,
    })
}

/// Where every source of the open migration stands.
///
/// # Errors
///
/// [`AppError::Refused`] where no migration is open, and the I/O error of a
/// held copy that exists and cannot be read: an unreadable copy is not a
/// changed one.
pub fn status(target: &Utf8Path) -> Result<StatusReport, AppError> {
    let (id, workspace, registry) = opened(target)?;
    let sources = registry
        .sources
        .iter()
        .map(|source| {
            let held = match source.placement {
                Placement::Archived => workspace.join(SOURCES_DIR).join(&source.path),
                Placement::InPlace => target.join(&source.path),
            };
            let changed = match std::fs::read(&held) {
                Ok(bytes) => Sha256::of(&bytes).to_string() != source.sha256,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
                Err(error) => return Err(AppError::Io(error)),
            };
            Ok(SourceState {
                id: source.id.clone(),
                path: source.path.clone(),
                status: source.status,
                group: source.group.clone(),
                changed,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let root = target.canonicalize_utf8()?;
    Ok(StatusReport {
        schema: SCHEMA,
        id,
        workspace,
        base_commit: registry.base_commit.clone(),
        sources,
        unfinished: registry.unfinished(|path| holds(&root, target, path)),
    })
}

/// Whether the target holds `destination` as a file inside it. A link that
/// leads out of the target is a file the target does not hold.
fn holds(root: &Utf8Path, target: &Utf8Path, destination: &Utf8Path) -> bool {
    target
        .join(destination)
        .canonicalize_utf8()
        .is_ok_and(|held| held.starts_with(root) && held.is_file())
}

/// End the open migration: remove the declaration key once every source is
/// verified or dropped. The workspace stays for the operator to remove.
///
/// SATISFIES setup:close-refuses-an-unfinished-source
///
/// # Errors
///
/// [`AppError::Refused`] naming every unfinished source. Nothing is written
/// when it refuses.
pub fn close(target: &Utf8Path, apply: bool) -> Result<CloseReport, AppError> {
    let (id, workspace, registry) = opened(target)?;
    let root = target.canonicalize_utf8()?;
    let unfinished = registry.unfinished(|path| holds(&root, target, path));
    if !unfinished.is_empty() {
        return Err(AppError::Refused(format!(
            "the migration {id} cannot close: {} (setup:close-refuses-an-unfinished-source)",
            unfinished.join("; ")
        )));
    }
    if apply {
        write_declaration(target, without_migration)?;
    }
    Ok(CloseReport {
        schema: SCHEMA,
        id,
        workspace,
        applied: apply,
    })
}
