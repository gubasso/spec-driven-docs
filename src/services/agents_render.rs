//! Render the managed documentation block for a root `AGENTS.md`.
//!
//! The block is the canonical documentation-routing section plus the
//! `SimpleEnglish` default directive, wrapped in the `AGENTS.md` markers with
//! the profile's documentation root substituted. The installer places it;
//! this only produces the bytes. The content lives in the embedded snippet,
//! so the block and the snippet cannot drift.

use crate::domain::marker::{AGENTS_BEGIN, AGENTS_END};

/// The embedded documentation snippet, with `{docs_root}` unresolved.
#[must_use]
pub fn snippet() -> &'static str {
    crate::embedded::SNIPPETS
        .get_file("AGENTS-docs.md")
        .and_then(include_dir::File::contents_utf8)
        .unwrap_or_default()
}

/// The complete marked block for the given documentation root, newline-
/// terminated.
#[must_use]
// The braces are the template placeholder itself, not a formatting argument.
#[allow(clippy::literal_string_with_formatting_args)]
pub fn render_block(docs_root: &str) -> String {
    let body = snippet().replace("{docs_root}", docs_root);
    let body = body.trim_end_matches('\n');
    format!("{AGENTS_BEGIN}\n{body}\n{AGENTS_END}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_block_carries_the_markers_and_the_root() {
        let block = render_block("docs");
        assert!(block.starts_with("<!-- BEGIN spec-driven-docs docs -->\n"));
        assert!(block.ends_with("<!-- END spec-driven-docs docs -->\n"));
        assert!(block.contains("docs/specs/SPEC-simple-english.md"));
        assert!(!block.contains("{docs_root}"));
        assert!(block.contains("SimpleEnglish `Plain` mode"));
    }

    #[test]
    fn the_underscore_root_reaches_the_block() {
        let block = render_block("_docs");
        assert!(block.contains("_docs/specs/SPEC-simple-english.md"));
    }
}
