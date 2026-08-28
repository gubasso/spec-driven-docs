//! Render the gate registry as pre-commit hook entries.
//!
//! One renderer serves both deliveries: the managed block spliced into an
//! instance's configuration, and the manifest published to consumers who
//! install this repository as a pre-commit repo. Rendering both from the
//! registry is what keeps a gate from reaching one delivery and missing the
//! other. What the registry contains is `gates`' business; where the output
//! lands is the caller's.

use std::fmt::Write as _;

use clap::ValueEnum;

use crate::domain::marker;
use crate::gates::GATES;

/// The two output shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Style {
    /// Entries nested under a `hooks:` key inside a consumer's `repos:`
    /// sequence.
    Block,
    /// The top-level sequence a `.pre-commit-hooks.yaml` is.
    Manifest,
}

/// Everything a render depends on.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// The output shape.
    pub style: Style,
    /// What replaces `{docs_root}` in wiring patterns — a literal root for
    /// an instance, a pattern for the published manifest.
    pub docs_root: String,
    /// The command prefix an entry invokes, e.g. `sdd` or `cargo run -q --`.
    pub entry: String,
    /// The pre-commit language the entries declare.
    pub language: String,
    /// The sequence-item indentation of the consumer's `repos:` entries;
    /// only the block style reads it.
    pub indent: String,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            style: Style::Block,
            docs_root: "_docs".to_string(),
            entry: "sdd".to_string(),
            language: "system".to_string(),
            indent: "  ".to_string(),
        }
    }
}

/// A single-quoted YAML scalar; an apostrophe is escaped by doubling it.
fn quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

// The braces are the wiring template's placeholder, not a formatting argument.
#[allow(clippy::literal_string_with_formatting_args)]
fn substitute_root(pattern: &str, docs_root: &str) -> String {
    pattern.replace("{docs_root}", docs_root)
}

/// Render the gate entries alone, in the requested style.
#[must_use]
pub fn render_gates(options: &RenderOptions) -> String {
    let (item, field) = match options.style {
        Style::Block => (
            format!("{0}    - ", options.indent),
            format!("{0}      ", options.indent),
        ),
        Style::Manifest => ("- ".to_string(), "  ".to_string()),
    };
    let mut out = String::new();
    for gate in GATES {
        let _ = writeln!(out, "{item}id: {}", gate.id);
        let _ = writeln!(out, "{field}name: {}", quoted(gate.name));
        let _ = writeln!(out, "{field}entry: {} gate {}", options.entry, gate.id);
        let _ = writeln!(out, "{field}language: {}", options.language);
        if let Some(files) = gate.files {
            let _ = writeln!(
                out,
                "{field}files: {}",
                quoted(&substitute_root(files, &options.docs_root))
            );
        }
        if let Some(types) = gate.types {
            let _ = writeln!(out, "{field}types: [{types}]");
        }
        if let Some(exclude) = gate.exclude {
            let _ = writeln!(
                out,
                "{field}exclude: {}",
                quoted(&substitute_root(exclude, &options.docs_root))
            );
        }
        if gate.always_run {
            let _ = writeln!(out, "{field}always_run: true");
            let _ = writeln!(out, "{field}pass_filenames: false");
        }
    }
    out
}

/// Render the complete managed block an instance's configuration carries:
/// markers, the verifier hook, and every gate.
#[must_use]
pub fn render_block(options: &RenderOptions) -> String {
    let indent = &options.indent;
    let mut out = String::new();
    out.push_str(marker::BEGIN);
    out.push('\n');
    let _ = writeln!(out, "{indent}- repo: local");
    let _ = writeln!(out, "{indent}  hooks:");
    let _ = writeln!(out, "{indent}    - id: spec-driven-docs-verify");
    let _ = writeln!(out, "{indent}      name: verify spec-driven docs instance");
    let _ = writeln!(out, "{indent}      entry: {} verify", options.entry);
    let _ = writeln!(out, "{indent}      language: {}", options.language);
    let _ = writeln!(out, "{indent}      always_run: true");
    let _ = writeln!(out, "{indent}      pass_filenames: false");
    out.push_str(&render_gates(options));
    out.push_str(marker::END);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_style_renders_top_level_entries() {
        let options = RenderOptions {
            style: Style::Manifest,
            docs_root: "_?docs".to_string(),
            language: "rust".to_string(),
            ..RenderOptions::default()
        };
        let out = render_gates(&options);
        assert!(out.starts_with("- id: adr-cites-a-live-rule\n"));
        assert!(out.contains("  entry: sdd gate adr-filename-shape\n"));
        assert!(out.contains("  language: rust\n"));
        assert!(out.contains("  files: '^_?docs/decisions/.*\\.md$'\n"));
        assert!(out.contains("  types: [markdown]\n"));
        assert_eq!(out.matches("- id: ").count(), crate::gates::GATES.len());
    }

    #[test]
    fn block_style_carries_the_markers_and_the_verifier() {
        let out = render_block(&RenderOptions::default());
        assert!(out.starts_with("# BEGIN spec-driven-docs managed\n"));
        assert!(out.ends_with("# END spec-driven-docs managed\n"));
        assert!(out.contains("      - id: spec-driven-docs-verify\n"));
        assert!(out.contains("        entry: sdd verify\n"));
        assert!(out.contains("      - id: adr-filename-shape\n"));
        assert!(out.contains("        files: '^_docs/decisions/.*\\.md$'\n"));
    }

    #[test]
    fn always_run_gates_do_not_take_filenames() {
        let out = render_gates(&RenderOptions::default());
        assert_eq!(
            out.matches("always_run: true").count(),
            out.matches("pass_filenames: false").count()
        );
    }

    #[test]
    fn an_apostrophe_in_a_name_would_be_doubled() {
        assert_eq!(quoted("it's"), "'it''s'");
    }

    #[test]
    fn the_block_splices_into_a_plain_config() {
        let block = render_block(&RenderOptions::default());
        let spliced = crate::domain::marker::splice("repos:\n", &block).unwrap();
        let (base, found) = crate::domain::marker::split_block(&spliced).unwrap();
        assert_eq!(base, "repos:\n");
        assert_eq!(found.as_deref(), Some(block.as_str()));
    }
}
