//! The registry an open migration keeps, and the rules that move it.
//!
//! A migration archives a corpus, lands a greenfield instance, and rebuilds
//! the documentation from the archive over many runs. The registry is the
//! one record of that work: every source with its checksum, its kind, the
//! group the agent filed it in, and a dated history of each status it took.
//! Only the `sdd migrate` verbs write it, so every entry in that history is
//! one the binary appended.
//!
//! This module is pure: it parses, renders, and changes a registry in
//! memory. Where the registry lives and what touches the disk is the
//! migration service's business.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

/// The file the registry is written to, inside the workspace.
pub const REGISTRY_FILE: &str = "registry.yaml";
/// The directory the archived sources sit under, inside the workspace.
pub const SOURCES_DIR: &str = "sources";
/// The registry shape this binary reads and writes.
pub const REGISTRY_SCHEMA: u32 = 1;

/// Where one source stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Archived or registered, and not yet mapped.
    Pending,
    /// Filed into a group whose destinations the operator approved.
    Mapped,
    /// Its destinations are written.
    Written,
    /// Its destinations are written and read against the source.
    Verified,
    /// Deliberately carried nowhere, with the reason recorded.
    Dropped,
}

impl Status {
    /// The word the command line and the registry spell.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Mapped => "mapped",
            Self::Written => "written",
            Self::Verified => "verified",
            Self::Dropped => "dropped",
        }
    }

    /// Whether a source in this status is finished.
    #[must_use]
    pub const fn finished(self) -> bool {
        matches!(self, Self::Verified | Self::Dropped)
    }
}

/// What kind of document a source is, read from its name and its home.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A decision record of a previous convention.
    Decision,
    /// A specification.
    Spec,
    /// A step-by-step guide.
    Guide,
    /// Root metadata the project needs during the migration.
    Metadata,
    /// Any other document.
    Document,
}

/// Whether a source moves into the archive or is rewritten where it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Placement {
    /// Moved into the archive, and rebuilt from there.
    Archived,
    /// Root metadata the project still needs, rewritten in place.
    InPlace,
}

/// One dated status change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// When, in UTC.
    pub at: String,
    /// The status the source took.
    pub status: Status,
    /// Why, where the status is `dropped`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One source document of the previous corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// A slug unique within the registry.
    pub id: String,
    /// Where it sat in the repository at the base commit.
    pub path: Utf8PathBuf,
    /// The SHA-256 of its bytes at the base commit.
    pub sha256: String,
    /// What kind of document it is.
    pub kind: Kind,
    /// Whether it was archived or stays in place.
    pub placement: Placement,
    /// The topics the agent tagged it with.
    #[serde(default)]
    pub tags: Vec<String>,
    /// The group the agent filed it in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Where it stands now.
    pub status: Status,
    /// Every status it took, oldest first. Never rewritten.
    #[serde(default)]
    pub history: Vec<Event>,
}

/// Sources the agent reads into one context and rebuilds together.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// A slug unique within the registry.
    pub id: String,
    /// The source ids filed in it.
    #[serde(default)]
    pub sources: Vec<String>,
    /// The documents of the instance the group becomes.
    #[serde(default)]
    pub destinations: Vec<Utf8PathBuf>,
}

/// The whole registry of one open migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    /// The registry shape.
    pub schema_version: u32,
    /// The migration id the declaration names.
    pub id: String,
    /// The commit that held every source byte when the archive ran.
    pub base_commit: String,
    /// Every source, in path order.
    pub sources: Vec<Source>,
    /// Every group, in the order the agent filed them.
    #[serde(default)]
    pub groups: Vec<Group>,
}

/// A registry change this binary refuses.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// The file is not a registry of this shape.
    #[error("the registry does not parse: {0}")]
    Shape(String),
    /// No source and no group carries this id or path.
    #[error("the registry names no source or group `{0}`")]
    Unknown(String),
    /// A drop with no reason.
    #[error("a source is dropped with its reason: pass --reason")]
    NoReason,
    /// A group flag on a group, or a destination on a single source.
    #[error("{0}")]
    Misplaced(String),
    /// An id that is not a slug.
    #[error("`{0}` is not a slug of lowercase letters, digits, and hyphens")]
    NotASlug(String),
}

/// What one `mark` asks for, beyond the status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Marking {
    /// File the source into this group, creating the group where it is new.
    pub group: Option<String>,
    /// Tag the source with each of these topics.
    pub tags: Vec<String>,
    /// Give the group each of these destinations.
    pub destinations: Vec<Utf8PathBuf>,
    /// Why the source is dropped.
    pub reason: Option<String>,
}

/// Whether `value` is a lowercase slug.
#[must_use]
pub fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The slug a path reads as: lowercase, every other character a hyphen.
#[must_use]
pub fn slug_of(path: &Utf8Path) -> String {
    let mut out = String::new();
    for character in path.as_str().chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// The kind a document is, read from its name and the directories it sits
/// in. Root metadata is read by the caller, which knows the metadata names.
#[must_use]
pub fn kind_of(path: &Utf8Path, metadata: bool) -> Kind {
    if metadata {
        return Kind::Metadata;
    }
    let name = path.file_name().unwrap_or_default();
    let within = |names: &[&str]| {
        path.parent().is_some_and(|parent| {
            parent
                .components()
                .any(|part| names.contains(&part.as_str().to_ascii_lowercase().as_str()))
        })
    };
    let shaped = |prefix: &str| {
        name.starts_with(prefix) && path.extension().is_some_and(|extension| extension == "md")
    };
    if shaped("ADR-") || within(&["decisions", "adr", "adrs"]) {
        Kind::Decision
    } else if shaped("SPEC-") || within(&["specs"]) {
        Kind::Spec
    } else if within(&["guides"]) {
        Kind::Guide
    } else {
        Kind::Document
    }
}

impl Registry {
    /// Parse a registry.
    ///
    /// # Errors
    ///
    /// [`RegistryError::Shape`] when the text is not a registry of this
    /// shape.
    pub fn parse(text: &str) -> Result<Self, RegistryError> {
        let parsed: Self =
            yaml_serde::from_str(text).map_err(|error| RegistryError::Shape(error.to_string()))?;
        if parsed.schema_version != REGISTRY_SCHEMA {
            return Err(RegistryError::Shape(format!(
                "schema_version {} is not {REGISTRY_SCHEMA}",
                parsed.schema_version
            )));
        }
        Ok(parsed)
    }

    /// The registry as the file spells it.
    ///
    /// # Errors
    ///
    /// [`RegistryError::Shape`] where the registry cannot be rendered.
    pub fn render(&self) -> Result<String, RegistryError> {
        yaml_serde::to_string(self).map_err(|error| RegistryError::Shape(error.to_string()))
    }

    /// Apply one `mark`: set the status of a source, or of every source in a
    /// group, and append a dated event to each.
    ///
    /// A group flag and tags apply to a single source. Destinations apply to
    /// a group, because a group is what becomes the documents. An earlier
    /// event is never touched.
    ///
    /// # Errors
    ///
    /// [`RegistryError`] naming what the request cannot mean. Nothing is
    /// changed when it refuses.
    pub fn mark(
        &mut self,
        target: &str,
        status: Status,
        marking: &Marking,
        at: &str,
    ) -> Result<Vec<String>, RegistryError> {
        if status == Status::Dropped && marking.reason.is_none() {
            return Err(RegistryError::NoReason);
        }
        for slug in marking.group.iter().chain(&marking.tags) {
            if !is_slug(slug) {
                return Err(RegistryError::NotASlug(slug.clone()));
            }
        }
        let event = Event {
            at: at.to_string(),
            status,
            reason: marking.reason.clone(),
        };
        if let Some(index) = self.groups.iter().position(|group| group.id == target) {
            if marking.group.is_some() || !marking.tags.is_empty() {
                return Err(RegistryError::Misplaced(format!(
                    "`{target}` is a group; --group and --tag file a single source"
                )));
            }
            for destination in &marking.destinations {
                if !self.groups[index].destinations.contains(destination) {
                    self.groups[index].destinations.push(destination.clone());
                }
            }
            let members = self.groups[index].sources.clone();
            for source in &mut self.sources {
                if members.contains(&source.id) {
                    source.status = status;
                    source.history.push(event.clone());
                }
            }
            return Ok(members);
        }
        let index = self
            .sources
            .iter()
            .position(|source| source.id == target || source.path.as_str() == target)
            .ok_or_else(|| RegistryError::Unknown(target.to_string()))?;
        if !marking.destinations.is_empty() {
            return Err(RegistryError::Misplaced(format!(
                "`{target}` is a source; --destination belongs to its group"
            )));
        }
        let id = self.sources[index].id.clone();
        if let Some(group) = &marking.group {
            for existing in &mut self.groups {
                existing.sources.retain(|member| *member != id);
            }
            match self
                .groups
                .iter_mut()
                .find(|existing| existing.id == *group)
            {
                Some(existing) => existing.sources.push(id.clone()),
                None => self.groups.push(Group {
                    id: group.clone(),
                    sources: vec![id.clone()],
                    destinations: Vec::new(),
                }),
            }
            self.sources[index].group = Some(group.clone());
        }
        let source = &mut self.sources[index];
        for tag in &marking.tags {
            if !source.tags.contains(tag) {
                source.tags.push(tag.clone());
            }
        }
        source.status = status;
        source.history.push(event);
        Ok(vec![id])
    }

    /// Why the migration cannot close yet, one line per reason.
    ///
    /// A source is finished when it is verified or dropped. A verified
    /// source needs a group whose every destination exists, because a fact
    /// with no destination is not migrated.
    #[must_use]
    pub fn unfinished(&self, exists: impl Fn(&Utf8Path) -> bool) -> Vec<String> {
        let mut reasons = Vec::new();
        for source in &self.sources {
            if !source.status.finished() {
                reasons.push(format!("{} is {}", source.id, source.status.as_str()));
                continue;
            }
            if source.status == Status::Dropped {
                continue;
            }
            let group = source
                .group
                .as_ref()
                .and_then(|id| self.groups.iter().find(|group| group.id == *id));
            match group {
                None => reasons.push(format!("{} is verified and filed in no group", source.id)),
                Some(group) if group.destinations.is_empty() => reasons.push(format!(
                    "{} is verified and its group {} names no destination",
                    source.id, group.id
                )),
                Some(group) => {
                    for destination in &group.destinations {
                        if !exists(destination) {
                            reasons.push(format!(
                                "{}: the destination {destination} does not exist",
                                group.id
                            ));
                        }
                    }
                }
            }
        }
        reasons.sort();
        reasons.dedup();
        reasons
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        let source = |id: &str, path: &str| Source {
            id: id.to_string(),
            path: Utf8PathBuf::from(path),
            sha256: "00".to_string(),
            kind: Kind::Decision,
            placement: Placement::Archived,
            tags: Vec::new(),
            group: None,
            status: Status::Pending,
            history: Vec::new(),
        };
        Registry {
            schema_version: REGISTRY_SCHEMA,
            id: "20261009T213000Z".to_string(),
            base_commit: "4f1c2e0".to_string(),
            sources: vec![
                source("docs-adr-0003-md", "docs/adr/0003.md"),
                source("docs-adr-0007-md", "docs/adr/0007.md"),
            ],
            groups: Vec::new(),
        }
    }

    #[test]
    fn a_path_reads_as_one_slug() {
        assert_eq!(
            slug_of(Utf8Path::new("docs/adr/0007-Use Postgres.md")),
            "docs-adr-0007-use-postgres-md"
        );
    }

    #[test]
    fn a_kind_is_read_from_the_name_and_the_home() {
        assert_eq!(
            kind_of(Utf8Path::new("docs/adr/0007.md"), false),
            Kind::Decision
        );
        assert_eq!(kind_of(Utf8Path::new("x/SPEC-a.md"), false), Kind::Spec);
        assert_eq!(
            kind_of(Utf8Path::new("docs/guides/a.md"), false),
            Kind::Guide
        );
        assert_eq!(kind_of(Utf8Path::new("README.md"), true), Kind::Metadata);
        assert_eq!(
            kind_of(Utf8Path::new("docs/intro.md"), false),
            Kind::Document
        );
    }

    /// VERIFIES setup:mark-appends-and-never-rewrites
    #[test]
    fn mark_appends_history_and_never_rewrites_an_earlier_entry() {
        let mut registry = registry();
        let filing = Marking {
            group: Some("storage".to_string()),
            tags: vec!["storage".to_string()],
            ..Marking::default()
        };
        registry
            .mark("docs-adr-0003-md", Status::Mapped, &filing, "t1")
            .unwrap();
        registry
            .mark("docs/adr/0007.md", Status::Mapped, &filing, "t2")
            .unwrap();
        let first = registry.sources[0].history.clone();
        let destinations = Marking {
            destinations: vec![Utf8PathBuf::from("_docs/decisions/ADR-store-state.md")],
            ..Marking::default()
        };
        let marked = registry
            .mark("storage", Status::Verified, &destinations, "t3")
            .unwrap();
        assert_eq!(marked, ["docs-adr-0003-md", "docs-adr-0007-md"]);
        assert_eq!(registry.sources[0].history[..1], first[..]);
        assert_eq!(registry.sources[0].history.len(), 2);
        assert_eq!(registry.groups[0].sources.len(), 2);
        assert_eq!(registry.groups[0].destinations.len(), 1);
    }

    #[test]
    fn a_drop_needs_its_reason_and_a_refusal_changes_nothing() {
        let mut registry = registry();
        let before = registry.clone();
        assert_eq!(
            registry.mark(
                "docs-adr-0003-md",
                Status::Dropped,
                &Marking::default(),
                "t"
            ),
            Err(RegistryError::NoReason)
        );
        assert!(
            registry
                .mark("nope", Status::Mapped, &Marking::default(), "t")
                .is_err()
        );
        assert_eq!(registry, before);
        let reason = Marking {
            reason: Some("superseded by 0007".to_string()),
            ..Marking::default()
        };
        registry
            .mark("docs-adr-0003-md", Status::Dropped, &reason, "t")
            .unwrap();
        let text = registry.render().unwrap();
        assert_eq!(Registry::parse(&text).unwrap(), registry);
    }

    /// VERIFIES setup:close-refuses-an-unfinished-source
    #[test]
    fn close_waits_for_every_source_and_every_destination() {
        let mut registry = registry();
        assert_eq!(registry.unfinished(|_| true).len(), 2);
        let filing = Marking {
            group: Some("storage".to_string()),
            ..Marking::default()
        };
        registry
            .mark("docs-adr-0003-md", Status::Verified, &filing, "t")
            .unwrap();
        registry
            .mark("docs-adr-0007-md", Status::Verified, &filing, "t")
            .unwrap();
        assert!(
            !registry.unfinished(|_| true).is_empty(),
            "no destination yet"
        );
        let destinations = Marking {
            destinations: vec![Utf8PathBuf::from("ADR-x.md")],
            ..Marking::default()
        };
        registry
            .mark("storage", Status::Verified, &destinations, "t")
            .unwrap();
        assert!(
            !registry.unfinished(|_| false).is_empty(),
            "the file is absent"
        );
        assert!(registry.unfinished(|_| true).is_empty());
    }

    /// VERIFIES setup:close-refuses-an-unfinished-source
    #[test]
    fn a_group_with_no_live_source_does_not_block_close() {
        let mut registry = registry();
        let into = |group: &str| Marking {
            group: Some(group.to_string()),
            ..Marking::default()
        };
        let aimed = |path: &str| Marking {
            destinations: vec![Utf8PathBuf::from(path)],
            ..Marking::default()
        };
        // One group loses its source to another group, and one loses its
        // only source to a drop. Neither destination exists.
        registry
            .mark("docs-adr-0003-md", Status::Mapped, &into("first"), "t")
            .unwrap();
        registry
            .mark("first", Status::Mapped, &aimed("gone.md"), "t")
            .unwrap();
        registry
            .mark("docs-adr-0003-md", Status::Verified, &into("kept"), "t")
            .unwrap();
        registry
            .mark("kept", Status::Verified, &aimed("kept.md"), "t")
            .unwrap();
        registry
            .mark("docs-adr-0007-md", Status::Mapped, &into("dropped"), "t")
            .unwrap();
        registry
            .mark("dropped", Status::Mapped, &aimed("never.md"), "t")
            .unwrap();
        let reason = Marking {
            reason: Some("said elsewhere".to_string()),
            ..Marking::default()
        };
        registry
            .mark("docs-adr-0007-md", Status::Dropped, &reason, "t")
            .unwrap();
        assert!(
            registry
                .unfinished(|path| path == Utf8Path::new("kept.md"))
                .is_empty()
        );
    }
}
