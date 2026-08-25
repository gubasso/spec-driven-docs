//! Gate: a document states what is true now.
//!
//! This reports the markers that narrate how a document got that way —
//! "formerly", "used to be", "this replaces", "inherited from". Code is
//! stripped before matching, because a document stating the rule quotes the
//! words it forbids, and both code constructs are stripped on `CommonMark`'s
//! terms: a fence belongs to the container it opened in and ends with it,
//! and a code span pairs equal backtick runs but never crosses a blank line,
//! so prose is judged one paragraph at a time. Decision records are excluded
//! by wiring — history is their job.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::DocumentStatesThePresent];

const MARKERS: &[&str] = &["formerly", "used to be", "this replaces", "inherited from"];

/// Blank every closed code span, preserving newlines so line numbers
/// survive; an unclosed run is literal text and stays.
fn strip_spans(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '`' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let mut j = i;
        while j < chars.len() && chars[j] == '`' {
            j += 1;
        }
        let run = j - i;
        let mut end = None;
        let mut k = j;
        while k < chars.len() {
            if chars[k] == '`' {
                let mut m = k;
                while m < chars.len() && chars[m] == '`' {
                    m += 1;
                }
                if m - k == run {
                    end = Some(m);
                    break;
                }
                k = m;
            } else {
                k += 1;
            }
        }
        if let Some(end) = end {
            for &ch in &chars[i..end] {
                out.push(if ch == '\n' { '\n' } else { ' ' });
            }
            i = end;
        } else {
            for _ in 0..run {
                out.push('`');
            }
            i = j;
        }
    }
    out
}

struct Paragraph {
    prose: Vec<String>,
    lines: Vec<usize>,
    raw: Vec<String>,
}

fn report(paragraph: &mut Paragraph, file: &str, violations: &mut Vec<Violation>) {
    if paragraph.prose.is_empty() {
        return;
    }
    let mut text = String::new();
    for line in &paragraph.prose {
        text.push_str(line);
        text.push('\n');
    }
    let stripped = strip_spans(&text);
    for (index, line) in stripped.lines().enumerate().take(paragraph.prose.len()) {
        let lower = line.to_lowercase();
        if MARKERS.iter().any(|marker| lower.contains(marker)) {
            violations.push(Violation::Finding(Finding::on_line(
                RuleId::DocumentStatesThePresent,
                file,
                paragraph.lines[index],
                paragraph.raw[index].clone(),
            )));
        }
    }
    paragraph.prose.clear();
    paragraph.lines.clear();
    paragraph.raw.clear();
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

fn judge(file: &str, text: &str, violations: &mut Vec<Violation>) {
    let mut fence: Option<Fence> = None;
    let mut paragraph = Paragraph {
        prose: Vec::new(),
        lines: Vec::new(),
        raw: Vec::new(),
    };

    for (number, raw) in text.lines().enumerate() {
        let (quotes, indent, content) = container(raw);

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
                    continue;
                }
                Some(open)
                    if delimiter == open.delimiter
                        && length >= open.length
                        && content.trim_start_matches(delimiter).trim_end().is_empty() =>
                {
                    fence = None;
                    continue;
                }
                Some(_) => continue,
            }
        }
        if fence.is_some() {
            continue;
        }
        if content.is_empty() {
            report(&mut paragraph, file, violations);
            continue;
        }
        paragraph.prose.push(content.to_string());
        paragraph.lines.push(number + 1);
        paragraph.raw.push(raw.to_string());
    }
    report(&mut paragraph, file, violations);
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
    fn accepts_present_tense_prose() {
        assert!(run_on("# Present\n\nThe rule applies now.\n").is_empty());
    }

    #[test]
    fn rejects_narration_with_the_offending_line() {
        let out = run_on("# History\n\nThis replaces an older rule.\n");
        assert_eq!(
            out,
            vec![
                "FAIL docs-format:document-states-the-present doc.md:3: This replaces an older rule."
                    .to_string()
            ]
        );
    }

    #[test]
    fn a_fenced_quotation_of_the_markers_is_code() {
        assert!(run_on("# Doc\n\n```text\nformerly a rule\n```\n").is_empty());
    }

    #[test]
    fn a_code_span_quoting_a_marker_is_code() {
        assert!(run_on("The gate rejects `formerly` in prose.\n").is_empty());
    }

    #[test]
    fn a_span_cannot_cross_a_blank_line() {
        let out = run_on("An odd `backtick here.\n\nformerly narrated` prose.\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("doc.md:3"));
    }

    #[test]
    fn a_fence_left_open_ends_with_its_container() {
        let out = run_on("> ```\n> code\n\nformerly outside the quote.\n");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("doc.md:4"));
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert_eq!(run_on("Formerly this was different.\n").len(), 1);
    }
}
