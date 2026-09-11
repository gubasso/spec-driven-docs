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
//! A suppression states that reason in either of two places. Where the tool
//! that honors the form defines a reason position of its own, the reason
//! written there counts, so a generated artifact another project owns
//! satisfies this rule in its own idiom and nothing here reaches into bytes
//! it does not author. Where the tool defines no such position, the
//! `sdd: permanent` marker is the portable fallback. Only a position the
//! tool formally defines counts, so prose that merely sits near a
//! suppression never satisfies the rule.
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
//! A file with no filename suffix takes its form family from its shebang, so
//! a script such as `scripts/publish` is judged as the language its
//! interpreter names. A shebang that hides its command behind an escape in an
//! `env -S` string names no language here, and that file keeps the name it
//! has.
//!
//! Three surfaces stay outside this scan: a file with no suffix and no
//! shebang, such as the `justfile` this repository carries, whose recipes run
//! under a shell and can hold a suppression no line declares; a block comment
//! holding a suppression; and the `[lints]` table of a manifest.

use std::collections::BTreeSet;

use camino::Utf8Path;

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

/// Where a form's own tool reads the reason the author wrote for it.
///
/// Only a position the tool formally defines counts. Prose that merely sits
/// near a suppression is not a reason channel: accepting it would let an
/// unrelated comment satisfy the rule.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Channel {
    /// The tool defines no reason position, so the marker is the only form.
    None,
    /// The text after the directive's closing bracket, as zizmor reads it.
    AfterBracket,
    /// The text after a `--` separator, as `ESLint` reads it.
    AfterSeparator,
    /// The string of a `reason` argument, as Rust and pytest read it.
    ReasonArgument,
    /// The string of a bare `=` value, as `#[ignore]` reads it.
    ValueString,
    /// The first argument, as `unittest.skip` reads its reason.
    FirstArgument,
    /// The second argument, as `unittest.skipIf` reads its reason past the
    /// condition.
    SecondArgument,
}

/// One suppression form: the text that opens it and the reason channel its
/// own tool defines.
struct Form {
    token: &'static str,
    channel: Channel,
}

const fn form(token: &'static str, channel: Channel) -> Form {
    Form { token, channel }
}

/// One family of suppression forms, with the file suffixes it is live in.
struct Family {
    suffixes: &'static [&'static str],
    opener: Opener,
    forms: &'static [Form],
}

const FAMILIES: &[Family] = &[
    Family {
        suffixes: &[".rs"],
        opener: Opener::Line,
        forms: &[
            form("#[allow(", Channel::ReasonArgument),
            form("#[expect(", Channel::ReasonArgument),
            form("#![allow(", Channel::ReasonArgument),
            form("#![expect(", Channel::ReasonArgument),
            form("#[ignore", Channel::ValueString),
        ],
    },
    // A longer token is listed ahead of the prefix it extends, because the
    // first match wins and `@unittest.skipIf` starts with `@unittest.skip`.
    Family {
        suffixes: &[".py"],
        opener: Opener::Line,
        forms: &[
            form("@pytest.mark.xfail", Channel::ReasonArgument),
            form("@pytest.mark.skip", Channel::ReasonArgument),
            form("@unittest.skipIf", Channel::SecondArgument),
            form("@unittest.skipUnless", Channel::SecondArgument),
            form("@unittest.skip", Channel::FirstArgument),
            form("@unittest.expectedFailure", Channel::None),
        ],
    },
    Family {
        suffixes: &[".py", ".sh", ".bash", ".yaml", ".yml", ".toml"],
        opener: Opener::After("#"),
        forms: &[
            form("shellcheck disable=", Channel::None),
            form("noqa", Channel::None),
            form("ruff: noqa", Channel::None),
            form("flake8: noqa", Channel::None),
            form("type: ignore", Channel::None),
            form("zizmor: ignore[", Channel::AfterBracket),
        ],
    },
    Family {
        suffixes: &[".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs"],
        opener: Opener::After("//"),
        forms: &[form("eslint-disable", Channel::AfterSeparator)],
    },
    Family {
        suffixes: &[".md", ".html"],
        opener: Opener::After("<!--"),
        forms: &[
            form("dprint-ignore", Channel::None),
            form("markdownlint-disable", Channel::None),
        ],
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

/// The name the form families are looked up by.
///
/// A file with no suffix states its language in its shebang, so the lookup
/// borrows the suffix that interpreter implies. The real path stays the one a
/// finding prints.
fn kind_name(name: &str, first: &str) -> String {
    // The basename comes from the path API, so the separator this platform
    // writes is the one that splits it. A dot in a directory is not a suffix
    // on the file.
    let stem = Utf8Path::new(name).file_name().unwrap_or(name);
    if stem.contains('.') {
        return name.to_string();
    }
    let Some(rest) = first.strip_prefix("#!") else {
        return name.to_string();
    };
    match interpreter(rest) {
        Some("sh" | "bash" | "dash" | "ksh" | "zsh") => format!("{name}.sh"),
        Some("python" | "python3") => format!("{name}.py"),
        _ => name.to_string(),
    }
}

/// The interpreter a shebang names, without its directory.
///
/// The kernel reads the first word as the interpreter and passes everything
/// after it as one argument, so an optional argument is never the interpreter
/// and `#!/bin/sh -e` is a shell script. `env` is the one exception: it runs
/// the first of its own operands that is neither an option, an option's own
/// argument, nor an assignment, so `#!/usr/bin/env -S python3 -X dev` is a
/// Python script.
///
/// `env -S` reads quotes when it splits, and this scan reads a quoted run and
/// nothing else of that grammar. An escape inside such a string is read as
/// the letters that spell it, so a command hidden behind one keeps the name
/// its file already has and stays outside the scan.
///
/// The operands are read whether or not `-S` is present. Linux passes the
/// whole tail to `env` as one argument and Darwin splits it, so a multiword
/// line runs on one platform and not the other. Reading it either way names
/// the language the author wrote, and naming it is what puts the file inside
/// the scan. The other reading would leave a real script on a real platform
/// unjudged, which is the gap this scan exists to close.
fn interpreter(rest: &str) -> Option<&str> {
    /// Every `env` option that takes its argument as a separate word. `-S`
    /// is not one: it splits the rest of the line, and `env` reads options
    /// again inside what it split, so the command still follows it.
    const TAKES_A_WORD: &[&str] = &["-u", "--unset", "-C", "--chdir", "-a", "--argv0"];
    fn basename(word: &str) -> &str {
        word.rsplit('/').next().unwrap_or(word)
    }
    /// Consume one option's argument, which is a quoted run where it opens
    /// with a quote. Without this, the tail of `-C "/tmp dir"` reads as the
    /// command.
    fn take_argument<'a>(words: &mut impl Iterator<Item = &'a str>) {
        let Some(word) = words.next() else { return };
        let Some(quote) = word.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            return;
        };
        if word.len() > 1 && word.ends_with(quote) {
            return;
        }
        words
            .take_while(|word| !word.ends_with(quote))
            .for_each(drop);
    }
    let mut words = rest.split_whitespace();
    let first = basename(words.next()?);
    if first != "env" {
        return Some(first);
    }
    while let Some(word) = words.next() {
        if TAKES_A_WORD.contains(&word) {
            take_argument(&mut words);
        } else if !word.starts_with('-') && !word.contains('=') {
            return Some(basename(word.trim_matches(['"', '\''])));
        }
    }
    None
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
///
/// The match ignores case, because a linter that honors `noqa` honors
/// `NOQA` too.
fn comment_carries(comment: &str, opener: &str, token: &str) -> bool {
    let lowered = comment.to_ascii_lowercase();
    let mut start = 0usize;
    while let Some(offset) = lowered[start..].find(opener) {
        let index = start + offset;
        if lowered[index + opener.len()..]
            .trim_start_matches(' ')
            .starts_with(token)
        {
            return true;
        }
        start = index + opener.len();
    }
    false
}

/// Where a suppression on this line starts carrying its annotation, and the
/// reason channel the form's own tool defines.
///
/// A comment-borne form annotates from where the comment opens, so a case
/// id or a marker written in code earlier on the line is not the
/// suppression's. A line-borne form carries its annotation on the whole
/// line: a Rust attribute and a Python decorator both hold their reason
/// inside themselves.
fn suppression_at(file: &str, line: &str) -> Option<(usize, &'static Form)> {
    for family in FAMILIES {
        if !family.suffixes.iter().any(|suffix| file.ends_with(suffix)) {
            continue;
        }
        match family.opener {
            Opener::Line => {
                let code = line.trim_start();
                if let Some(form) = family
                    .forms
                    .iter()
                    .find(|form| code.starts_with(form.token))
                {
                    return Some((0, form));
                }
            }
            Opener::After(opener) => {
                let Some(index) = comment_start(line, opener, quote_marks(file)) else {
                    continue;
                };
                let comment = &line[index..];
                if let Some(form) = family
                    .forms
                    .iter()
                    .find(|form| comment_carries(comment, opener, form.token))
                {
                    return Some((index, form));
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

/// Where `fence` first appears unescaped. An escaped delimiter is part of
/// the string it sits in, so it closes nothing.
fn find_unescaped(line: &str, fence: &str) -> Option<usize> {
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if line[index..].starts_with(fence) {
            return Some(index);
        }
    }
    None
}

/// Where each line's live code begins, or `None` where the whole line sits
/// inside a multi-line string of the file's own language.
///
/// A line that opens inside such a string is content up to its closing
/// delimiter and live code after it, which is where a linter asks for the
/// suppression a long string earns.
fn live_from(file: &str, lines: &[&str]) -> Vec<Option<usize>> {
    let fences: &[&str] = if is_python(file) {
        &["\"\"\"", "'''"]
    } else if comment_opener(file) == Some("//") && !is_rust(file) {
        &["`"]
    } else {
        return vec![Some(0); lines.len()];
    };
    let comment = comment_opener(file);
    let mut open: Option<&str> = None;
    lines
        .iter()
        .map(|line| {
            let Some(fence) = open else {
                for opened in fence_toggles(line, fences, comment) {
                    open = match open {
                        None => Some(opened),
                        Some(current) if current == opened => None,
                        Some(current) => Some(current),
                    };
                }
                return Some(0);
            };
            let closes = find_unescaped(line, fence);
            if closes.is_some() {
                open = None;
            }
            closes.map(|index| index + fence.len())
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

/// How many bytes of `text` at `index` open a raw string, and how many
/// hashes close it. `None` where no raw string opens there.
///
/// A hashed raw string ends only at a quote carrying its own hash count, so
/// a quote inside the body is content.
fn raw_opener(text: &str, index: usize) -> Option<(usize, usize)> {
    let after = text[index..].strip_prefix('r')?;
    let hashes = after.chars().take_while(|c| *c == '#').count();
    after[hashes..].starts_with('"').then_some((
        // `r`, the hashes, and the opening quote.
        1 + hashes + 1,
        hashes,
    ))
}

/// One past the end of the literal that opens at `index`, or `None` where
/// no literal opens there.
///
/// One reader owns literal boundaries, so every scan below agrees on which
/// text is content. A literal that never closes runs to the end, which
/// keeps an unbalanced line from reading as no literal at all.
fn literal_end(text: &str, index: usize) -> Option<usize> {
    if let Some((opener, hashes)) = raw_opener(text, index) {
        let body = index + opener;
        let close = format!("\"{}", "#".repeat(hashes));
        return Some(
            text[body..]
                .find(&close)
                .map_or(text.len(), |at| body + at + close.len()),
        );
    }
    let quote = text[index..]
        .chars()
        .next()
        .filter(|c| *c == '"' || *c == '\'')?;
    let body = index + quote.len_utf8();
    let mut escaped = false;
    for (at, character) in text[body..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            c if c == quote => return Some(body + at + c.len_utf8()),
            _ => {}
        }
    }
    Some(text.len())
}

/// The character a numeric code point names, written in the given base.
fn from_code(digits: &str, base: u32) -> Option<char> {
    u32::from_str_radix(digits, base)
        .ok()
        .and_then(char::from_u32)
}

/// The digits an escape carries, up to `width` of them in the given base.
fn code_digits(body: &mut std::str::Chars, first: Option<char>, width: usize, base: u32) -> String {
    let mut digits: String = first.into_iter().collect();
    while digits.len() < width {
        match body.clone().next().filter(|c| c.is_digit(base)) {
            Some(next) => {
                body.next();
                digits.push(next);
            }
            None => break,
        }
    }
    digits
}

/// Append what one escape sequence stands for, having already read its
/// backslash.
///
/// The represented character is what the reason says, so `\t`, `\x20`, and
/// `\040` state the whitespace they are rather than the letters that spell
/// them. A reason that reads as whitespace states nothing whichever way it
/// is written.
///
/// Python's `\N{name}` names a character this gate cannot resolve without a
/// Unicode name table, so it contributes nothing rather than its own
/// letters. A reason spelled only in named escapes therefore reads as
/// empty, which fails, and a named escape beside real prose leaves that
/// prose to speak.
fn push_escaped(out: &mut String, body: &mut std::str::Chars) {
    let Some(character) = body.next() else { return };
    match character {
        'n' => out.push('\n'),
        'r' => out.push('\r'),
        't' => out.push('\t'),
        'f' => out.push('\u{0c}'),
        'v' => out.push('\u{0b}'),
        'a' => out.push('\u{07}'),
        'b' => out.push('\u{08}'),
        // Python spells an octal escape with up to three digits, and `\0`
        // is the shortest of them.
        digit @ '0'..='7' => out.extend(from_code(&code_digits(body, Some(digit), 3, 8), 8)),
        'x' => out.extend(from_code(&code_digits(body, None, 2, 16), 16)),
        // A named escape resolves to a character this gate cannot name, so
        // it is consumed and contributes nothing.
        'N' if body.clone().next() == Some('{') => {
            body.take_while(|c| *c != '}').for_each(drop);
        }
        // Rust brackets the code point and Python takes a fixed width.
        'u' | 'U' => {
            let digits = if body.clone().next() == Some('{') {
                body.next();
                body.take_while(|c| *c != '}').collect()
            } else {
                code_digits(body, None, if character == 'u' { 4 } else { 8 }, 16)
            };
            out.extend(from_code(&digits, 16));
        }
        other => out.push(other),
    }
}

/// The text of a quoted string starting at `from`, or `None` where no
/// string opens there. The scan stops at the closing quote, so a reason
/// carries its own text and nothing after it.
///
/// A raw string is one of the spellings Rust's attribute grammar accepts,
/// so `r"..."` and `r#"..."#` read the same as an ordinary string.
fn quoted_from(text: &str, from: usize) -> Option<String> {
    let start = from + text[from..].len() - text[from..].trim_start().len();
    let rest = &text[start..];
    if let Some((opener, hashes)) = raw_opener(rest, 0) {
        let close = format!("\"{}", "#".repeat(hashes));
        return rest[opener..]
            .find(&close)
            .map(|at| rest[opener..opener + at].to_string());
    }
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let mut body = rest[quote.len_utf8()..].chars();
    let mut out = String::new();
    while let Some(character) = body.next() {
        match character {
            '\\' => push_escaped(&mut out, &mut body),
            c if c == quote => return Some(out),
            c => out.push(c),
        }
    }
    None
}

/// The suppression's own extent, from its token to the delimiter that
/// closes it.
///
/// Everything past that delimiter belongs to whatever else the line
/// carries. Without the bound, a second attribute on the same line lends
/// its text to a suppression that states nothing, which is the silent mask
/// this gate exists to prevent.
fn extent_from<'a>(code: &'a str, token: &str) -> Option<&'a str> {
    let start = code.find(token)?;
    let text = &code[start..];
    let mut depth = 0i32;
    let mut opened = false;
    let mut index = 0usize;
    while let Some(character) = text[index..].chars().next() {
        if let Some(end) = literal_end(text, index) {
            index = end;
            continue;
        }
        match character {
            '(' | '[' => {
                depth += 1;
                opened = true;
            }
            ')' | ']' => {
                depth -= 1;
                if opened && depth <= 0 {
                    return Some(&code[start..start + index + character.len_utf8()]);
                }
            }
            _ => {}
        }
        index += character.len_utf8();
    }
    // A suppression whose delimiters never close owns the rest of the text,
    // so an unbalanced line still reads as the suppression it is.
    opened.then_some(&code[start..])
}

/// Every top-level argument of the call the extent opens, in order.
fn arguments(extent: &str) -> Vec<&str> {
    let Some(open) = extent.find('(') else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut from = open + 1;
    let mut at = open + 1;
    while let Some(character) = extent[at..].chars().next() {
        if let Some(end) = literal_end(extent, at) {
            at = end;
            continue;
        }
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth > 0 => depth -= 1,
            ')' => {
                out.push(&extent[from..at]);
                return out;
            }
            ',' if depth == 0 => {
                out.push(&extent[from..at]);
                from = at + 1;
            }
            _ => {}
        }
        at += character.len_utf8();
    }
    out
}

/// The text a `reason` argument carries, in the `reason = "..."` Rust
/// writes and the `reason="..."` pytest writes.
///
/// The name counts outside every quoted span, so the same word inside
/// another argument's string is that string's text and not a reason.
fn keyword_value(argument: &str) -> Option<String> {
    let after = argument.trim_start().strip_prefix("reason")?;
    let equals = after
        .find('=')
        .filter(|at| after[..*at].trim().is_empty())?;
    // A comparison binds nothing, and a longer name is a different name.
    if after[equals + 1..].starts_with('=') {
        return None;
    }
    quoted_from(after, equals + 1)
}

/// The reason the suppression's own `reason` argument carries.
///
/// The name is read among the suppression's top-level arguments alone, so a
/// nested call cannot lend its own reason to a suppression that states
/// none.
fn reason_argument(extent: &str) -> Option<String> {
    arguments(extent)
        .iter()
        .find_map(|argument| keyword_value(argument))
}

/// The reason a call carries in the parameter named `reason`, or in the
/// slot that parameter occupies when the caller passes it positionally.
fn positional_reason(extent: &str, slot: usize) -> Option<String> {
    reason_argument(extent).or_else(|| quoted_from(arguments(extent).get(slot)?, 0))
}

/// The reason the form's own tool reads, or `None` where the tool defines
/// no reason position or the author wrote none.
///
/// The reason is read from the suppression's own lines alone. A comment
/// above the suppression is prose the tool never reads, so it satisfies
/// nothing here. A channel the tool spells in code reads those lines' code
/// alone, bounded to the suppression's own extent, so neither a trailing
/// comment nor a second attribute beside it lends a reason.
fn native_reason(file: &str, channel: Channel, token: &str, region: &[&str]) -> Option<String> {
    fn trailing(text: &str) -> String {
        text.trim_end()
            .trim_end_matches("-->")
            .trim_end_matches("*/")
            .trim()
            .to_string()
    }
    match channel {
        Channel::None => None,
        Channel::AfterBracket => {
            let line = region.first()?;
            let at = line.find(token)?;
            let close = line[at..].find(']')?;
            Some(trailing(&line[at + close + 1..]))
        }
        // ESLint separates the rule list from the description with a `--`
        // that stands alone, so a hyphenated rule name is not a separator.
        Channel::AfterSeparator => {
            let line = region.first()?;
            let at = line.find(token)?;
            let separator = line[at..].find(" -- ")?;
            Some(trailing(&line[at + separator + 4..]))
        }
        Channel::ReasonArgument
        | Channel::ValueString
        | Channel::FirstArgument
        | Channel::SecondArgument => {
            let code: Vec<&str> = region.iter().map(|line| code_region(file, line)).collect();
            let joined = code.join("\n");
            let extent = extent_from(&joined, token)?;
            match channel {
                Channel::ReasonArgument => reason_argument(extent),
                Channel::ValueString => {
                    let equals = extent.find('=')?;
                    quoted_from(extent, equals + 1)
                }
                // Python binds a keyword argument to its parameter wherever
                // the caller writes it, so the name wins over the slot.
                Channel::FirstArgument => positional_reason(extent, 0),
                _ => positional_reason(extent, 1),
            }
        }
    }
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(4096).any(|&b| b == 0)
}

/// What a suppression declares about itself.
///
/// The disposition carries no reason text, because no caller reads one: the
/// gate judges that a reason exists and leaves whether it is truthful to
/// review, exactly as it does for the marker form.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Disposition {
    /// Neither a case nor a reason, so the suppression becomes permanent by
    /// default.
    Missing,
    /// A `KI-<slug>` case, which a record must define.
    KnownIssue,
    /// A permanent exception, stated in the tool's own reason channel or in
    /// the portable marker.
    Accepted,
    /// A case and an explicit permanent marker, which are exclusive.
    Conflict,
    /// The marker with nothing after it.
    MarkerWithoutReason,
}

/// One suppression, with the lines a reader can read its reason from.
struct Site {
    file: String,
    /// The name the file's own syntax is read by, which differs from `file`
    /// only where a shebang supplies the suffix the name lacks.
    kind: String,
    number: usize,
    line: String,
    annotation: Vec<String>,
    /// Where the suppression's own lines start in `annotation`, past the
    /// comment line above it.
    region_from: usize,
    form: &'static Form,
}

impl Site {
    fn cites_a_case(&self) -> bool {
        self.annotation
            .iter()
            .any(|line| cited_cases(line).next().is_some())
    }

    /// The reason the portable marker states, wherever a reader can see it.
    fn marker_reason(&self) -> Option<String> {
        self.annotation
            .iter()
            .find_map(|line| permanent_reason(line))
    }

    /// The reason the form's own tool carries, read from the suppression's
    /// own lines. Whitespace states nothing, so it is no reason.
    fn native_reason(&self) -> Option<String> {
        let region: Vec<&str> = self.annotation[self.region_from..]
            .iter()
            .map(String::as_str)
            .collect();
        native_reason(&self.kind, self.form.channel, self.form.token, &region)
            .filter(|reason| !reason.trim().is_empty())
    }

    /// The marker is an explicit declaration, so it decides on its own
    /// wherever the author wrote one. The native channel decides only where
    /// no marker and no case is present.
    fn disposition(&self) -> Disposition {
        match (self.cites_a_case(), self.marker_reason()) {
            (true, Some(_)) => Disposition::Conflict,
            (true, None) => Disposition::KnownIssue,
            (false, Some(reason)) if reason.is_empty() => Disposition::MarkerWithoutReason,
            (false, Some(_)) => Disposition::Accepted,
            (false, None) if self.native_reason().is_some() => Disposition::Accepted,
            (false, None) => Disposition::Missing,
        }
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
///
/// The second return names where the suppression's own lines start, so a
/// channel the tool defines is read from those lines and never from the
/// comment above them.
fn annotation(file: &str, lines: &[&str], index: usize, start: usize) -> (Vec<String>, usize) {
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
    let region_from = out.len();
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
    (out, region_from)
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
        // A file with no suffix names its language in its shebang, and the
        // lookup reads that name. The real path stays in `Site.file`, so a
        // finding prints a path a reader can open.
        let kind = kind_name(&name, lines.first().copied().unwrap_or_default());
        let live = live_from(&kind, &lines);
        for (index, line) in lines.iter().enumerate() {
            if matches!(
                kinds.get(index),
                Some(LineKind::Fence | LineKind::FrontMatter)
            ) {
                continue;
            }
            let Some(offset) = live[index] else { continue };
            let Some((found, form)) = suppression_at(&kind, &line[offset..]) else {
                continue;
            };
            let start = offset + found;
            if !is_closing(line) {
                let (annotation, region_from) = annotation(&kind, &lines, index, start);
                sites.push(Site {
                    file: name.clone(),
                    kind: kind.clone(),
                    number: index + 1,
                    line: (*line).to_string(),
                    annotation,
                    region_from,
                    form,
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
        match site.disposition() {
            Disposition::KnownIssue | Disposition::Accepted => {}
            Disposition::MarkerWithoutReason => {
                violations.push(Violation::Finding(Finding::on_line(
                    PERMANENT,
                    &site.file,
                    site.number,
                    "the permanent marker states no reason",
                )));
            }
            Disposition::Conflict => violations.push(Violation::Finding(Finding::on_line(
                PERMANENT,
                &site.file,
                site.number,
                "names a case and states a permanent exception",
            ))),
            Disposition::Missing => caseless.push(site),
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
    fn a_tools_own_reason_states_a_permanent_exception() {
        for (name, text) in [
            (
                "local.yml",
                "on: push  # zizmor: ignore[dangerous-triggers] the definition is the trusted one\n",
            ),
            (
                "local.rs",
                "#[allow(dead_code, reason = \"the field is the wire format\")]\n",
            ),
            (
                "local.rs",
                "#[expect(dead_code, reason = \"the field is the wire format\")]\n",
            ),
            (
                "local.rs",
                "#[ignore = \"the fixture needs a live network\"]\n",
            ),
            (
                "local.py",
                "@pytest.mark.xfail(reason=\"the parser rejects a valid literal\", strict=True)\ndef test_x():\n    pass\n",
            ),
            (
                "local.ts",
                "// eslint-disable-next-line no-eval -- the input is a literal in this file\n",
            ),
        ] {
            assert!(run_on(name, text).is_empty(), "{name}: {text}");
        }
    }

    /// The shape a generated workflow carries, which this project does not
    /// author and must not edit.
    #[test]
    fn a_generated_workflow_states_its_reason_in_its_own_idiom() {
        let text = concat!(
            "on:\n",
            "  # zizmor: ignore[dangerous-triggers] the trigger is what makes this gate\n",
            "  # unforgeable, and the header above states why it is safe here.\n",
            "  pull_request_target:\n",
        );
        assert!(run_on("local.yml", text).is_empty());
    }

    #[test]
    fn a_tools_own_reason_left_empty_is_no_reason() {
        for (name, text) in [
            (
                "local.yml",
                "on: push  # zizmor: ignore[dangerous-triggers]\n",
            ),
            ("local.rs", "#[allow(dead_code, reason = \"\")]\n"),
            ("local.rs", "#[ignore]\n"),
            ("local.ts", "// eslint-disable-next-line no-eval --\n"),
        ] {
            let out = run_on(name, text);
            assert_eq!(out.len(), 2, "{name}: {text}");
            assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
        }
    }

    #[test]
    fn a_tool_that_defines_no_reason_position_still_takes_the_marker() {
        for (name, text) in [
            ("local.py", "x = 1  # noqa: E501 the line is one URL\n"),
            (
                "local.sh",
                "# shellcheck disable=SC2329 reached through a trap\n",
            ),
            (
                "local.md",
                "<!-- markdownlint-disable MD013 the table is data -->\n",
            ),
        ] {
            let out = run_on(name, text);
            assert_eq!(out.len(), 2, "{name}: {text}");
            assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
        }
    }

    #[test]
    fn prose_above_a_suppression_is_not_the_tools_own_reason() {
        for (name, text) in [
            (
                "local.rs",
                "// reason = \"this comment is not the attribute\"\n#[allow(dead_code)]\n",
            ),
            (
                "local.rs",
                "#[allow(dead_code)] // reason = \"this comment is not the attribute\"\n",
            ),
        ] {
            let out = run_on(name, text);
            assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
        }
    }

    /// A reason belongs to the suppression that carries it, and to no
    /// other construct sharing the line.
    #[test]
    fn a_reason_beside_a_suppression_is_not_the_suppressions() {
        for (name, text) in [
            (
                "local.rs",
                "#[allow(dead_code)] #[doc = \"reason = 'unrelated prose'\"] fn f() {}\n",
            ),
            (
                "local.rs",
                "#[allow(dead_code)] #[expect(unused, reason = \"the other one states it\")]\n",
            ),
        ] {
            let out = run_on(name, text);
            assert_eq!(
                out[0], "FAIL spec-to-code:a-suppression-names-its-case",
                "{name}: {text}"
            );
        }
    }

    #[test]
    fn a_whitespace_reason_states_nothing() {
        for (name, text) in [
            ("local.rs", "#[allow(dead_code, reason = \"   \")]\n"),
            ("local.rs", "#[ignore = \"  \"]\n"),
            (
                "local.py",
                "@pytest.mark.skip(reason=\"  \")\ndef test_x():\n    pass\n",
            ),
            (
                "local.py",
                "@unittest.skip(\"  \")\ndef test_x():\n    pass\n",
            ),
        ] {
            let out = run_on(name, text);
            assert_eq!(
                out[0], "FAIL spec-to-code:a-suppression-names-its-case",
                "{name}: {text}"
            );
        }
    }

    #[test]
    fn a_raw_string_reason_is_a_reason() {
        for text in [
            "#[ignore = r\"requires a live service\"]\n",
            "#[ignore = r#\"requires a \"live\" service\"#]\n",
            "#[allow(dead_code, reason = r\"the field is the wire format\")]\n",
        ] {
            assert!(run_on("local.rs", text).is_empty(), "{text}");
        }
    }

    #[test]
    fn a_positional_skip_reason_is_a_reason() {
        for text in [
            "@unittest.skip(\"the service is unavailable\")\ndef test_x():\n    pass\n",
            "@unittest.skipIf(sys.platform == \"win32\", \"the path is posix only\")\ndef test_x():\n    pass\n",
            "@unittest.skipUnless(os.name == \"posix\", \"the path is posix only\")\ndef test_x():\n    pass\n",
        ] {
            assert!(run_on("local.py", text).is_empty(), "{text}");
        }
    }

    /// The condition is not the reason, so a skip that states only a
    /// condition still says nothing.
    #[test]
    fn a_skip_condition_is_not_its_reason() {
        let out = run_on(
            "local.py",
            "@unittest.skipIf(sys.platform == \"win32\")\ndef test_x():\n    pass\n",
        );
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    /// Python binds a keyword argument to its parameter wherever the caller
    /// writes it, so the name states the reason from any slot.
    #[test]
    fn a_keyword_skip_reason_is_a_reason() {
        for text in [
            "@unittest.skip(reason=\"the service is unavailable\")\ndef test_x():\n    pass\n",
            "@unittest.skipIf(condition=True, reason=\"the path is posix only\")\ndef test_x():\n    pass\n",
            "@unittest.skipUnless(reason=\"the path is posix only\", condition=True)\ndef test_x():\n    pass\n",
        ] {
            assert!(run_on("local.py", text).is_empty(), "{text}");
        }
    }

    /// An escape states the character it stands for, so a reason spelled
    /// only in whitespace escapes states nothing.
    #[test]
    fn an_escaped_whitespace_reason_states_nothing() {
        for text in [
            "#[allow(dead_code, reason = \"\\t\")]\n",
            "#[allow(dead_code, reason = \"\\n\\r\")]\n",
        ] {
            let out = run_on("local.rs", text);
            assert_eq!(
                out[0], "FAIL spec-to-code:a-suppression-names-its-case",
                "{text}"
            );
        }
    }

    /// A hashed raw string ends only at a quote carrying its own hashes, so
    /// a delimiter inside the body is the reason's own text.
    #[test]
    fn a_raw_reason_carrying_a_delimiter_is_still_one_literal() {
        let text = "#[allow(dead_code, reason = r#\"the token \")]\" is data\"#)]\n";
        assert!(run_on("local.rs", text).is_empty());
    }

    /// A nested call states its own reason, never its caller's.
    #[test]
    fn a_nested_call_does_not_lend_its_reason() {
        for (name, text) in [
            (
                "local.py",
                "@unittest.skipIf(condition=check(reason=\"borrowed\"), reason=\"\")\ndef test_x():\n    pass\n",
            ),
            (
                "local.py",
                "@pytest.mark.skip(reason=compute(reason=\"borrowed\"))\ndef test_x():\n    pass\n",
            ),
        ] {
            let out = run_on(name, text);
            assert_eq!(
                out[0], "FAIL spec-to-code:a-suppression-names-its-case",
                "{name}: {text}"
            );
        }
    }

    /// An escape states the character it names, whatever its spelling.
    #[test]
    fn a_numeric_whitespace_escape_states_nothing() {
        for text in [
            "#[allow(dead_code, reason = \"\\x20\")]\n",
            "#[allow(dead_code, reason = \"\\u{20}\")]\n",
            "#[allow(dead_code, reason = \"\\u{20}\\t\\x20\")]\n",
        ] {
            let out = run_on("local.rs", text);
            assert_eq!(
                out[0], "FAIL spec-to-code:a-suppression-names-its-case",
                "{text}"
            );
        }
    }

    /// Every spelling both languages define reads as the character it
    /// names, so no whitespace escape closes the rule.
    #[test]
    fn every_whitespace_escape_states_nothing() {
        for (name, text) in [
            (
                "local.py",
                "@unittest.skip(\"\\v\")\ndef test_x():\n    pass\n",
            ),
            (
                "local.py",
                "@unittest.skip(\"\\f\")\ndef test_x():\n    pass\n",
            ),
            (
                "local.py",
                "@unittest.skip(\"\\040\")\ndef test_x():\n    pass\n",
            ),
            (
                "local.py",
                "@unittest.skip(\"\\u0020\")\ndef test_x():\n    pass\n",
            ),
            (
                "local.py",
                "@unittest.skip(\"\\N{SPACE}\")\ndef test_x():\n    pass\n",
            ),
        ] {
            let out = run_on(name, text);
            assert_eq!(
                out[0], "FAIL spec-to-code:a-suppression-names-its-case",
                "{name}: {text}"
            );
        }
    }

    /// The decoder states the character, so a reason that is not whitespace
    /// still reads as one.
    #[test]
    fn a_numeric_escape_inside_a_reason_keeps_it() {
        let text = "#[allow(dead_code, reason = \"the\\x20field is the wire format\")]\n";
        assert!(run_on("local.rs", text).is_empty());
        let named =
            "@unittest.skip(\"\\N{BULLET} the service is unavailable\")\ndef test_x():\n    pass\n";
        assert!(run_on("local.py", named).is_empty());
    }

    /// A name that merely begins with `reason` is a different name.
    #[test]
    fn a_longer_name_is_not_the_reason_parameter() {
        let out = run_on(
            "local.py",
            "@unittest.skipIf(reason_code == \"x\", 12)\ndef test_x():\n    pass\n",
        );
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    #[test]
    fn a_case_inside_a_tools_own_reason_stays_a_known_issue() {
        assert!(
            run_on(
                "local.rs",
                "#[allow(dead_code, reason = \"KI-vendor-quirk\")]\n"
            )
            .is_empty()
        );
        let out = run_on("local.rs", "#[allow(dead_code, reason = \"KI-absent\")]\n");
        assert_eq!(
            out[0],
            "FAIL spec-to-code:a-suppression-names-its-case: KI-absent resolves to no record"
        );
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
    fn a_linters_other_spellings_are_the_same_form() {
        for text in [
            "value = 1  # NOQA: E501\n",
            "# ruff: noqa\n",
            "# flake8: noqa\n",
        ] {
            let out = run_on("local.py", text);
            assert_eq!(out.len(), 2, "{text}");
            assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
        }
    }

    #[test]
    fn the_line_a_multiline_string_closes_on_is_still_content() {
        for (name, text) in [
            ("local.py", "DOC = \"\"\"\n# noqa: E501 \"\"\"\n"),
            ("local.ts", "const doc = `\n// eslint-disable-next-line`;\n"),
        ] {
            assert!(run_on(name, text).is_empty(), "{name}: {text}");
        }
    }

    #[test]
    fn an_escaped_delimiter_closes_no_multiline_string() {
        let text = "const t = `\nconst label = \\`value\\`;\n// eslint-disable-next-line\n`;\n";
        assert!(run_on("local.ts", text).is_empty());
    }

    #[test]
    fn a_suppression_after_a_closing_delimiter_is_live() {
        let out = run_on(
            "local.py",
            "DOC = \"\"\"\nlong text\n\"\"\"  # noqa: E501\n",
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
        assert!(out[1].contains("local.py:3"));
        let text = "DOC = \"\"\"\nlong text\n\"\"\"  # noqa: E501 KI-vendor-quirk\n";
        assert!(run_on("local.py", text).is_empty());
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

    /// A script with no suffix states its language in its shebang, so the
    /// forms that language carries are live in it.
    #[test]
    fn a_shebang_makes_a_suffixless_script_readable() {
        let out = run_on("publish", "#!/bin/bash\n# shellcheck disable=SC2086\n");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
        assert!(out[1].contains("publish:2"));
    }

    #[test]
    fn a_suffixless_script_takes_the_marker_and_the_case() {
        for text in [
            "#!/bin/bash\n# shellcheck disable=SC2086  # sdd: permanent the word split is wanted\n",
            "#!/bin/bash\n# shellcheck disable=SC2086  # KI-vendor-quirk\n",
        ] {
            assert!(run_on("publish", text).is_empty(), "{text}");
        }
    }

    /// The lookup name is not the reporting path, so a finding names the file
    /// a reader can open.
    #[test]
    fn a_finding_names_the_real_path_and_not_the_borrowed_suffix() {
        let out = run_on("publish", "#!/bin/bash\n# shellcheck disable=SC2086\n");
        assert!(out[1].contains("./publish:2"), "{}", out[1]);
        assert!(!out[1].contains("publish.sh"), "{}", out[1]);
    }

    #[test]
    fn an_env_shebang_reads_the_same_as_a_direct_one() {
        let out = run_on(
            "publish",
            "#!/usr/bin/env bash\n# shellcheck disable=SC2086\n",
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
    }

    /// The remaining gap, stated as behavior: a file with no suffix and no
    /// shebang names no language, so no form is live in it.
    #[test]
    fn a_file_without_a_suffix_and_without_a_shebang_is_skipped() {
        assert!(run_on("justfile", "check:\n    # shellcheck disable=SC2086\n").is_empty());
    }

    /// The interpreter list is closed, so a language this gate carries no
    /// family for stays outside the scan.
    #[test]
    fn an_unlisted_interpreter_names_no_family() {
        assert!(run_on("run", "#!/usr/bin/perl\n# noqa: E501\n").is_empty());
    }

    #[test]
    fn a_suffix_still_decides_where_the_file_has_one() {
        assert!(run_on("notes.txt", "# shellcheck disable=SC2086\n").is_empty());
    }

    /// The kernel passes everything after the interpreter as one argument, so
    /// an optional argument is not the interpreter.
    #[test]
    fn an_optional_argument_is_not_the_interpreter() {
        let out = run_on("publish", "#!/bin/bash -e\n# shellcheck disable=SC2086\n");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
        assert!(run_on("run", "#!/usr/bin/perl bash\n# noqa: E501\n").is_empty());
    }

    /// `env` runs the first of its operands that is neither an option nor an
    /// assignment, so the script's own language is still the one read.
    #[test]
    fn an_env_shebang_reads_past_its_options_and_assignments() {
        for text in [
            "#!/usr/bin/env -S bash -e\n# shellcheck disable=SC2086\n",
            "#!/usr/bin/env -S LC_ALL=C bash\n# shellcheck disable=SC2086\n",
            "#!/usr/bin/env -S -u FOO bash\n# shellcheck disable=SC2086\n",
            "#!/usr/bin/env --unset FOO bash\n# shellcheck disable=SC2086\n",
        ] {
            let out = run_on("publish", text);
            assert_eq!(out.len(), 2, "{text}");
            assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
        }
    }

    /// An option's own argument is not the command, so a name that resembles
    /// an interpreter does not become one.
    #[test]
    fn an_env_option_argument_is_not_the_command() {
        assert_eq!(
            kind_name("publish", "#!/usr/bin/env -u python3 bash"),
            "publish.sh"
        );
        assert_eq!(
            kind_name("publish", "#!/usr/bin/env -C /tmp python3"),
            "publish.py"
        );
    }

    /// `env -S` reads quotes when it splits, so an argument carrying a space
    /// is one argument and its tail is not the command.
    #[test]
    fn a_quoted_env_option_argument_is_one_argument() {
        assert_eq!(
            kind_name("publish", "#!/usr/bin/env -S -C \"/tmp dir\" bash"),
            "publish.sh"
        );
        assert_eq!(
            kind_name("publish", "#!/usr/bin/env -S -C '/tmp/dir' python3"),
            "publish.py"
        );
    }

    /// The dot is tested on the basename, so a dot in a directory name is not
    /// a suffix on the file.
    #[test]
    fn a_dot_in_a_directory_is_not_a_suffix() {
        assert_eq!(kind_name("scripts.d/publish", "cmd\n"), "scripts.d/publish");
        assert_eq!(
            kind_name("scripts.d/publish", "#!/bin/sh"),
            "scripts.d/publish.sh"
        );
        assert_eq!(
            kind_name("scripts/publish.sh", "#!/usr/bin/env python3"),
            "scripts/publish.sh"
        );
    }

    /// The separator is the one this platform writes, so a Windows path
    /// splits where Windows splits it.
    #[cfg(windows)]
    #[test]
    fn a_windows_separator_still_bounds_the_basename() {
        assert_eq!(
            kind_name("scripts.d\\publish", "#!/bin/sh"),
            "scripts.d\\publish.sh"
        );
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
