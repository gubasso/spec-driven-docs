//! `docs` subcommand: runtime-shape.
//!
//! The bare verb is the index. One argument, or several, is a topic. The
//! index is what an always-loaded routing line can name in one clause, so
//! an agent in a repository that adopted this tool reaches any chapter in
//! two commands.
//!
//! A command target is printed rather than run. A verb that dispatched to
//! another verb would hide which one answered.

use serde::Serialize;

use crate::cli::docs::DocsArgs;
use crate::context::AppContext;
use crate::domain::docs_catalog::{CATALOG, JSON_SCHEMA, Kind, ShelfId, Target, Topic};
use crate::error::AppError;
use crate::output;
use crate::services::reader;

/// One topic, as the machine reads it.
#[derive(Debug, Serialize)]
struct Entry<'a> {
    id: &'a str,
    title: &'a str,
    summary: &'a str,
    kind: &'static str,
    aliases: &'a [String],
    target: &'a Target,
    /// The embedded document's size, or null for a command target.
    bytes: Option<usize>,
    /// Passed through from the document's own frontmatter, never invented.
    #[serde(skip_serializing_if = "Option::is_none")]
    token_estimate: Option<u32>,
}

/// The whole index, as the machine reads it.
#[derive(Debug, Serialize)]
struct Report<'a> {
    schema: &'static str,
    topics: Vec<Entry<'a>>,
}

/// Read the corpus this binary carries.
///
/// # Errors
///
/// [`AppError::Usage`] when a query matches no topic or more than one.
pub fn run(_ctx: &AppContext, args: &DocsArgs) -> Result<(), AppError> {
    if args.topic.is_empty() {
        if args.json {
            return index_json();
        }
        index_text();
        return Ok(());
    }
    let query = args.topic.join(" ");
    let topic = CATALOG
        .resolve(&query)
        .map_err(|refusal| AppError::Usage(refusal.to_string()))?;
    if args.json {
        return output::json(&entry(topic));
    }
    render(topic)
}

/// The shelf a document target names.
const fn shelf_of(shelf: ShelfId) -> &'static reader::Shelf {
    match shelf {
        ShelfId::Method => &reader::METHOD,
        ShelfId::Spec => &reader::SPECS,
        ShelfId::Template => &reader::TEMPLATES,
    }
}

/// The embedded text one topic points at, where it points at one.
fn document(topic: &Topic) -> Option<&'static str> {
    match &topic.target {
        Target::Document { shelf, name } => reader::get(shelf_of(*shelf), name),
        Target::Command(_) => None,
    }
}

/// The token estimate a document's own frontmatter declares.
///
/// Read, never computed. This repository runs no tokenizer, and a number
/// it did not measure is a number it cannot stand behind.
fn declared_token_estimate(text: &str) -> Option<u32> {
    let body = text.strip_prefix("---\n")?;
    let front = body.split("\n---").next()?;
    front
        .lines()
        .find_map(|line| line.strip_prefix("token-estimate:"))
        .and_then(|value| value.trim().parse().ok())
}

fn entry(topic: &Topic) -> Entry<'_> {
    let text = document(topic);
    Entry {
        id: &topic.id,
        title: &topic.title,
        summary: &topic.summary,
        kind: topic.kind.as_str(),
        aliases: &topic.aliases,
        target: &topic.target,
        bytes: text.map(str::len),
        token_estimate: text.and_then(declared_token_estimate),
    }
}

fn index_json() -> Result<(), AppError> {
    output::json(&Report {
        schema: JSON_SCHEMA,
        topics: CATALOG.topics.iter().map(entry).collect(),
    })
}

fn index_text() {
    output::line("Read one topic with `sdd docs <topic>`. A topic takes several words.");
    let width = CATALOG
        .topics
        .iter()
        .map(|topic| topic.id.len())
        .max()
        .unwrap_or(0);
    for kind in Kind::every() {
        let held: Vec<&Topic> = CATALOG
            .topics
            .iter()
            .filter(|topic| topic.kind == kind)
            .collect();
        if held.is_empty() {
            continue;
        }
        output::line("");
        output::line(kind.heading());
        for topic in held {
            output::line(format!(
                "  {:width$}  {}",
                topic.id,
                topic.summary,
                width = width
            ));
        }
    }
}

fn render(topic: &Topic) -> Result<(), AppError> {
    match &topic.target {
        Target::Document { shelf, name } => {
            let Some(text) = reader::get(shelf_of(*shelf), name) else {
                return Err(AppError::Usage(format!(
                    "{} names {}/{name}, which no shelf serves",
                    topic.id,
                    shelf.as_str()
                )));
            };
            output::line(text.trim_end_matches('\n'));
            Ok(())
        }
        Target::Command(argv) => {
            output::line(format!("{}: {}", topic.title, topic.summary));
            output::line(format!("Run: sdd {}", argv.join(" ")));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    #[test]
    fn a_declared_token_estimate_is_passed_through_and_never_invented() {
        let declared = "---\ndigest-of: method/\ntoken-estimate: 580\n---\n\n# AGENTS\n";
        assert_eq!(declared_token_estimate(declared), Some(580));
        assert_eq!(declared_token_estimate("# Chapter\n\nProse.\n"), None);
    }

    #[test]
    fn every_catalog_target_resolves() {
        let names = crate::cli::subcommand_names();
        for topic in &CATALOG.topics {
            match &topic.target {
                Target::Document { shelf, name } => assert!(
                    reader::get(shelf_of(*shelf), name).is_some(),
                    "{} names {}/{name}, which no shelf serves",
                    topic.id,
                    shelf.as_str()
                ),
                Target::Command(argv) => {
                    let verb = argv.first().unwrap();
                    assert!(
                        names.contains(verb),
                        "{} names the verb '{verb}', which the parser does not offer",
                        topic.id
                    );
                }
            }
        }
    }

    #[test]
    fn every_catalog_id_and_alias_is_unique_case_folded() {
        let mut seen = std::collections::BTreeSet::new();
        for topic in &CATALOG.topics {
            for key in std::iter::once(&topic.id).chain(topic.aliases.iter()) {
                assert!(
                    seen.insert(key.to_lowercase()),
                    "'{key}' is declared twice in the catalog"
                );
            }
        }
    }

    #[test]
    fn the_catalog_summaries_are_one_line_and_within_their_cap() {
        for topic in &CATALOG.topics {
            assert!(
                !topic.summary.contains('\n'),
                "{} carries a multi-line summary",
                topic.id
            );
            assert!(
                topic.summary.len() <= 100,
                "{} carries a {}-character summary, over the 100 cap",
                topic.id,
                topic.summary.len()
            );
        }
    }

    #[test]
    fn size_is_computed_and_never_authored() {
        let authored = crate::embedded::asset(crate::domain::docs_catalog::CATALOG_PATH).unwrap();
        let text = std::str::from_utf8(authored).unwrap();
        for forbidden in ["bytes =", "token_estimate =", "token-estimate ="] {
            assert!(
                !text.contains(forbidden),
                "the catalog authors '{forbidden}', which it reads from the payload instead"
            );
        }
    }
}
