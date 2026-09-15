//! What this release lands, read as data rather than compiled in.
//!
//! The release declares its own projection in `instance/projection.toml`,
//! which the binary carries. Selection stays in data because the
//! declaration is simpler to read, to diff, and to review than the Rust
//! constants it replaced.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::profile::{DocsRoot, ProfileId};

/// Where the declaration sits inside the payload.
pub const DECLARATION_PATH: &str = "instance/projection.toml";

/// A declaration this engine cannot read.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DeclarationError {
    /// The bytes are not the declaration's shape.
    #[error("{DECLARATION_PATH} does not parse: {0}")]
    Malformed(String),

    /// The declaration is internally inconsistent.
    #[error("{DECLARATION_PATH} is inconsistent: {0}")]
    Inconsistent(String),
}

/// One payload projection: an embedded source and its instance destination.
///
/// An adopted destination may carry a `{docs_root}` placeholder, resolved
/// per profile by [`crate::domain::profile::resolve_destination`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    /// The payload path, as the bundle names it.
    pub source: String,
    /// The destination, relative to the instance root.
    pub destination: String,
}

/// One profile the release offers.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDeclaration {
    /// The profile's kebab-case name.
    pub id: ProfileId,
    /// Where that profile keeps the documents the gates read.
    pub docs_root: DocsRoot,
}

/// One sentinel: the rule, and the adopted specification that owns it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SentinelDeclaration {
    /// The rule an instance's specifications must define.
    pub rule: String,
    /// The payload specification that carries it.
    pub source: String,
    /// Where an instance holds that specification, templated.
    pub destination: String,
    /// The declaration it authorizes, for the note that names it.
    pub declares: String,
}

/// Everything one release declares about what it lands.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    /// The template copies the canon keeps in its own documentation tree.
    #[serde(default)]
    pub canon_templates: Vec<String>,
    /// Every profile the release offers.
    pub profiles: Vec<ProfileDeclaration>,
    /// Byte projections the canon keeps owning.
    #[serde(default)]
    pub managed: Vec<Projection>,
    /// Seeds the instance owns from the moment they land.
    #[serde(default)]
    pub adopted: Vec<Projection>,
    /// One sentinel per feature a project can declare.
    #[serde(default)]
    pub sentinels: Vec<SentinelDeclaration>,
}

impl Declaration {
    /// Read a declaration, refusing a schema this engine does not decode.
    ///
    /// # Errors
    ///
    /// [`DeclarationError`] when the bytes do not parse or when the
    /// declaration contradicts itself.
    pub fn parse(bytes: &[u8]) -> Result<Self, DeclarationError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|source| DeclarationError::Malformed(source.to_string()))?;
        let held: Self = toml::from_str(text)
            .map_err(|source| DeclarationError::Malformed(source.to_string()))?;
        held.consistent()?;
        Ok(held)
    }

    /// Whether the declaration says one thing.
    fn consistent(&self) -> Result<(), DeclarationError> {
        if self.profiles.is_empty() {
            return Err(DeclarationError::Inconsistent(
                "no profile is declared".to_string(),
            ));
        }
        for entry in &self.managed {
            if entry.destination.contains('{') {
                return Err(DeclarationError::Inconsistent(format!(
                    "the managed destination {} is templated, and only an adopted destination may be",
                    entry.destination
                )));
            }
        }
        let mut seen: Vec<&str> = Vec::new();
        for entry in self.managed.iter().chain(&self.adopted) {
            if seen.contains(&entry.destination.as_str()) {
                return Err(DeclarationError::Inconsistent(format!(
                    "{} is projected twice",
                    entry.destination
                )));
            }
            seen.push(&entry.destination);
        }
        Ok(())
    }

    /// What one profile lands, as this release declares it.
    #[must_use]
    pub fn profile(&self, id: ProfileId) -> Option<crate::domain::profile::Profile<'_>> {
        Some(crate::domain::profile::Profile {
            id,
            docs_root: self.docs_root(id)?,
            managed: &self.managed,
            adopted: &self.adopted,
        })
    }

    /// The documentation root one profile takes, where it is declared.
    #[must_use]
    pub fn docs_root(&self, id: ProfileId) -> Option<DocsRoot> {
        self.profiles
            .iter()
            .find(|profile| profile.id == id)
            .map(|profile| profile.docs_root)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    const MINIMAL: &str = r#"
[[profiles]]
id = "codebase"
docs_root = "docs"
"#;

    #[test]
    fn a_minimal_declaration_parses() {
        let held = Declaration::parse(MINIMAL.as_bytes()).unwrap();
        assert_eq!(held.docs_root(ProfileId::Codebase), Some(DocsRoot::Docs));
        assert_eq!(held.docs_root(ProfileId::KnowledgeBase), None);
    }

    #[test]
    fn a_declaration_with_no_profile_refuses() {
        // Absent is a parse failure and empty is an inconsistency. Both
        // refuse, because a release that offers no profile lands nothing.
        assert!(matches!(
            Declaration::parse(b"").unwrap_err(),
            DeclarationError::Malformed(_)
        ));
        assert!(matches!(
            Declaration::parse(b"profiles = []\n").unwrap_err(),
            DeclarationError::Inconsistent(_)
        ));
    }

    #[test]
    fn a_templated_managed_destination_is_inconsistent() {
        let text =
            format!("{MINIMAL}\n[[managed]]\nsource = \"a\"\ndestination = \"{{docs_root}}/a\"\n");
        let error = Declaration::parse(text.as_bytes()).unwrap_err();
        assert!(error.to_string().contains("is templated"), "{error}");
    }

    #[test]
    fn one_destination_projected_twice_is_inconsistent() {
        let text = format!(
            "{MINIMAL}\n[[managed]]\nsource = \"a\"\ndestination = \"x\"\n[[adopted]]\nsource = \"b\"\ndestination = \"x\"\n"
        );
        let error = Declaration::parse(text.as_bytes()).unwrap_err();
        assert!(error.to_string().contains("projected twice"), "{error}");
    }

    #[test]
    fn an_unknown_field_refuses_rather_than_being_ignored() {
        let text = format!("{MINIMAL}\nsomething_new = 1\n");
        assert!(matches!(
            Declaration::parse(text.as_bytes()).unwrap_err(),
            DeclarationError::Malformed(_)
        ));
    }
}
