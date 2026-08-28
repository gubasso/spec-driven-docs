//! Installation profiles: what a target repository receives.
//!
//! A profile declares the documentation root and the payload projection —
//! which embedded files land managed and which land adopted, and where.
//! The declarations are code so a profile referencing an asset the payload
//! does not carry fails a test instead of an install. Copying bytes and
//! recording hashes is the installer's work.

use std::fmt;

use camino::Utf8PathBuf;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

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

impl ProfileId {
    /// The profile's declaration.
    #[must_use]
    pub const fn profile(self) -> &'static Profile {
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

/// One payload projection: an embedded source and its instance destination.
///
/// An adopted destination may carry a `{docs_root}` placeholder, resolved
/// per profile by [`resolve_destination`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Projection {
    /// The embedded payload path.
    pub source: &'static str,
    /// The destination, relative to the instance root.
    pub destination: &'static str,
}

const fn proj(source: &'static str, destination: &'static str) -> Projection {
    Projection {
        source,
        destination,
    }
}

/// Substitute the profile's documentation root into a destination template.
#[must_use]
// The braces are the template placeholder itself, not a formatting argument.
#[allow(clippy::literal_string_with_formatting_args)]
pub fn resolve_destination(destination: &str, docs_root: DocsRoot) -> Utf8PathBuf {
    Utf8PathBuf::from(destination.replace("{docs_root}", docs_root.as_str()))
}

/// What one profile installs.
#[derive(Debug)]
pub struct Profile {
    /// The profile this declaration belongs to.
    pub id: ProfileId,
    /// The documentation root the instance uses.
    pub docs_root: DocsRoot,
    /// Byte projections the canon keeps owning.
    pub managed: &'static [Projection],
    /// Seeds the instance owns from the moment they land.
    pub adopted: &'static [Projection],
}

/// Byte projections every profile installs.
///
/// No skill appears here. A skill name is what an agent's picker keys on,
/// so an instance copy and the user-scope copy of one skill are two entries
/// under one name in every session opened inside that instance. User scope
/// owns them alone (ADR-give-every-skill-one-owner).
const MANAGED: &[Projection] = &[
    proj(
        ".markdownlint/adr.markdownlint-cli2.jsonc",
        ".spec-driven-docs/markdownlint/adr.markdownlint-cli2.jsonc",
    ),
    proj(
        ".markdownlint/spec.markdownlint-cli2.jsonc",
        ".spec-driven-docs/markdownlint/spec.markdownlint-cli2.jsonc",
    ),
    proj(
        ".markdownlint/relative-links.markdownlint-cli2.jsonc",
        ".spec-driven-docs/markdownlint/relative-links.markdownlint-cli2.jsonc",
    ),
];

const ADOPTED: &[Projection] = &[
    proj(
        "_docs/specs/SPEC-decision-records.md",
        "{docs_root}/specs/SPEC-decision-records.md",
    ),
    proj(
        "_docs/specs/SPEC-instance.md",
        "{docs_root}/specs/SPEC-instance.md",
    ),
    proj(
        "_docs/specs/SPEC-docs-format.md",
        "{docs_root}/specs/SPEC-docs-format.md",
    ),
    proj(
        "_docs/specs/SPEC-docs-foundations.md",
        "{docs_root}/specs/SPEC-docs-foundations.md",
    ),
    proj(
        "_docs/specs/SPEC-docs-specs.md",
        "{docs_root}/specs/SPEC-docs-specs.md",
    ),
    proj(
        "_docs/specs/SPEC-comparison-docs.md",
        "{docs_root}/specs/SPEC-comparison-docs.md",
    ),
    proj(
        "_docs/specs/SPEC-known-issues.md",
        "{docs_root}/specs/SPEC-known-issues.md",
    ),
    proj(
        "_docs/specs/SPEC-spec-to-code.md",
        "{docs_root}/specs/SPEC-spec-to-code.md",
    ),
    proj(
        "templates/TEMPLATE-spec.md",
        "{docs_root}/specs/TEMPLATE-spec.md",
    ),
    proj(
        "templates/TEMPLATE-adr.md",
        "{docs_root}/decisions/TEMPLATE-adr.md",
    ),
    proj(
        "templates/TEMPLATE-agents-digest.md",
        "{docs_root}/reference/TEMPLATE-agents-digest.md",
    ),
];

static CODEBASE: Profile = Profile {
    id: ProfileId::Codebase,
    docs_root: DocsRoot::Docs,
    managed: MANAGED,
    adopted: ADOPTED,
};

static KNOWLEDGE_BASE: Profile = Profile {
    id: ProfileId::KnowledgeBase,
    docs_root: DocsRoot::UnderscoreDocs,
    managed: MANAGED,
    adopted: ADOPTED,
};

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
        for entry in MANAGED {
            assert!(
                !entry.destination.contains('{'),
                "{} is templated",
                entry.destination
            );
        }
        for entry in ADOPTED {
            assert!(
                entry.destination.starts_with("{docs_root}/"),
                "{} is not rooted",
                entry.destination
            );
        }
    }
}
