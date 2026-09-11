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

// sdd: permanent the braces are the wiring template's placeholder, not a formatting argument
#[allow(clippy::literal_string_with_formatting_args)]
fn substitute_root(pattern: &str, docs_root: &str) -> String {
    pattern.replace("{docs_root}", docs_root)
}

/// Escape one literal character for a regex.
fn escape(out: &mut String, ch: char) {
    if ".^$+()|[]{}\\*?".contains(ch) {
        out.push('\\');
    }
    out.push(ch);
}

/// Project one glob onto a Python regex for pre-commit's `files:` field.
///
/// # The contract is one-directional
///
/// The rendered selection MUST match every path the matcher judges, and MAY
/// match more. `globset` compiles to a Rust byte regex and pre-commit
/// applies Python `re.search`, and the two languages are not the same, so
/// promising equivalence would mean writing and maintaining a converter
/// between them. `PathFilter` stays authoritative at runtime and drops the
/// excess, which makes a lossy projection cost a wasted invocation and never
/// a missed violation.
///
/// The restricted grammar `crate::domain::path_filter` enforces is what
/// makes the projection small: a leading `!` or `#`, an absolute path, and a
/// `..` component are already refused, so this handles `*`, `**`, `?`, a
/// character class, a brace alternation, and literals.
fn glob_to_regex(glob: &str) -> String {
    let mut out = String::from("^");
    let bytes: Vec<char> = glob.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            '*' if bytes.get(i + 1) == Some(&'*') => {
                if bytes.get(i + 2) == Some(&'/') {
                    // `**/` matches zero or more leading directories.
                    out.push_str("(?:.*/)?");
                    i += 3;
                } else {
                    out.push_str(".*");
                    i += 2;
                }
            }
            '*' => {
                out.push_str("[^/]*");
                i += 1;
            }
            '?' => {
                out.push_str("[^/]");
                i += 1;
            }
            '[' => {
                let close = bytes[i..].iter().position(|c| *c == ']').map(|p| i + p);
                if let Some(close) = close {
                    out.push('[');
                    let mut j = i + 1;
                    if bytes.get(j) == Some(&'!') {
                        out.push('^');
                        j += 1;
                    }
                    for ch in &bytes[j..close] {
                        out.push(*ch);
                    }
                    out.push(']');
                    i = close + 1;
                } else {
                    escape(&mut out, '[');
                    i += 1;
                }
            }
            '{' => {
                let close = bytes[i..].iter().position(|c| *c == '}').map(|p| i + p);
                if let Some(close) = close {
                    out.push_str("(?:");
                    let branches: String = bytes[i + 1..close].iter().collect();
                    let rendered: Vec<String> = branches
                        .split(',')
                        .map(|branch| {
                            let whole = glob_to_regex(branch);
                            whole
                                .trim_start_matches('^')
                                .trim_end_matches('$')
                                .to_string()
                        })
                        .collect();
                    out.push_str(&rendered.join("|"));
                    out.push(')');
                    i = close + 1;
                } else {
                    escape(&mut out, '{');
                    i += 1;
                }
            }
            ch => {
                escape(&mut out, ch);
                i += 1;
            }
        }
    }
    out.push('$');
    out
}

/// Project many globs onto one alternation, as pre-commit takes one regex.
fn render_patterns(globs: &[&str], docs_root: &str) -> Option<String> {
    let rendered: Vec<String> = globs
        .iter()
        .map(|glob| glob_to_regex(&substitute_root(glob, docs_root)))
        .collect();
    match rendered.len() {
        0 => None,
        1 => Some(rendered[0].clone()),
        _ => {
            let inner: Vec<String> = rendered
                .iter()
                .map(|r| r.trim_start_matches('^').trim_end_matches('$').to_string())
                .collect();
            Some(format!("^(?:{})$", inner.join("|")))
        }
    }
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
        // An `always_run` row takes no filenames, so pre-commit ignores
        // `files:` and `exclude:` there. The registry still declares what
        // the row judges, and `PathFilter` still applies it; emitting a
        // selector pre-commit never reads would tell a reader of the block
        // something untrue.
        if !gate.always_run {
            if let Some(files) = render_patterns(gate.include, &options.docs_root) {
                let _ = writeln!(out, "{field}files: {}", quoted(&files));
            }
        }
        if let Some(types) = gate.types {
            let _ = writeln!(out, "{field}types: [{types}]");
        }
        if !gate.always_run {
            if let Some(exclude) = render_patterns(gate.exclude, &options.docs_root) {
                let _ = writeln!(out, "{field}exclude: {}", quoted(&exclude));
            }
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
        assert!(out.contains("        files: '^docs/decisions/[^/]*\\.md$'\n"));
        assert!(out.contains("        exclude: '^docs/decisions/.*$'\n"));
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
        assert!(out.contains("        files: '^_docs/decisions/[^/]*\\.md$'\n"));
    }

    #[test]
    fn a_glob_projects_onto_an_anchored_regex() {
        assert_eq!(glob_to_regex("README.md"), r"^README\.md$");
        assert_eq!(glob_to_regex("_docs/*.md"), r"^_docs/[^/]*\.md$");
        assert_eq!(glob_to_regex("_docs/**/*.md"), r"^_docs/(?:.*/)?[^/]*\.md$");
        assert_eq!(glob_to_regex("**/AGENTS.md"), r"^(?:.*/)?AGENTS\.md$");
        assert_eq!(glob_to_regex("vendor/**"), r"^vendor/.*$");
        assert_eq!(glob_to_regex("a?.md"), r"^a[^/]\.md$");
        assert_eq!(glob_to_regex("[abc].md"), r"^[abc]\.md$");
        assert_eq!(glob_to_regex("{SPEC,ADR}-a.md"), r"^(?:SPEC|ADR)-a\.md$");
    }

    #[test]
    fn several_includes_render_as_one_alternation() {
        let rendered = render_patterns(&["a.md", "b/*.md"], "_docs").expect("two patterns render");
        assert_eq!(rendered, r"^(?:a\.md|b/[^/]*\.md)$");
        assert_eq!(render_patterns(&[], "_docs"), None);
    }

    /// The one-directional contract, asserted over every row: pre-commit's
    /// rendered selection matches everything the matcher judges.
    ///
    /// SATISFIES release:a-delivered-gate-reads-what-the-convention-owns
    #[test]
    fn the_rendered_pattern_is_a_superset_of_the_matcher() {
        use crate::domain::path_filter::{Layer, PathFilter, Pattern};

        let corpus = [
            "README.md",
            "AGENTS.md",
            "method/AGENTS.md",
            "method/08-gates.md",
            "CHANGELOG.md",
            "_docs/specs/SPEC-release.md",
            "_docs/decisions/ADR-a-choice.md",
            "_docs/reference/known-issues/KI-a-case.md",
            "_docs/reference/tracking.yaml",
            "_docs/guides/release.md",
            "comparison-docs/COMPARISON-tools.md",
            ".spec-driven-docs/manifest.json",
            "src/gates.rs",
            "vendor/third/lib.rs",
        ];

        for gate in GATES {
            let filter = PathFilter::build(
                gate.include
                    .iter()
                    .map(|g| Pattern::new(substitute_root(g, "_docs"), Layer::Registry))
                    .collect(),
                gate.exclude
                    .iter()
                    .map(|g| Pattern::new(substitute_root(g, "_docs"), Layer::Registry))
                    .collect(),
            )
            .expect("every registry pattern compiles");

            let files = render_patterns(gate.include, "_docs");
            let excludes = render_patterns(gate.exclude, "_docs");

            for path in corpus {
                if !filter.judges(camino::Utf8Path::new(path)) {
                    continue;
                }
                if let Some(files) = &files {
                    let selector = regex::Regex::new(files).expect("the projection compiles");
                    assert!(
                        selector.is_match(path),
                        "{}: the matcher judges {path} and the rendered files: {files} does not select it",
                        gate.id
                    );
                }
                if let Some(excludes) = &excludes {
                    let selector = regex::Regex::new(excludes).expect("the projection compiles");
                    assert!(
                        !selector.is_match(path),
                        "{}: the matcher judges {path} and the rendered exclude: {excludes} drops it",
                        gate.id
                    );
                }
            }
        }
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
