//! Gate: every suppression says why it is there, and says it in the one
//! form that fits what it does.
//!
//! A suppression over a defect somewhere else names the case that justifies
//! it, and that case resolves to a record. A suppression this project chose
//! and keeps states its reason instead, because no record could carry a
//! retirement condition anyone can meet, and a record with an unmeetable
//! condition is the permanent mask the case rule exists to prevent. A
//! suppression with neither becomes permanent by default: the next reader
//! takes it for a design choice and nothing says what would retire it.
//!
//! A form counts only in a file the tool that honors it reads. `#[allow(`
//! is live Rust and a quotation in markdown; `noqa` is live in a Python or
//! shell comment and a quotation here. That scoping is what lets this file,
//! the specs and the method chapters name a form without being judged by
//! it. A form inside a string, inside a multi-line string, or inside a
//! document's fence is a quotation for the same reason. Binary files and
//! vendored trees are skipped, and the known-issues directory is exempt,
//! because a record may discuss suppressions.
//!
//! Three surfaces stay outside this scan: an extensionless shell script, a
//! block comment holding a suppression, and the `[lints]` table of a
//! manifest. The `simple-english-disable` marker stays outside too,
//! because `simple-english:an-exception-names-its-reason` already requires
//! a reason on it, and a second rule would name one defect twice.

use std::collections::BTreeSet;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::markdown_prose::{LineKind, classify};
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, GateError, GateResult, Violation, walk_files};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[
    RuleId::SuppressionNamesItsCase,
    RuleId::PermanentExceptionStatesItsReason,
];

const CASE: RuleId = RuleId::SuppressionNamesItsCase;
const PERMANENT: RuleId = RuleId::PermanentExceptionStatesItsReason;

/// The marker a permanent exception carries, ahead of its reason.
const MARKER: &str = "sdd: permanent";

/// How a form opens the text the tool reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Opener {
    /// The token opens the line, as a Rust attribute does.
    Line,
    /// The token follows a comment opener on the line.
    After(&'static str),
}

/// One family of suppression forms, with the file suffixes it is live in.
struct Family {
    suffixes: &'static [&'static str],
    opener: Opener,
    tokens: &'static [&'static str],
}

const FAMILIES: &[Family] = &[
    Family {
        suffixes: &[".rs"],
        opener: Opener::Line,
        tokens: &[
            "#[allow(",
            "#[expect(",
            "#![allow(",
            "#![expect(",
            "#[ignore",
        ],
    },
    Family {
        suffixes: &[".py"],
        opener: Opener::Line,
        tokens: &[
            "@pytest.mark.xfail",
            "@pytest.mark.skip",
            "@unittest.skip",
            "@unittest.expectedFailure",
        ],
    },
    Family {
        suffixes: &[".py", ".sh", ".bash", ".yaml", ".yml", ".toml"],
        opener: Opener::After("#"),
        tokens: &[
            "shellcheck disable=",
            "noqa",
            "type: ignore",
            "zizmor: ignore[",
        ],
    },
    Family {
        suffixes: &[".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs"],
        opener: Opener::After("//"),
        tokens: &["eslint-disable"],
    },
    Family {
        suffixes: &[".md", ".html"],
        opener: Opener::After("<!--"),
        tokens: &["dprint-ignore", "markdownlint-disable"],
    },
];

/// The comment opener a file's own syntax uses, for the line above a
/// suppression. The markdown forms have room on their own line, so they
/// take no window.
fn comment_opener(file: &str) -> Option<&'static str> {
    const OPENERS: &[(&[&str], &str)] = &[
        (&[".rs", ".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs"], "//"),
        (&[".py", ".sh", ".bash", ".yaml", ".yml", ".toml"], "#"),
    ];
    OPENERS
        .iter()
        .find(|(suffixes, _)| suffixes.iter().any(|suffix| file.ends_with(suffix)))
        .map(|(_, opener)| *opener)
}

/// Where the line's own comment opens, outside every quoted span.
///
/// Quotes are interpreted in the code that precedes the comment and never
/// inside it, so an apostrophe in comment prose closes nothing and a form
/// written in a string literal opens nothing.
fn comment_start(line: &str, opener: &str, quotes: &[char]) -> Option<usize> {
    let mut open: Option<char> = None;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if open.is_none() && line[index..].starts_with(opener) {
            return Some(index);
        }
        match (open, character) {
            (_, '\\') => escaped = true,
            (None, character) if quotes.contains(&character) => open = Some(character),
            (Some(quote), character) if character == quote => open = None,
            _ => {}
        }
    }
    None
}

/// The quote delimiters a file's own language carries. A JavaScript
/// template literal is a string, so a form written in one is a quotation.
fn quote_marks(file: &str) -> &'static [char] {
    if comment_opener(file) == Some("//") && !is_rust(file) {
        &['"', '\'', '`']
    } else {
        &['"', '\'']
    }
}

fn is_rust(file: &str) -> bool {
    // sdd: permanent the corpus convention is lowercase, and `.RS` is not Rust
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    file.ends_with(".rs")
}

fn is_python(file: &str) -> bool {
    // sdd: permanent the corpus convention is lowercase, and `.PY` is not Python
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    file.ends_with(".py")
}

/// The line's code, with its comment removed.
fn code_region<'a>(file: &str, line: &'a str) -> &'a str {
    comment_opener(file)
        .and_then(|opener| comment_start(line, opener, quote_marks(file)))
        .map_or(line, |index| &line[..index])
}

/// Whether the comment carries `token` right after one of its openers.
fn comment_carries(comment: &str, opener: &str, token: &str) -> bool {
    let mut start = 0usize;
    while let Some(offset) = comment[start..].find(opener) {
        let index = start + offset;
        if comment[index + opener.len()..]
            .trim_start_matches(' ')
            .starts_with(token)
        {
            return true;
        }
        start = index + opener.len();
    }
    false
}

/// Where a suppression on this line starts carrying its annotation.
///
/// A comment-borne form annotates from where the comment opens, so a case
/// id or a marker written in code earlier on the line is not the
/// suppression's. A line-borne form carries its annotation on the whole
/// line: a Rust attribute and a Python decorator both hold their reason
/// inside themselves.
fn suppression_at(file: &str, line: &str) -> Option<usize> {
    for family in FAMILIES {
        if !family.suffixes.iter().any(|suffix| file.ends_with(suffix)) {
            continue;
        }
        match family.opener {
            Opener::Line => {
                let code = line.trim_start();
                if family.tokens.iter().any(|token| code.starts_with(token)) {
                    return Some(0);
                }
            }
            Opener::After(opener) => {
                let Some(index) = comment_start(line, opener, quote_marks(file)) else {
                    continue;
                };
                let comment = &line[index..];
                if family
                    .tokens
                    .iter()
                    .any(|token| comment_carries(comment, opener, token))
                {
                    return Some(index);
                }
            }
        }
    }
    None
}

/// Every fence delimiter the line opens or closes in live code.
///
/// A delimiter inside an ordinary string or a comment is content, so
/// `delimiter = '\"\"\"'` opens nothing. The fence check runs before the
/// quote check, so a real triple quote is not read as one ordinary quote.
fn fence_toggles<'a>(line: &'a str, fences: &[&'a str], comment: Option<&str>) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut open: Option<char> = None;
    let mut escaped = false;
    let mut skip_to = 0usize;
    for (index, character) in line.char_indices() {
        if index < skip_to {
            continue;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if open.is_none() {
            if let Some(fence) = fences
                .iter()
                .find(|fence| line[index..].starts_with(**fence))
            {
                out.push(*fence);
                skip_to = index + fence.len();
                continue;
            }
            if comment.is_some_and(|opener| line[index..].starts_with(opener)) {
                break;
            }
        }
        match (open, character) {
            (_, '\\') => escaped = true,
            (None, '"' | '\'' | '`') => open = Some(character),
            (Some(quote), character) if character == quote => open = None,
            _ => {}
        }
    }
    out
}

/// Every line that sits inside a multi-line string of the file's own
/// language, where a form is content rather than a directive.
fn quoted_lines(file: &str, lines: &[&str]) -> Vec<bool> {
    let fences: &[&str] = if is_python(file) {
        &["\"\"\"", "'''"]
    } else if comment_opener(file) == Some("//") && !is_rust(file) {
        &["`"]
    } else {
        return vec![false; lines.len()];
    };
    let comment = comment_opener(file);
    let mut open: Option<&str> = None;
    lines
        .iter()
        .map(|line| {
            let was_open = open.is_some();
            // Inside a multi-line string the whole line is content, so only
            // the delimiter that closes it is read.
            let toggles = open.map_or_else(
                || fence_toggles(line, fences, comment),
                |fence| line.matches(fence).map(|_| fence).take(1).collect(),
            );
            for fence in toggles {
                match open {
                    None => open = Some(fence),
                    Some(current) if current == fence => open = None,
                    Some(_) => {}
                }
            }
            was_open && open.is_some()
        })
        .collect()
}

fn is_closing(line: &str) -> bool {
    line.contains("dprint-ignore-end") || line.contains("markdownlint-enable")
}

/// Whether a token starting at `index` opens on a word boundary, so a
/// longer word that ends in the token is not read as the token.
const fn on_a_boundary(text: &str, index: usize) -> bool {
    index == 0
        || !text.as_bytes()[index - 1].is_ascii_alphanumeric()
            && text.as_bytes()[index - 1] != b'-'
            && text.as_bytes()[index - 1] != b'_'
}

fn cited_cases(line: &str) -> impl Iterator<Item = String> + '_ {
    line.match_indices("KI-")
        .filter(|(index, _)| on_a_boundary(line, *index))
        .filter_map(|(index, _)| {
            let rest = &line[index + 3..];
            let slug: String = rest
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                .collect();
            // The slug closes on a boundary too, so `KI-vendor-quirkXYZ` is
            // one unknown case rather than a known one with a suffix.
            let closes = rest[slug.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
            (!slug.is_empty() && closes).then(|| format!("KI-{slug}"))
        })
}

/// The reason a permanent marker states on one line.
///
/// The reason is read from the marker's own line, so a marker that ends its
/// line states no reason. Reading past the line end would let the
/// suppression below a bare marker read as its reason. The marker opens on
/// a word boundary and closes on whitespace, so `not-sdd: permanent` and
/// `sdd: permanently` are different words.
fn permanent_reason(line: &str) -> Option<String> {
    let after = line
        .match_indices(MARKER)
        .filter(|(index, _)| on_a_boundary(line, *index))
        .map(|(index, _)| &line[index + MARKER.len()..])
        .find(|after| after.is_empty() || after.starts_with(char::is_whitespace))?;
    let rest = after
        .trim_end()
        .trim_end_matches("-->")
        .trim_end_matches("*/")
        .trim();
    Some(rest.to_string())
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(4096).any(|&b| b == 0)
}

/// One suppression, with the lines a reader can read its reason from.
struct Site {
    file: String,
    number: usize,
    line: String,
    annotation: Vec<String>,
}

impl Site {
    fn cites_a_case(&self) -> bool {
        self.annotation
            .iter()
            .any(|line| cited_cases(line).next().is_some())
    }

    fn reason(&self) -> Option<String> {
        self.annotation
            .iter()
            .find_map(|line| permanent_reason(line))
    }
}

fn unbalanced(text: &str) -> i32 {
    let mut depth = 0i32;
    let mut open: Option<u8> = None;
    let mut escaped = false;
    for byte in text.bytes() {
        if escaped {
            escaped = false;
            continue;
        }
        match (open, byte) {
            (_, b'\\') => escaped = true,
            (None, b'"' | b'\'') => open = Some(byte),
            (Some(quote), byte) if byte == quote => open = None,
            (None, b'(' | b'[') => depth += 1,
            (None, b')' | b']') => depth -= 1,
            _ => {}
        }
    }
    depth
}

/// Every line a reader can read the suppression's reason from: the comment
/// line above it when the file's syntax has one, the suppression's own
/// annotation region, and the lines its delimiters continue onto.
///
/// The lines stay separate, because a reason is read from the line its
/// marker sits on. Rust's own idiom carries the reason above the attribute,
/// and a multi-line inner attribute has no room on its own line.
fn annotation(file: &str, lines: &[&str], index: usize, start: usize) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(opener) = comment_opener(file) {
        if let Some(above) = index
            .checked_sub(1)
            .map(|previous| lines[previous].trim_start())
            .filter(|previous| previous.starts_with(opener))
        {
            out.push(above.to_string());
        }
    }
    out.push(lines[index][start..].to_string());
    // Delimiters are counted in the code alone, so punctuation in a trailing
    // comment cannot borrow the line below as this suppression's annotation.
    // A suppression that never balances owns its opening line only.
    let mut depth = unbalanced(code_region(file, lines[index]));
    let mut continuation = Vec::new();
    let mut next = index + 1;
    while depth > 0 && next < lines.len() {
        continuation.push(lines[next].to_string());
        depth += unbalanced(code_region(file, lines[next]));
        next += 1;
    }
    if depth == 0 {
        out.extend(continuation);
    }
    out
}

fn sites(ctx: &GateCtx) -> Result<Vec<Site>, GateError> {
    let mut sites = Vec::new();
    for file in walk_files(ctx) {
        if file
            .components()
            .any(|part| part.as_str() == "known-issues")
        {
            continue;
        }
        let bytes =
            std::fs::read(ctx.path(&file)).map_err(|source| GateError::io(file.clone(), source))?;
        if looks_binary(&bytes) {
            continue;
        }
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        let name = file.as_str().trim_start_matches("./").to_string();
        let lines: Vec<&str> = text.lines().collect();
        // A fence in a document holds an example of a form rather than a
        // live one, and every chapter that teaches a form shows it in a
        // fence. The shared classifier owns which lines those are, so this
        // gate keeps no second fence state machine.
        // sdd: permanent the corpus convention is lowercase, and `.MD` is not a document
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        let kinds = if name.ends_with(".md") {
            classify(&text)
        } else {
            Vec::new()
        };
        let quoted = quoted_lines(&name, &lines);
        for (index, line) in lines.iter().enumerate() {
            if matches!(
                kinds.get(index),
                Some(LineKind::Fence | LineKind::FrontMatter)
            ) || quoted[index]
            {
                continue;
            }
            let Some(start) = suppression_at(&name, line) else {
                continue;
            };
            if !is_closing(line) {
                sites.push(Site {
                    file: name.clone(),
                    number: index + 1,
                    line: (*line).to_string(),
                    annotation: annotation(&name, &lines, index, start),
                });
            }
        }
    }
    Ok(sites)
}

/// Judge every suppression in the repository.
///
/// # Errors
///
/// [`GateError::Io`] when a candidate file cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    let sites = sites(ctx)?;
    let mut violations = Vec::new();

    let mut caseless = Vec::new();
    for site in &sites {
        match (site.cites_a_case(), site.reason()) {
            (true, None) => {}
            (false, Some(reason)) if !reason.is_empty() => {}
            (false, Some(_)) => violations.push(Violation::Finding(Finding::on_line(
                PERMANENT,
                &site.file,
                site.number,
                "the permanent marker states no reason",
            ))),
            (true, Some(_)) => violations.push(Violation::Finding(Finding::on_line(
                PERMANENT,
                &site.file,
                site.number,
                "names a case and states a permanent exception",
            ))),
            (false, None) => caseless.push(site),
        }
    }
    if !caseless.is_empty() {
        violations.push(Violation::Finding(Finding::global(CASE, "")));
        for site in caseless {
            violations.push(Violation::Note(format!(
                "./{}:{}:{}",
                site.file, site.number, site.line
            )));
        }
    }

    let known: BTreeSet<String> = ki_records(ctx, args)?
        .iter()
        .filter_map(|record| {
            record
                .file_name()
                .map(|name| name.trim_end_matches(".md").to_string())
        })
        .collect();
    let cited: BTreeSet<String> = sites
        .iter()
        .flat_map(|site| site.annotation.iter().flat_map(|line| cited_cases(line)))
        .collect();
    for case in cited {
        if !known.contains(&case) {
            violations.push(Violation::Finding(Finding::global(
                CASE,
                format!("{case} resolves to no record"),
            )));
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let records = dir.path().join("_docs/reference/known-issues");
        std::fs::create_dir_all(&records).unwrap();
        std::fs::write(records.join("KI-vendor-quirk.md"), "# Quirk\n").unwrap();
        dir
    }

    fn run_on(name: &str, text: &str) -> Vec<String> {
        let dir = fixture();
        std::fs::write(dir.path().join(name), text).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn a_repository_without_suppressions_passes() {
        let dir = fixture();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        assert!(run(&ctx, &[]).unwrap().is_empty());
    }

    #[test]
    fn every_form_passes_when_it_names_a_record() {
        for (name, text) in [
            (
                "local.md",
                "<!-- markdownlint-disable MD013 KI-vendor-quirk -->\n",
            ),
            ("local.md", "<!-- dprint-ignore KI-vendor-quirk -->\n"),
            ("local.rs", "#[allow(dead_code)] // KI-vendor-quirk\n"),
            ("local.rs", "#[expect(dead_code)] // KI-vendor-quirk\n"),
            ("local.rs", "#[ignore = \"KI-vendor-quirk\"]\n"),
            (
                "local.sh",
                "# shellcheck disable=SC2329  # KI-vendor-quirk\n",
            ),
            ("local.py", "x = 1  # noqa: E501  KI-vendor-quirk\n"),
            ("local.py", "x = 1  # type: ignore  KI-vendor-quirk\n"),
            (
                "local.yml",
                "on: push  # zizmor: ignore[dangerous-triggers] KI-vendor-quirk\n",
            ),
            (
                "local.ts",
                "// eslint-disable-next-line no-eval KI-vendor-quirk\n",
            ),
        ] {
            assert!(run_on(name, text).is_empty(), "{name}: {text}");
        }
    }

    #[test]
    fn every_form_fails_when_it_says_nothing() {
        for (name, text) in [
            ("local.md", "<!-- markdownlint-disable MD013 -->\n"),
            ("local.rs", "#[allow(dead_code)]\n"),
            ("local.rs", "#![allow(clippy::unwrap_used)]\n"),
            ("local.sh", "# shellcheck disable=SC2329\n"),
            ("local.py", "x = 1  # noqa: E501\n"),
            ("local.ts", "// eslint-disable-next-line no-eval\n"),
        ] {
            let out = run_on(name, text);
            assert_eq!(out.len(), 2, "{name}: {text}");
            assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
            assert!(out[1].contains(&format!("{name}:1")));
        }
    }

    #[test]
    fn a_permanent_marker_with_a_reason_passes() {
        for (name, text) in [
            (
                "local.rs",
                "// sdd: permanent the braces are a template placeholder\n#[allow(clippy::x)]\n",
            ),
            (
                "local.rs",
                "#[allow(clippy::x)] // sdd: permanent the lint is wrong here\n",
            ),
            (
                "local.sh",
                "# shellcheck disable=SC2329  # sdd: permanent reached through a trap\n",
            ),
            (
                "local.md",
                "<!-- markdownlint-disable MD013 sdd: permanent the table is data -->\n",
            ),
        ] {
            assert!(run_on(name, text).is_empty(), "{name}: {text}");
        }
    }

    #[test]
    fn a_marker_on_the_line_above_a_multi_line_attribute_passes() {
        let text = "// sdd: permanent a test module panics as its failure signal\n#![allow(\n    clippy::unwrap_used\n)]\n";
        assert!(run_on("local.rs", text).is_empty());
    }

    #[test]
    fn a_non_comment_line_above_supplies_nothing() {
        let text = "let reason = \"KI-vendor-quirk\";\n#[allow(dead_code)]\n";
        let out = run_on("local.rs", text);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    #[test]
    fn a_marker_without_a_reason_is_rejected() {
        for text in [
            "#[allow(dead_code)] // sdd: permanent\n",
            "// sdd: permanent\n#[allow(dead_code)]\n",
        ] {
            let out = run_on("local.rs", text);
            assert_eq!(out.len(), 1, "{text}");
            assert!(
                out[0].ends_with(": the permanent marker states no reason"),
                "{text}"
            );
        }
    }

    #[test]
    fn an_expected_failure_is_a_suppression() {
        assert!(
            run_on(
                "test_x.py",
                "@pytest.mark.xfail(reason=\"KI-vendor-quirk\", strict=True)\ndef test_x():\n    pass\n"
            )
            .is_empty()
        );
        let out = run_on(
            "test_x.py",
            "@pytest.mark.xfail(strict=True)\ndef test_x():\n    pass\n",
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    #[test]
    fn a_form_inside_a_string_is_a_quotation() {
        for (name, text) in [
            ("local.py", "value = \"# noqa: E501\"\n"),
            ("local.sh", "printf '%s' '# shellcheck disable=SC2329'\n"),
            (
                "local.ts",
                "const form = \"// eslint-disable-next-line\";\n",
            ),
        ] {
            assert!(run_on(name, text).is_empty(), "{name}: {text}");
        }
    }

    #[test]
    fn a_fenced_example_in_a_document_is_a_quotation() {
        for fence in ["```markdown", "~~~markdown", "````markdown"] {
            let close = fence.trim_end_matches("markdown");
            let text = format!(
                "# Chapter\n\n{fence}\n<!-- markdownlint-disable MD013 -->\n{close}\n\nProse.\n"
            );
            assert!(run_on("chapter.md", &text).is_empty(), "{fence}");
        }
    }

    #[test]
    fn an_apostrophe_before_a_live_directive_does_not_hide_it() {
        let out = run_on("local.py", "value = \"it's long\"  # noqa: E501\n");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    #[test]
    fn a_marker_inside_a_longer_word_is_not_the_marker() {
        let out = run_on(
            "local.py",
            "x = 1  # noqa: E501 not-sdd: permanent reason\n",
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    #[test]
    fn a_case_written_in_code_before_the_comment_is_not_the_suppressions() {
        let out = run_on("local.py", "path = \"KI-vendor-quirk.md\"  # noqa: E501\n");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    #[test]
    fn a_multiline_expected_failure_carries_its_case() {
        let text = "@pytest.mark.xfail(\n    reason=\"KI-vendor-quirk\",\n    strict=True,\n)\ndef test_x():\n    pass\n";
        assert!(run_on("test_x.py", text).is_empty());
    }

    #[test]
    fn a_long_attribute_carries_its_case_past_any_line_count() {
        let lints = "    clippy::a_lint,\n".repeat(20);
        let text = format!("#[allow(\n{lints}    // KI-vendor-quirk\n)]\nfn f() {{}}\n");
        assert!(run_on("local.rs", &text).is_empty());
    }

    #[test]
    fn an_apostrophe_in_comment_prose_does_not_hide_a_later_directive() {
        let out = run_on("local.py", "value = 1  # don't reflow  # noqa: E501\n");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    #[test]
    fn comment_punctuation_does_not_extend_the_annotation() {
        let text = "#[allow(dead_code)] // (\nfn kept() {} // KI-vendor-quirk\n";
        let out = run_on("local.rs", text);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    #[test]
    fn a_line_carrying_multibyte_text_is_scanned_without_panicking() {
        assert!(run_on("local.py", "value = \"a 🤖 walks in\"  # a note\n").is_empty());
        let out = run_on("local.py", "value = \"a 🤖 walks in\"  # noqa: E501\n");
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn a_form_inside_a_multiline_string_is_a_quotation() {
        for (name, text) in [
            ("local.py", "DOC = \"\"\"\n# noqa: E501\n\"\"\"\n"),
            (
                "local.ts",
                "const doc = `\n// eslint-disable-next-line\n`;\n",
            ),
            ("local.ts", "const doc = `// eslint-disable-next-line`;\n"),
        ] {
            assert!(run_on(name, text).is_empty(), "{name}: {text}");
        }
    }

    #[test]
    fn a_quoted_fence_delimiter_opens_no_multiline_string() {
        for opener in [
            "delimiter = '\"\"\"'\n",
            "# a docstring opens with \"\"\"\n",
        ] {
            let out = run_on("local.py", &format!("{opener}value = 1  # noqa: E501\n"));
            assert_eq!(out.len(), 2, "{opener}");
            assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
        }
    }

    #[test]
    fn a_token_that_runs_on_is_not_the_token() {
        let out = run_on(
            "local.py",
            "x = 1  # noqa: E501 sdd: permanently justified\n",
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");

        let out = run_on("local.rs", "#[allow(dead_code)] // KI-vendor-quirkXYZ\n");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    #[test]
    fn a_case_and_a_marker_together_are_rejected() {
        let out = run_on(
            "local.rs",
            "#[allow(dead_code)] // KI-vendor-quirk sdd: permanent both\n",
        );
        assert_eq!(out.len(), 1);
        assert!(out[0].ends_with(": names a case and states a permanent exception"));
    }

    #[test]
    fn a_case_resolving_to_no_record_is_rejected() {
        let out = run_on(
            "local.md",
            "<!-- markdownlint-disable KI-absent-record -->\n",
        );
        assert_eq!(
            out,
            vec![
                "FAIL spec-to-code:a-suppression-names-its-case: KI-absent-record resolves to no record"
                    .to_string()
            ]
        );
    }

    #[test]
    fn a_form_named_outside_its_file_kind_is_a_quotation() {
        for (name, text) in [
            (
                "prose.md",
                "The `#[allow(dead_code)]` attribute suppresses a lint.\n",
            ),
            (
                "prose.md",
                "A Python file carries `# noqa: E501` at the line.\n",
            ),
            (
                "local.rs",
                "let form = \"<!-- markdownlint-disable -->\";\n",
            ),
            ("local.rs", "let form = \"# shellcheck disable=SC2329\";\n"),
            ("local.rs", "let form = \"// eslint-disable-next-line\";\n"),
        ] {
            assert!(run_on(name, text).is_empty(), "{name}: {text}");
        }
    }

    #[test]
    fn closing_markers_are_not_suppressions() {
        let text = "<!-- dprint-ignore-end -->\n<!-- markdownlint-enable -->\n";
        assert!(run_on("local.md", text).is_empty());
    }

    #[test]
    fn a_vendored_tree_is_skipped() {
        let dir = fixture();
        let vendored = dir.path().join("third-party/upstream");
        std::fs::create_dir_all(&vendored).unwrap();
        std::fs::write(vendored.join("hook.py"), "x = 1  # noqa: E501\n").unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        assert!(run(&ctx, &[]).unwrap().is_empty());
    }
}
