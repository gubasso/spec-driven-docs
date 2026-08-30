//! Render the gate registry as the managed pre-commit block.
//!
//! The registry is the one declaration and this is its one delivery: the
//! block an instance's configuration carries, rendered at install time and
//! never committed anywhere in between. A gate reaches an instance because
//! it is in the registry, so it cannot reach the payload and miss the
//! wiring. What the registry contains is `gates`' business; where the
//! output lands is the caller's.
//!
//! There is deliberately no second shape. A `.pre-commit-hooks.yaml` would
//! serve repositories that never adopt this framework, and most gates read
//! an instance layout those repositories do not have.

use std::fmt::Write as _;

use crate::domain::marker;
use crate::gates::GATES;

/// The pre-commit language every entry declares.
///
/// An instance runs `sdd` from its own PATH, which is what `system` means;
/// no other language has a caller.
const LANGUAGE: &str = "system";

/// Everything a render depends on.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// What replaces `{docs_root}` in wiring patterns — the literal root
    /// the instance's profile selected.
    pub docs_root: String,
    /// The command prefix an entry invokes, e.g. `sdd` or `cargo run -q --`.
    pub entry: String,
    /// The sequence-item indentation of the consumer's `repos:` entries.
    pub indent: String,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            docs_root: "_docs".to_string(),
            entry: "sdd".to_string(),
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

/// Render the gate entries alone, without the markers or the verifier.
fn render_gates(options: &RenderOptions) -> String {
    let item = format!("{0}    - ", options.indent);
    let field = format!("{0}      ", options.indent);
    let mut out = String::new();
    for gate in GATES {
        let _ = writeln!(out, "{item}id: {}", gate.id);
        let _ = writeln!(out, "{field}name: {}", quoted(gate.name));
        let _ = writeln!(out, "{field}entry: {} gate {}", options.entry, gate.id);
        let _ = writeln!(out, "{field}language: {LANGUAGE}");
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
    let _ = writeln!(out, "{indent}      language: {LANGUAGE}");
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
    fn every_gate_renders_its_wiring_fields() {
        let out = render_gates(&RenderOptions::default());
        assert!(out.starts_with("      - id: adr-cites-a-live-rule\n"));
        assert!(out.contains("        entry: sdd gate adr-filename-shape\n"));
        assert!(out.contains("        language: system\n"));
        assert!(out.contains("        types: [markdown]\n"));
        assert_eq!(out.matches("- id: ").count(), crate::gates::GATES.len());
    }

    /// The profile picks the root, so a `docs` instance wires `docs` paths.
    #[test]
    fn the_docs_root_reaches_every_templated_pattern() {
        let out = render_gates(&RenderOptions {
            docs_root: "docs".to_string(),
            ..RenderOptions::default()
        });
        assert!(out.contains("        files: '^docs/decisions/.*\\.md$'\n"));
        assert!(out.contains("        exclude: '^docs/decisions/'\n"));
        assert!(!out.contains("{docs_root}"));
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
