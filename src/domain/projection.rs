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
/// A destination may carry a `{docs_root}` placeholder, resolved per
/// profile by [`crate::domain::profile::resolve_destination`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    /// The payload path, as the bundle names it.
    pub source: String,
    /// The destination, relative to the instance root.
    pub destination: String,
    /// The named set of files whose presence at the target root makes an
    /// adopted seed yield: the landing does not introduce it there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yields_to: Option<String>,
}

impl Projection {
    /// The set this projection yields to, where it names one this engine
    /// knows.
    #[must_use]
    pub fn yield_set(&self) -> Option<YieldSet> {
        self.yields_to.as_deref().and_then(YieldSet::parse)
    }
}

/// A named set of files that makes an adopted seed yield.
///
/// A set is named in the declaration and resolved here, so the declaration
/// stays readable and the file names stay in the module that owns them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YieldSet {
    /// Every markdownlint configuration name of both discovery families.
    MarkdownlintConfiguration,
}

impl YieldSet {
    /// The set a declaration names, where this engine knows it.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "markdownlint-configuration" => Some(Self::MarkdownlintConfiguration),
            _ => None,
        }
    }

    /// The file names, relative to the target root, the set holds.
    pub fn names(self) -> impl Iterator<Item = &'static str> {
        match self {
            Self::MarkdownlintConfiguration => crate::domain::markdownlint::discovery_names(),
        }
    }
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
        for entry in self.managed.iter().chain(&self.adopted) {
            if let Some(name) = &entry.yields_to
                && YieldSet::parse(name).is_none()
            {
                return Err(DeclarationError::Inconsistent(format!(
                    "{} yields to {name}, a set this engine does not know",
                    entry.destination
                )));
            }
        }
        if let Some(entry) = self.managed.iter().find(|entry| entry.yields_to.is_some()) {
            return Err(DeclarationError::Inconsistent(format!(
                "the managed destination {} yields, and only an adopted seed may",
                entry.destination
            )));
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
    fn a_templated_managed_destination_parses() {
        let text =
            format!("{MINIMAL}\n[[managed]]\nsource = \"a\"\ndestination = \"{{docs_root}}/a\"\n");
        let held = Declaration::parse(text.as_bytes()).unwrap();
        assert_eq!(held.managed[0].destination, "{docs_root}/a");
    }

    #[test]
    fn a_known_yield_set_resolves_to_its_names() {
        let text = format!(
            "{MINIMAL}\n[[adopted]]\nsource = \"a\"\ndestination = \"a\"\nyields_to = \"markdownlint-configuration\"\n"
        );
        let held = Declaration::parse(text.as_bytes()).unwrap();
        let set = held.adopted[0].yield_set().unwrap();
        assert!(set.names().any(|name| name == ".markdownlint-cli2.jsonc"));
        assert!(set.names().any(|name| name == ".markdownlint.yaml"));
    }

    #[test]
    fn an_unknown_yield_set_is_inconsistent() {
        let text = format!(
            "{MINIMAL}\n[[adopted]]\nsource = \"a\"\ndestination = \"a\"\nyields_to = \"everything\"\n"
        );
        let error = Declaration::parse(text.as_bytes()).unwrap_err();
        assert!(
            matches!(error, DeclarationError::Inconsistent(_)),
            "{error}"
        );
        assert!(error.to_string().contains("everything"), "{error}");
    }

    #[test]
    fn a_yielding_managed_file_is_inconsistent() {
        let text = format!(
            "{MINIMAL}\n[[managed]]\nsource = \"a\"\ndestination = \"a\"\nyields_to = \"markdownlint-configuration\"\n"
        );
        let error = Declaration::parse(text.as_bytes()).unwrap_err();
        assert!(
            error.to_string().contains("only an adopted seed"),
            "{error}"
        );
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
