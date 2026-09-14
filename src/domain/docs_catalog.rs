//! The one description of what this binary's corpus holds.
//!
//! Every document the binary carries is already one command away. What is
//! missing is a route: a list of bare names tells a reader nothing about
//! which name answers their question. This catalog carries the summary and
//! the aliases an author owes that reader, and one resolver turns a
//! question into exactly one topic.
//!
//! Size is never authored. It is read from the embedded bytes when asked,
//! so a number here can never disagree with the document it describes.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Where the catalog sits inside a bundle.
pub const CATALOG_PATH: &str = "instance/docs-catalog.toml";

/// The schema this engine reads.
pub const SCHEMA: u32 = 1;

/// The machine schema `sdd docs --json` declares.
pub const JSON_SCHEMA: &str = "sdd.docs/1";

/// A catalog this engine cannot read.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CatalogError {
    /// The bytes are not the catalog's shape.
    #[error("{CATALOG_PATH} does not parse: {0}")]
    Malformed(String),

    /// The catalog is written in a schema this engine does not read.
    #[error("{CATALOG_PATH} declares schema {0}, and this engine reads {SCHEMA}")]
    UnknownSchema(u32),
}

/// Which shelf serves a document topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShelfId {
    /// The method chapters.
    Method,
    /// The specification seeds and the canon-only specs.
    Spec,
    /// The document templates.
    Template,
}

impl ShelfId {
    /// The word the catalog spells.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Method => "method",
            Self::Spec => "spec",
            Self::Template => "template",
        }
    }
}

/// What reading a topic gets the reader.
///
/// Two kinds and no third. A document target names something a shelf
/// already serves. A command target names an argv of this binary, so a
/// topic whose best answer is a help page routes there rather than
/// acquiring a document nobody needs to write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Target {
    /// One document on one shelf.
    Document {
        /// The shelf that serves it.
        shelf: ShelfId,
        /// The short name the shelf addresses it by.
        name: String,
    },
    /// One argv of this binary, printed rather than run.
    Command(Vec<String>),
}

/// What a topic is, for grouping the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A chapter of the method.
    MethodChapter,
    /// A specification, adopted or canon-only.
    SpecSeed,
    /// A stable authoring template.
    Template,
    /// Something an operator does, answered by a help page.
    OperatorTask,
}

impl Kind {
    /// The word the catalog spells.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MethodChapter => "method-chapter",
            Self::SpecSeed => "spec-seed",
            Self::Template => "template",
            Self::OperatorTask => "operator-task",
        }
    }

    /// The index heading this kind sits under.
    #[must_use]
    pub const fn heading(self) -> &'static str {
        match self {
            Self::MethodChapter => "Method chapters",
            Self::SpecSeed => "Specifications",
            Self::Template => "Templates",
            Self::OperatorTask => "Operator tasks",
        }
    }

    /// Every kind, in the order the index prints them.
    #[must_use]
    pub const fn every() -> [Self; 4] {
        [
            Self::OperatorTask,
            Self::MethodChapter,
            Self::SpecSeed,
            Self::Template,
        ]
    }
}

/// One thing a reader can ask for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Topic {
    /// The stable identifier, unique case-folded across the catalog.
    pub id: String,
    /// The title a reader sees.
    pub title: String,
    /// One line saying what reading this answers.
    pub summary: String,
    /// What this topic is.
    pub kind: Kind,
    /// Other phrasings that resolve here.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// What reading it gets.
    pub target: Target,
}

/// The whole catalog, parsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    /// Always [`SCHEMA`] once parsed.
    pub catalog_schema: u32,
    /// Every topic, in authored order.
    #[serde(default, rename = "topic")]
    pub topics: Vec<Topic>,
}

/// What this binary's own release declares its corpus to hold.
///
/// The bytes are embedded, so a catalog that does not parse is a defect in
/// the build rather than a state a command can meet. The canon suite parses
/// the same file, so the failure lands in the test run.
#[expect(
    clippy::expect_used,
    reason = "the catalog is compiled in; a parse failure is a build defect the canon suite catches first"
)]
pub static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
    let bytes = crate::embedded::asset(CATALOG_PATH)
        .expect("the payload carries instance/docs-catalog.toml");
    Catalog::parse(bytes).expect("the embedded documentation catalog parses")
});

/// Why a query resolved to no single topic.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Unresolved {
    /// More than one topic matched, and the engine guesses none.
    #[error("'{query}' matches {}; name one", .candidates.join(", "))]
    Ambiguous {
        /// What was asked for.
        query: String,
        /// Every topic that matched.
        candidates: Vec<String>,
    },

    /// Nothing matched, with the nearest identifiers.
    #[error("no topic matches '{query}'; nearest: {}", .nearest.join(", "))]
    Missing {
        /// What was asked for.
        query: String,
        /// The closest identifiers, or every identifier when none is close.
        nearest: Vec<String>,
    },
}

/// Normalize a query or a key the same way, so both sides compare alike.
fn fold(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl Catalog {
    /// Read one catalog.
    ///
    /// # Errors
    ///
    /// [`CatalogError`] when the bytes do not parse or the schema is one
    /// this engine does not read.
    pub fn parse(bytes: &[u8]) -> Result<Self, CatalogError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|source| CatalogError::Malformed(source.to_string()))?;
        let held: Self =
            toml::from_str(text).map_err(|source| CatalogError::Malformed(source.to_string()))?;
        if held.catalog_schema != SCHEMA {
            return Err(CatalogError::UnknownSchema(held.catalog_schema));
        }
        Ok(held)
    }

    /// The topic serving one shelf document, where the catalog has one.
    #[must_use]
    pub fn for_document(&self, on: ShelfId, wanted: &str) -> Option<&Topic> {
        self.topics.iter().find(|topic| {
            matches!(&topic.target, Target::Document { shelf: held, name: held_name }
                if *held == on && held_name == wanted)
        })
    }

    /// Turn one query into exactly one topic.
    ///
    /// The order is exact id, then exact alias, then a case-folded prefix
    /// over both. An ambiguous query names every candidate rather than
    /// picking one, because a reader who asked loosely gets a shorter list
    /// and never a document they did not mean.
    ///
    /// # Errors
    ///
    /// [`Unresolved`] naming either the candidates or the nearest ids.
    pub fn resolve(&self, query: &str) -> Result<&Topic, Unresolved> {
        let wanted = fold(query);
        if let Some(topic) = self.topics.iter().find(|topic| fold(&topic.id) == wanted) {
            return Ok(topic);
        }

        let by_alias: Vec<&Topic> = self
            .topics
            .iter()
            .filter(|topic| topic.aliases.iter().any(|alias| fold(alias) == wanted))
            .collect();
        if let [only] = by_alias.as_slice() {
            return Ok(only);
        }
        if !by_alias.is_empty() {
            return Err(Unresolved::Ambiguous {
                query: wanted,
                candidates: by_alias.iter().map(|topic| topic.id.clone()).collect(),
            });
        }

        let by_prefix: Vec<&Topic> = self
            .topics
            .iter()
            .filter(|topic| {
                fold(&topic.id).starts_with(&wanted)
                    || topic
                        .aliases
                        .iter()
                        .any(|alias| fold(alias).starts_with(&wanted))
            })
            .collect();
        match by_prefix.as_slice() {
            [only] => Ok(only),
            [] => Err(Unresolved::Missing {
                query: wanted.clone(),
                nearest: self.nearest(&wanted),
            }),
            many => Err(Unresolved::Ambiguous {
                query: wanted,
                candidates: many.iter().map(|topic| topic.id.clone()).collect(),
            }),
        }
    }

    /// The identifiers closest to a query that matched nothing.
    ///
    /// Substring containment in either direction, capped, and every id
    /// where nothing is close, because a reader who missed needs a list
    /// rather than an apology.
    fn nearest(&self, wanted: &str) -> Vec<String> {
        let mut near: Vec<String> = self
            .topics
            .iter()
            .filter(|topic| {
                let id = fold(&topic.id);
                id.contains(wanted) || wanted.contains(&id)
            })
            .map(|topic| topic.id.clone())
            .collect();
        if near.is_empty() {
            near = self.topics.iter().map(|topic| topic.id.clone()).collect();
        }
        near.truncate(8);
        near
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    const SAMPLE: &str = r#"
catalog_schema = 1

[[topic]]
id = "agent-context"
title = "Agent context"
summary = "What an agent loads and the budget an always-loaded file pays."
kind = "method-chapter"
aliases = ["context budget", "always loaded"]
target = { document = { shelf = "method", name = "agent-context" } }

[[topic]]
id = "agents-digest"
title = "The digest template"
summary = "A stable starting point for an author-instructions file."
kind = "template"
aliases = ["digest template"]
target = { document = { shelf = "template", name = "agents-digest" } }

[[topic]]
id = "upgrade"
title = "Upgrading an instance"
summary = "Take a landed instance to a chosen release through one plan."
kind = "operator-task"
aliases = ["new version"]
target = { command = ["reconcile", "plan", "--help"] }
"#;

    fn sample() -> Catalog {
        Catalog::parse(SAMPLE.as_bytes()).unwrap()
    }

    #[test]
    fn a_topic_resolves_by_id_by_alias_and_by_unique_prefix() {
        let catalog = sample();
        assert_eq!(
            catalog.resolve("agent-context").unwrap().id,
            "agent-context"
        );
        assert_eq!(
            catalog.resolve("Context Budget").unwrap().id,
            "agent-context"
        );
        assert_eq!(catalog.resolve("upg").unwrap().id, "upgrade");
        assert_eq!(
            catalog.resolve("always  loaded").unwrap().id,
            "agent-context"
        );
    }

    #[test]
    fn an_ambiguous_query_names_every_candidate_and_guesses_none() {
        let catalog = sample();
        let refused = catalog.resolve("agent").unwrap_err();
        let Unresolved::Ambiguous { candidates, .. } = refused else {
            panic!("an ambiguous prefix resolved to one topic");
        };
        assert_eq!(candidates, vec!["agent-context", "agents-digest"]);
    }

    #[test]
    fn a_missing_topic_names_the_nearest_ids() {
        let catalog = sample();
        let refused = catalog.resolve("release process").unwrap_err();
        let Unresolved::Missing { nearest, .. } = refused else {
            panic!("a missing topic did not report as missing");
        };
        assert!(!nearest.is_empty());
    }

    #[test]
    fn an_unknown_schema_or_shape_refuses() {
        assert!(matches!(
            Catalog::parse(b"catalog_schema = 9\n").unwrap_err(),
            CatalogError::UnknownSchema(9)
        ));
        assert!(matches!(
            Catalog::parse(b"catalog_schema = 1\nextra = 1\n").unwrap_err(),
            CatalogError::Malformed(_)
        ));
    }

    #[test]
    fn a_document_topic_is_found_by_its_shelf_and_name() {
        let catalog = sample();
        assert_eq!(
            catalog
                .for_document(ShelfId::Method, "agent-context")
                .unwrap()
                .id,
            "agent-context"
        );
        assert!(
            catalog
                .for_document(ShelfId::Spec, "agent-context")
                .is_none()
        );
    }
}
