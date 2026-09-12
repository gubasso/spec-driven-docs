//! Render the managed documentation block for a root `AGENTS.md`.
//!
//! The block is the canonical documentation-routing section, wrapped in the
//! `AGENTS.md` markers with the profile's documentation root substituted and
//! the project's writing-style selection rendered as one route line, or as
//! no line where the selection is `none`. The installer places it; this
//! only produces the bytes. The content lives in the embedded snippet, so
//! the block and the snippet cannot drift.

use crate::domain::instance_config::WritingStyle;
use crate::domain::marker::{AGENTS_BEGIN, AGENTS_END};

/// The list item that stands for the route in the snippet. It is a list
/// item rather than a bare placeholder so a markdown formatter reads the
/// snippet as the list it is.
const ROUTE_PLACEHOLDER: &str = "- {writing_style}";

/// The embedded documentation snippet, with `{docs_root}` and
/// `{writing_style}` unresolved.
#[must_use]
pub fn snippet() -> &'static str {
    crate::embedded::SNIPPETS
        .get_file("AGENTS-docs.md")
        .and_then(include_dir::File::contents_utf8)
        .unwrap_or_default()
}

/// The one route line the block carries for a selection, or `None` where
/// the selection installs no route.
#[must_use]
pub fn route_line(selection: &WritingStyle) -> Option<String> {
    selection
        .route()
        .map(|route| format!("- Read the writing style before you author or edit prose: {route}."))
}

/// The complete marked block for the given documentation root and
/// writing-style selection, newline-terminated.
#[must_use]
#[allow(
    clippy::literal_string_with_formatting_args,
    reason = "the braces are the block template's placeholder, not a formatting argument"
)]
pub fn render_block(docs_root: &str, selection: &WritingStyle) -> String {
    let mut body = String::new();
    for line in snippet().replace("{docs_root}", docs_root).lines() {
        let line = if line == ROUTE_PLACEHOLDER {
            match route_line(selection) {
                Some(route) => route,
                None => continue,
            }
        } else {
            line.to_string()
        };
        body.push_str(&line);
        body.push('\n');
    }
    let body = body.trim_end_matches('\n');
    format!("{AGENTS_BEGIN}\n{body}\n{AGENTS_END}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::instance_config::WritingSource;

    #[test]
    fn the_block_carries_the_markers_and_the_root() {
        let block = render_block("docs", &WritingStyle::default());
        assert!(block.starts_with("<!-- BEGIN spec-driven-docs docs -->\n"));
        assert!(block.ends_with("<!-- END spec-driven-docs docs -->\n"));
        assert!(block.contains(
            "Read the writing style before you author or edit prose: `sdd method writing-style`."
        ));
        assert!(!block.contains("{docs_root}"));
        assert!(!block.contains("{writing_style}"));
        assert!(!block.contains("simple-english"));
    }

    #[test]
    fn the_underscore_root_reaches_the_block() {
        let block = render_block("_docs", &WritingStyle::default());
        assert!(block.contains(
            "Read the writing style before you author or edit prose: `sdd method writing-style`."
        ));
        assert!(block.contains("`_docs/specs/SPEC-<domain>.md`"));
    }

    #[test]
    fn the_route_matches_each_selection() {
        let project = WritingStyle {
            source: WritingSource::Project,
            path: Some("docs/STYLE.md".to_string()),
        };
        assert!(
            render_block("docs", &project).contains(
                "Read the writing style before you author or edit prose: `docs/STYLE.md`."
            )
        );
        let none = WritingStyle {
            source: WritingSource::None,
            path: None,
        };
        let block = render_block("docs", &none);
        assert!(!block.contains("writing style"), "{block}");
        assert!(!block.contains("{writing_style}"));
        assert!(
            !block.contains("\n\n<!-- END"),
            "the removed route left a blank line:\n{block}"
        );
    }
}
