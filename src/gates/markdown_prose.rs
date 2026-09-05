//! Shared markdown structure for the prose gates.
//!
//! One scan classifies every source line of a markdown document: front
//! matter, a fenced code block, an HTML comment, a heading, a table row, a
//! thematic break, a blockquote, a list item, or plain prose. A prose line
//! also carries its container prefix stripped and its ordered-list depth, so
//! a gate can measure the prose alone and leave every structural line exact.
//! What a gate does with a prose line is the gate's business.

/// What one source line is, for a prose gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    /// Inside the leading `---` front matter, or its fence lines.
    FrontMatter,
    /// A fenced code block, opening and closing fences included.
    Fence,
    /// An HTML comment line (a directive is classified separately by the gate).
    Comment,
    /// A heading line.
    Heading,
    /// A table row or delimiter row.
    Table,
    /// A blank line, a thematic break, or a link reference definition.
    Blank,
    /// Prose the gate measures. Carries the container-stripped content, the
    /// 1-based line number, and whether it opens a numbered list item.
    Prose(Prose),
}

/// A prose line, stripped of its blockquote and list-marker prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prose {
    /// The 1-based source line number.
    pub number: usize,
    /// The content after any blockquote markers and one list marker.
    pub content: String,
    /// True where the line opens an ordered (numbered) list item.
    pub ordered_item: bool,
}

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

fn is_thematic_break(content: &str) -> bool {
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

fn ordered_marker(content: &str) -> Option<usize> {
    let digits = content.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let rest = &content[digits..];
    let ok = matches!(rest.chars().next(), Some('.' | ')'))
        && matches!(rest.chars().nth(1), None | Some(' ' | '\t'));
    ok.then_some(digits + 1)
}

fn bullet_marker(content: &str) -> Option<usize> {
    let ok = matches!(content.chars().next(), Some('-' | '*' | '+'))
        && matches!(content.chars().nth(1), None | Some(' ' | '\t'));
    ok.then_some(1)
}

fn is_definition(content: &str) -> bool {
    content
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
        .is_some_and(|(_, after)| after.starts_with(':'))
}

/// Classify every line of a markdown document.
#[must_use]
#[allow(clippy::too_many_lines, clippy::option_if_let_else)]
pub fn classify(text: &str) -> Vec<LineKind> {
    let mut out = Vec::new();
    let mut fence: Option<Fence> = None;
    let mut in_front_matter = false;
    let mut in_comment = false;
    let mut in_toc = false;

    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let (quotes, indent, content) = container(raw);

        if index == 0 && raw == "---" {
            in_front_matter = true;
            out.push(LineKind::FrontMatter);
            continue;
        }
        if in_front_matter {
            if raw == "---" || raw == "..." {
                in_front_matter = false;
            }
            out.push(LineKind::FrontMatter);
            continue;
        }
        if in_comment {
            out.push(LineKind::Comment);
            if content.contains("-->") {
                in_comment = false;
            }
            continue;
        }

        if let Some(open) = &fence
            && !content.is_empty()
            && (quotes < open.quotes || indent < open.indent)
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
                Some(open)
                    if delimiter == open.delimiter
                        && length >= open.length
                        && content.trim_start_matches(delimiter).trim_end().is_empty() =>
                {
                    fence = None;
                }
                Some(_) => {}
            }
            out.push(LineKind::Fence);
            continue;
        }
        if fence.is_some() {
            out.push(LineKind::Fence);
            continue;
        }
        // The generated table of contents is not authored prose. Its entries
        // mirror the requirement heading titles, dashes and all, so measuring
        // it would flag the generator's output. Skip the marked region.
        if content.trim() == "<!--TOC-->" {
            in_toc = !in_toc;
            out.push(LineKind::Comment);
            continue;
        }
        if in_toc {
            out.push(LineKind::Blank);
            continue;
        }
        if content.starts_with("<!--") && !content.contains("-->") {
            in_comment = true;
            out.push(LineKind::Comment);
            continue;
        }
        if content.starts_with("<!--") {
            out.push(LineKind::Comment);
            continue;
        }
        if content.is_empty() || is_thematic_break(content) || is_definition(content) {
            out.push(LineKind::Blank);
            continue;
        }
        if content.starts_with('#') {
            out.push(LineKind::Heading);
            continue;
        }
        if content.starts_with('|') || content.starts_with('<') {
            out.push(LineKind::Table);
            continue;
        }

        let (ordered_item, marker) = if let Some(width) = ordered_marker(content) {
            (true, width)
        } else if let Some(width) = bullet_marker(content) {
            (false, width)
        } else {
            (false, 0)
        };
        let stripped = content[marker..].trim_start().to_string();
        out.push(LineKind::Prose(Prose {
            number,
            content: stripped,
            ordered_item,
        }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_fences_headings_tables_and_front_matter() {
        let text = "---\ntitle: x\n---\n# Heading\n\nA plain paragraph.\n\n```text\ncode\n```\n\n| a | b |\n| - | - |\n- a list item\n1. a numbered item\n";
        let kinds = classify(text);
        let prose: Vec<&Prose> = kinds
            .iter()
            .filter_map(|k| {
                if let LineKind::Prose(p) = k {
                    Some(p)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(prose.len(), 3);
        assert_eq!(prose[0].content, "A plain paragraph.");
        assert_eq!(prose[1].content, "a list item");
        assert!(prose[2].ordered_item);
        assert_eq!(prose[2].content, "a numbered item");
    }

    #[test]
    fn a_multi_line_comment_is_not_prose() {
        let text = "<!-- a note\nover two lines -->\ntext after.\n";
        let prose: Vec<String> = classify(text)
            .into_iter()
            .filter_map(|k| {
                if let LineKind::Prose(p) = k {
                    Some(p.content)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(prose, vec!["text after.".to_string()]);
    }
}
