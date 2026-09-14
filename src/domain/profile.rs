//! Installation profiles: what a target repository receives.
//!
//! A profile declares the documentation root and the payload projection —
//! which payload files land managed and which land adopted, and where.
//! The declaration is data the release carries, so an engine can read what
//! any release it can fetch lands rather than only what it was compiled
//! with. Copying bytes and recording hashes is the installer's work.

use std::fmt;
use std::sync::LazyLock;

use camino::Utf8PathBuf;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub use crate::domain::projection::Projection;
use crate::domain::projection::{DECLARATION_PATH, Declaration};

/// The two installable profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
#[value(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum ProfileId {
    /// A codebase whose records live under `docs/`.
    Codebase,
    /// A knowledge base whose records live under `_docs/`.
    KnowledgeBase,
}

/// Every profile, in declaration order.
pub const EVERY_PROFILE: [ProfileId; 2] = [ProfileId::Codebase, ProfileId::KnowledgeBase];

impl ProfileId {
    /// Every profile, in declaration order.
    pub fn every() -> impl Iterator<Item = Self> {
        EVERY_PROFILE.into_iter()
    }

    /// This binary's own release, as this profile.
    ///
    /// A verb that lands another release reads that release's declaration
    /// instead, through [`crate::domain::projection::Declaration::profile`].
    #[must_use]
    pub fn profile(self) -> &'static Profile<'static> {
        match self {
            Self::Codebase => &CODEBASE,
            Self::KnowledgeBase => &KNOWLEDGE_BASE,
        }
    }

    /// The kebab-case name used on the command line and in the manifest.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codebase => "codebase",
            Self::KnowledgeBase => "knowledge-base",
        }
    }
}

impl fmt::Display for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where an instance keeps the documents the gates read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocsRoot {
    /// `docs/` — the codebase convention.
    #[serde(rename = "docs")]
    Docs,
    /// `_docs/` — the knowledge-base convention.
    #[serde(rename = "_docs")]
    UnderscoreDocs,
}

impl DocsRoot {
    /// The directory name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Docs => "docs",
            Self::UnderscoreDocs => "_docs",
        }
    }
}

impl fmt::Display for DocsRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Substitute the profile's documentation root into a destination template.
#[must_use]
#[allow(
    clippy::literal_string_with_formatting_args,
    reason = "the braces are the destination template's placeholder, not a formatting argument"
)]
pub fn resolve_destination(destination: &str, docs_root: DocsRoot) -> Utf8PathBuf {
    Utf8PathBuf::from(destination.replace("{docs_root}", docs_root.as_str()))
}

/// What one profile installs, as one declaration describes it.
#[derive(Debug, Clone, Copy)]
pub struct Profile<'a> {
    /// The profile this view belongs to.
    pub id: ProfileId,
    /// The documentation root the instance uses.
    pub docs_root: DocsRoot,
    /// Byte projections the canon keeps owning.
    pub managed: &'a [Projection],
    /// Seeds the instance owns from the moment they land.
    pub adopted: &'a [Projection],
}

/// What this binary's own release declares.
///
/// The bytes are embedded, so a declaration that does not parse is a defect
/// in the build rather than a state a command can meet. The canon suite
/// parses the same file, so the failure lands in the test run.
#[expect(
    clippy::expect_used,
    reason = "the declaration is compiled in; a parse failure is a build defect the canon suite catches first"
)]
pub static DECLARATION: LazyLock<Declaration> = LazyLock::new(|| {
    let bytes = crate::embedded::asset(DECLARATION_PATH)
        .expect("the payload carries instance/projection.toml");
    Declaration::parse(bytes).expect("the embedded projection declaration parses")
});

/// The template copies this repository keeps in its own documentation tree.
pub static CANON_TEMPLATES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    DECLARATION
        .canon_templates
        .iter()
        .map(String::as_str)
        .collect()
});

static CODEBASE: LazyLock<Profile<'static>> = LazyLock::new(|| profile_of(ProfileId::Codebase));
static KNOWLEDGE_BASE: LazyLock<Profile<'static>> =
    LazyLock::new(|| profile_of(ProfileId::KnowledgeBase));

/// One profile of this binary's own release.
#[expect(
    clippy::expect_used,
    reason = "a profile the declaration omits is the same build defect as a declaration that does not parse"
)]
fn profile_of(id: ProfileId) -> Profile<'static> {
    DECLARATION
        .profile(id)
        .expect("the embedded declaration offers every profile")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_bind_their_roots() {
        assert_eq!(ProfileId::Codebase.profile().docs_root, DocsRoot::Docs);
        assert_eq!(
            ProfileId::KnowledgeBase.profile().docs_root,
            DocsRoot::UnderscoreDocs
        );
    }

    #[test]
    fn destinations_resolve_per_root() {
        assert_eq!(
            resolve_destination("{docs_root}/specs/SPEC-distribution.md", DocsRoot::Docs),
            Utf8PathBuf::from("docs/specs/SPEC-distribution.md")
        );
        assert_eq!(
            resolve_destination(
                ".spec-driven-docs/markdownlint/x.jsonc",
                DocsRoot::UnderscoreDocs
            ),
            Utf8PathBuf::from(".spec-driven-docs/markdownlint/x.jsonc")
        );
    }

    #[test]
    fn serde_uses_the_kebab_names() {
        assert_eq!(
            serde_json::to_string(&ProfileId::KnowledgeBase).unwrap(),
            "\"knowledge-base\""
        );
        assert_eq!(
            serde_json::to_string(&DocsRoot::UnderscoreDocs).unwrap(),
            "\"_docs\""
        );
    }

    #[test]
    fn destination_templates_only_use_the_placeholder_in_adopted_paths() {
        let profile = ProfileId::KnowledgeBase.profile();
        for entry in profile.managed {
            assert!(
                !entry.destination.contains('{'),
                "{} is templated",
                entry.destination
            );
        }
        for entry in profile.adopted {
            // The declaration is the one adopted file outside the corpus: it
            // configures the tool rather than being documentation, so no
            // documentation root names it.
            if entry.destination == crate::domain::paths::CONFIG_PATH {
                continue;
            }
            assert!(
                entry.destination.starts_with("{docs_root}/"),
                "{} is not rooted",
                entry.destination
            );
        }
    }

    #[test]
    fn the_canon_templates_come_from_the_declaration() {
        assert_eq!(*CANON_TEMPLATES, DECLARATION.canon_templates);
    }
}
