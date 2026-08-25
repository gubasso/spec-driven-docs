//! Gate: prose stays unwrapped — a paragraph is one source line.
//!
//! This reports the continuation lines a hard wrap leaves behind: a plain
//! text line whose predecessor was still mid-paragraph. Blocks that own
//! their own line structure — fenced code, tables, headings, thematic
//! breaks, HTML, link definitions, front matter — never continue a
//! paragraph, and an explicit hard break (trailing backslash or two
//! spaces) is a deliberate new line, not a wrap. A fence belongs to the
//! container it opened in and ends with it, on `CommonMark`'s terms.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::ProseStaysUnwrapped];

const RULE: RuleId = RuleId::ProseStaysUnwrapped;

struct Fence {
    delimiter: char,
    length: usize,
    quotes: usize,
    indent: usize,
}

fn container(line: &str) -> (usize, usize, &str) {
    let mut rest = line;
    let mut quotes = 0;
    loop {
        let trimmed = rest.trim_start_matches([' ', '\t']);
        if let Some(after) = trimmed.strip_prefix('>') {
            quotes += 1;
            rest = after.strip_prefix([' ', '\t']).unwrap_or(after);
        } else {
            break;
        }
    }
    let content = rest.trim_start_matches([' ', '\t']);
    let indent = rest.len() - content.len();
    (quotes, indent, content)
}

fn fence_delimiter(content: &str) -> Option<(char, usize)> {
    let first = content.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let length = content.chars().take_while(|&c| c == first).count();
    (length >= 3).then_some((first, length))
}

/// A setext underline or thematic break: one marker character, spaces
/// between, and enough of them to be a line rather than a list marker.
fn is_line_marker(content: &str) -> bool {
    let mut marker = None;
    let mut count = 0;
    for ch in content.chars() {
        match ch {
            ' ' | '\t' => {}
            '-' | '=' | '*' | '_' => {
                if *marker.get_or_insert(ch) != ch {
                    return false;
                }
                count += 1;
            }
            _ => return false,
        }
    }
    match marker {
        Some('=') => count >= 1,
        Some('-') => count >= 2,
        Some(_) => count >= 3,
        None => false,
    }
}

fn is_ordered_marker(content: &str) -> bool {
    let digits = content.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 9 {
        return false;
    }
    let rest = &content[digits..];
    matches!(rest.chars().next(), Some('.' | ')'))
        && matches!(rest.chars().nth(1), None | Some(' ' | '\t'))
}

fn is_bullet_marker(content: &str) -> bool {
    matches!(content.chars().next(), Some('-' | '*' | '+'))
        && matches!(content.chars().nth(1), None | Some(' ' | '\t'))
}

/// A link reference definition or footnote definition start.
fn is_definition(content: &str) -> bool {
    let Some(rest) = content.strip_prefix('[') else {
        return false;
    };
    rest.split_once(']')
        .is_some_and(|(_, after)| after.starts_with(':'))
}

/// What a line means for the paragraph running above it.
enum Kind {
    /// Ends any paragraph; never continues one.
    Boundary,
    /// Opens a paragraph a following text line would continue.
    Opens,
    /// Plain paragraph text: continues an open paragraph.
    Text,
}

fn classify(content: &str) -> Kind {
    if content.is_empty()
        || content.starts_with('#')
        || content.starts_with('<')
        || content.starts_with('|')
        || is_line_marker(content)
        || is_definition(content)
    {
        return Kind::Boundary;
    }
    if is_bullet_marker(content) || is_ordered_marker(content) {
        return Kind::Opens;
    }
    Kind::Text
}

fn hard_break(raw: &str) -> bool {
    raw.ends_with('\\') || raw.ends_with("  ")
}

fn judge(file: &str, text: &str, violations: &mut Vec<Violation>) {
    let mut fence: Option<Fence> = None;
    let mut in_front_matter = false;
    let mut in_comment = false;
    let mut open = false;

    for (number, raw) in text.lines().enumerate() {
        let (quotes, indent, content) = container(raw);

        if number == 0 && raw == "---" {
            in_front_matter = true;
            continue;
        }
        if in_front_matter {
            if raw == "---" || raw == "..." {
                in_front_matter = false;
            }
            continue;
        }
        if in_comment {
            if content.contains("-->") {
                in_comment = false;
            }
            open = false;
            continue;
        }

        if let Some(opened) = &fence
            && !content.is_empty()
            && (quotes < opened.quotes || indent < opened.indent)
        {
            fence = None;
        }
        if let Some((delimiter, length)) = fence_delimiter(content) {
            match &fence {
                None => {
                    fence = Some(Fence {
                        delimiter,
                        length,
                        quotes,
                        indent,
                    });
                }
                Some(opened)
                    if delimiter == opened.delimiter
                        && length >= opened.length
                        && content.trim_start_matches(delimiter).trim_end().is_empty() =>
                {
                    fence = None;
                }
                Some(_) => {}
            }
            open = false;
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if content.starts_with("<!--") && !content.contains("-->") {
            in_comment = true;
            open = false;
            continue;
        }

        match classify(content) {
            Kind::Boundary => open = false,
            Kind::Opens => open = !hard_break(raw),
            Kind::Text => {
                if open {
                    violations.push(Violation::Finding(Finding::on_line(
                        RULE,
                        file,
                        number + 1,
                        "hard wrap; join with the previous line",
                    )));
                }
                open = !hard_break(raw);
            }
        }
    }
}

/// Judge every file pre-commit passed.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a file cannot be read.
pub fn run(ctx: &GateCtx, files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for file in files {
        let text = read_text(ctx, file)?;
        judge(file, &text, &mut violations);
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_on(text: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("doc.md"), text).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &["doc.md".to_string()])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_one_line_paragraphs_lists_and_tables() {
        assert!(
            run_on(
                "# Title\n\nA whole paragraph on one line, however long it runs.\n\n\
                 - one item on one line\n- another item\n1. ordered too\n\n\
                 | a | b |\n| - | - |\n| c | d |\n"
            )
            .is_empty()
        );
    }

    #[test]
    fn rejects_a_wrapped_paragraph_at_the_continuation_line() {
        let out = run_on("# Title\n\nA paragraph broken\nacross two lines.\n");
        assert_eq!(
            out,
            vec![
                "FAIL docs-format:prose-stays-unwrapped doc.md:4: \
                 hard wrap; join with the previous line"
                    .to_string()
            ]
        );
    }

    #[test]
    fn rejects_a_wrapped_list_item() {
        let out = run_on("- an item whose text\n  continues on a second line\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("doc.md:2"));
    }

    #[test]
    fn rejects_a_wrapped_blockquote_paragraph() {
        let out = run_on("> a quoted paragraph\n> wrapped across lines\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("doc.md:2"));
    }

    #[test]
    fn fenced_code_wraps_freely() {
        assert!(run_on("```text\nwrapped\nlines are\ncode\n```\n").is_empty());
    }

    #[test]
    fn a_fence_left_open_in_a_quote_ends_with_its_container() {
        let out = run_on("> ```\n> code\n\nplain text\nwrapped outside the quote\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("doc.md:5"));
    }

    #[test]
    fn an_explicit_hard_break_is_not_a_wrap() {
        assert!(run_on("first line\\\nsecond line\n\nspaced break  \nnext line\n").is_empty());
    }

    #[test]
    fn front_matter_and_comments_are_not_prose() {
        assert!(
            run_on("---\ntitle: x\ndate: y\n---\n\n<!-- a note\nover two lines -->\ntext\n")
                .is_empty()
        );
    }

    #[test]
    fn a_setext_underline_is_not_a_continuation() {
        assert!(run_on("A setext heading\n===\n\nAnother one\n---\n").is_empty());
    }

    #[test]
    fn a_new_block_after_text_is_not_a_continuation() {
        assert!(
            run_on("some text on one line\n## A heading\nsome text\n- a list starts\n| a |\n")
                .is_empty()
        );
    }

    #[test]
    fn a_lazy_continuation_under_a_list_item_is_a_wrap() {
        let out = run_on("- an item\nlazily continued\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("doc.md:2"));
    }
}
