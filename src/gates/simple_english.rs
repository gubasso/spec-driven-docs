//! Gate: prose follows the objective `SimpleEnglish` checks.
//!
//! This is a compatibility port of the deterministic checks the vendored
//! `SimpleEnglish` linter expresses, adapted to `Markdown`, to structural passage
//! resolution, and to this project's citable-failure contract. It reports
//! only the objective subset: sentence length, contractions, perfect tense,
//! `-ing` verbs, banned modals, semicolons, and logic dashes. Judgment rules
//! — active voice, term familiarity, one-item-one-name — are review-held in
//! `SPEC-simple-english.md`, never inferred here.
//!
//! The passage mode comes from structure. Every sentence carries the 25-word
//! descriptive limit, except the command sentence of a numbered step in a
//! guide, which carries 20. The gate never reads a verb or a heading to guess
//! the mode. Protected spans stay exact: a code span counts as one word, and
//! an uppercase RFC 2119 keyword is left alone.

use std::sync::OnceLock;

use regex::Regex;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::markdown_prose::{LineKind, classify};
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[
    RuleId::ObjectiveCheckMatchesItsUpstreamRule,
    RuleId::ExceptionNamesItsReason,
];

/// Repository-relative files held under a shrinking adoption exemption for `SimpleEnglish`.
///
/// The gate still scans each one and suppresses its findings, but fails if a
/// listed file becomes clean or disappears, so the list cannot outlive the
/// corpus it covers (the `ESLint` unused-suppression model, and the method's
/// shrinking-exemption shape). Empty is the goal.
pub const ADOPTION_EXEMPT: &[&str] = &[];

const DESCRIPTIVE_LIMIT: usize = 25;
const PROCEDURAL_LIMIT: usize = 20;

#[allow(clippy::expect_used)]
fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    // The patterns are compile-time constants in this module; a compile
    // failure is a bug, not a runtime condition.
    cell.get_or_init(|| Regex::new(pattern).expect("static pattern compiles"))
}

/// Collapse every protected span to one token, so a word count and the
/// text checks see one word where a reader sees one unit.
fn collapse(text: &str) -> String {
    static CODE: OnceLock<Regex> = OnceLock::new();
    static URL: OnceLock<Regex> = OnceLock::new();
    static PAREN: OnceLock<Regex> = OnceLock::new();
    let mut out = re(&CODE, r"`[^`]+`")
        .replace_all(text, " CODE ")
        .into_owned();
    out = re(&URL, r"https?://\S+")
        .replace_all(&out, " URL ")
        .into_owned();
    out = re(&PAREN, r"\([^)]*\)")
        .replace_all(&out, " PAREN ")
        .into_owned();
    out
}

/// Split a collapsed text into sentences, dropping fragments under two words.
fn sentences(collapsed: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = collapsed.chars().peekable();
    while let Some(ch) = chars.next() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | ':') && chars.peek().is_none_or(|n| n.is_whitespace()) {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| s.split_whitespace().count() >= 2)
        .collect()
}

fn word_count(sentence: &str) -> usize {
    sentence.split_whitespace().count()
}

/// One deterministic finding on a line: its upstream rule and its category.
struct Hit {
    upstream: &'static str,
    category: &'static str,
    detail: String,
}

/// Count logic dashes: an em dash, a spaced double hyphen, or a spaced single
/// hyphen between two non-digits. A range or a flag is not a logic dash.
fn logic_dashes(text: &str) -> usize {
    let mut count = text.matches('—').count();
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '-' {
            // A spaced ` -- ` or ` - ` between two non-digit characters.
            let before = if i >= 2 { Some(bytes[i - 2]) } else { None };
            let is_double = bytes.get(i + 1) == Some(&'-');
            let (dash_end, after_gap) = if is_double {
                (i + 1, i + 2)
            } else {
                (i, i + 1)
            };
            let spaced_left = i >= 1 && bytes[i - 1] == ' ';
            let spaced_right = bytes.get(after_gap) == Some(&' ');
            let after = bytes.get(after_gap + 1).copied();
            if spaced_left && spaced_right {
                let left_ok = before.is_some_and(|c| !c.is_ascii_digit());
                let right_ok = after.is_some_and(|c| !c.is_ascii_digit());
                if left_ok && right_ok {
                    count += 1;
                }
            }
            i = dash_end + 1;
        } else {
            i += 1;
        }
    }
    count
}

fn text_checks(line: &str, hits: &mut Vec<Hit>) {
    static CONTRACTION: OnceLock<Regex> = OnceLock::new();
    static PERFECT: OnceLock<Regex> = OnceLock::new();
    static ING: OnceLock<Regex> = OnceLock::new();
    static MODAL: OnceLock<Regex> = OnceLock::new();
    let body = collapse(line);

    let contractions = re(
        &CONTRACTION,
        r"(?i)\b\w+(n't|'ll|'re|'ve|'d)\b|\bit's\b|\byou're\b",
    )
    .find_iter(&body)
    .count();
    for _ in 0..contractions {
        hits.push(Hit {
            upstream: "4.2",
            category: "contraction",
            detail: "a contraction; keep full grammar".to_string(),
        });
    }

    let perfect = re(
        &PERFECT,
        r"(?i)\b(has|have|had)\s+been\b|\b(has|have)\s+\w+ed\b",
    )
    .find_iter(&body)
    .count();
    for _ in 0..perfect {
        hits.push(Hit {
            upstream: "3.4",
            category: "perfect-tense",
            detail: "a perfect tense; use a simple tense".to_string(),
        });
    }

    let ing = re(
        &ING,
        r"(?i),\s*(mak|allow|enabl|ensur|highlight|creat|provid|offer|help|reduc|improv|lead|caus|result)ing\b",
    )
    .find_iter(&body)
    .count();
    for _ in 0..ing {
        hits.push(Hit {
            upstream: "3.5",
            category: "ing-verb",
            detail: "an '-ing' clause as a verb; start a new sentence".to_string(),
        });
    }

    for m in re(&MODAL, r"(?i)\b(should|would|may|might|could|shall)\b").find_iter(&body) {
        // An uppercase RFC 2119 keyword is protected, not a banned modal.
        if m.as_str().chars().all(|c| c.is_ascii_uppercase()) {
            continue;
        }
        hits.push(Hit {
            upstream: "3.2",
            category: "banned-modal",
            detail: format!("the modal '{}'; use can, will, or must", m.as_str()),
        });
    }

    let semicolons = body.matches(';').count();
    for _ in 0..semicolons {
        hits.push(Hit {
            upstream: "8.1",
            category: "semicolon",
            detail: "a semicolon; write two sentences".to_string(),
        });
    }

    let dashes = logic_dashes(&body);
    for _ in 0..dashes {
        hits.push(Hit {
            upstream: "8-dash",
            category: "logic-dash",
            detail: "a dash splicing two statements; name the relation or write two sentences"
                .to_string(),
        });
    }
}

fn finding(path: &str, number: usize, hit: &Hit) -> Violation {
    Violation::Finding(Finding::on_line(
        RuleId::ObjectiveCheckMatchesItsUpstreamRule,
        path,
        number,
        format!(
            "upstream {} [{}]: {}",
            hit.upstream, hit.category, hit.detail
        ),
    ))
}

/// Whether the file is governed as a guide, where a numbered step command
/// carries the procedural limit.
fn is_guide(path: &str) -> bool {
    path.contains("/guides/") || path.rsplit('/').next() == Some("TEMPLATE-guide.md")
}

struct Directive {
    open_line: usize,
    close_line: Option<usize>,
    used: bool,
}

/// A directive control marker, distinguished from an ordinary comment.
enum Marker {
    Open { reason_ok: bool },
    Close,
    Unknown,
}

#[allow(clippy::option_if_let_else)]
fn directive(content: &str) -> Option<Marker> {
    let inner = content.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    let body = inner.strip_prefix("simple-english-")?;
    if let Some(rest) = body.strip_prefix("disable") {
        let reason = rest.trim_start_matches(':').trim();
        Some(Marker::Open {
            reason_ok: !reason.is_empty(),
        })
    } else if body.trim() == "enable" {
        Some(Marker::Close)
    } else {
        Some(Marker::Unknown)
    }
}

#[allow(clippy::too_many_lines)]
fn judge(path: &str, text: &str) -> Vec<Violation> {
    let kinds = classify(text);
    let guide = is_guide(path);
    let mut directive_violations = Vec::new();
    let mut regions: Vec<Directive> = Vec::new();
    let mut open: Option<usize> = None;

    // First pass: the exception directives, which are comment lines.
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        if !matches!(kinds.get(index), Some(LineKind::Comment)) {
            continue;
        }
        let content = raw.trim();
        match directive(content) {
            None => {}
            Some(Marker::Open { reason_ok }) => {
                if !reason_ok {
                    directive_violations.push(Violation::Finding(Finding::on_line(
                        RuleId::ExceptionNamesItsReason,
                        path,
                        number,
                        "an exception directive carries no reason".to_string(),
                    )));
                }
                if open.is_some() {
                    directive_violations.push(Violation::Finding(Finding::on_line(
                        RuleId::ExceptionNamesItsReason,
                        path,
                        number,
                        "a nested exception directive; close the first".to_string(),
                    )));
                } else {
                    open = Some(number);
                }
            }
            Some(Marker::Close) => {
                if let Some(start) = open.take() {
                    regions.push(Directive {
                        open_line: start,
                        close_line: Some(number),
                        used: false,
                    });
                } else {
                    directive_violations.push(Violation::Finding(Finding::on_line(
                        RuleId::ExceptionNamesItsReason,
                        path,
                        number,
                        "an exception close with no open directive".to_string(),
                    )));
                }
            }
            Some(Marker::Unknown) => {
                directive_violations.push(Violation::Finding(Finding::on_line(
                    RuleId::ExceptionNamesItsReason,
                    path,
                    number,
                    "an unknown simple-english directive".to_string(),
                )));
            }
        }
    }
    if let Some(start) = open {
        directive_violations.push(Violation::Finding(Finding::on_line(
            RuleId::ExceptionNamesItsReason,
            path,
            start,
            "an exception directive is never closed".to_string(),
        )));
        regions.push(Directive {
            open_line: start,
            close_line: None,
            used: true, // an unterminated region is already a failure
        });
    }

    let disabled = |number: usize, regions: &mut Vec<Directive>| -> bool {
        for region in regions.iter_mut() {
            let end = region.close_line.unwrap_or(usize::MAX);
            if number > region.open_line && number < end {
                region.used = true;
                return true;
            }
        }
        false
    };

    // Second pass: the prose checks.
    let mut violations = Vec::new();
    for kind in &kinds {
        let LineKind::Prose(prose) = kind else {
            continue;
        };
        let mut hits = Vec::new();
        text_checks(&prose.content, &mut hits);
        let collapsed = collapse(&prose.content);
        for (index, sentence) in sentences(&collapsed).into_iter().enumerate() {
            let procedural = guide && prose.ordered_item && index == 0;
            let limit = if procedural {
                PROCEDURAL_LIMIT
            } else {
                DESCRIPTIVE_LIMIT
            };
            let count = word_count(&sentence);
            if count > limit {
                let mode = if procedural { "5.1" } else { "6.3" };
                hits.push(Hit {
                    upstream: mode,
                    category: "sentence-over-limit",
                    detail: format!("{count} words, limit {limit}"),
                });
            }
        }
        if hits.is_empty() {
            continue;
        }
        if disabled(prose.number, &mut regions) {
            continue;
        }
        for hit in &hits {
            violations.push(finding(path, prose.number, hit));
        }
    }

    for region in &regions {
        if !region.used {
            directive_violations.push(Violation::Finding(Finding::on_line(
                RuleId::ExceptionNamesItsReason,
                path,
                region.open_line,
                "an exception region reports nothing; remove it".to_string(),
            )));
        }
    }

    directive_violations.extend(violations);
    directive_violations
}

/// Judge every markdown file pre-commit passed.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a file cannot be read.
pub fn run(ctx: &GateCtx, files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for file in files {
        let relative = file.trim_start_matches("./");
        let text = read_text(ctx, file)?;
        let found = judge(relative, &text);
        if ADOPTION_EXEMPT.contains(&relative) {
            if found.is_empty() {
                violations.push(Violation::Finding(Finding::on_file(
                    RuleId::ObjectiveCheckMatchesItsUpstreamRule,
                    relative,
                    "is clean; remove it from the adoption exemption list",
                )));
            }
            continue;
        }
        violations.extend(found);
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_on_named(name: &str, text: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[name.to_string()])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn run_on(text: &str) -> Vec<String> {
        run_on_named("doc.md", text)
    }

    #[test]
    fn accepts_plain_prose() {
        assert!(
            run_on("# Title\n\nThe gate reads the file. It reports one finding per breach.\n")
                .is_empty()
        );
    }

    #[test]
    fn a_descriptive_sentence_over_25_words_fails() {
        let long = "This one sentence runs on and on and on and on and on and on and on and on and on and on and on and on well past the limit here.";
        let out = run_on(&format!("# T\n\n{long}\n"));
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("[sentence-over-limit]"));
        assert!(out[0].contains("upstream 6.3"));
    }

    #[test]
    fn a_25_word_descriptive_sentence_passes() {
        let s = "one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five.";
        assert!(run_on(&format!("# T\n\n{s}\n")).is_empty());
    }

    #[test]
    fn a_guide_step_command_uses_the_20_word_limit() {
        let s = "Run the command that installs the tool and configures it and verifies it and prints the version and exits cleanly now.";
        // 21 words, ordered item in a guide: procedural, over 20.
        let out = run_on_named("_docs/guides/x.md", &format!("# T\n\n1. {s}\n"));
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("upstream 5.1"));
    }

    #[test]
    fn the_same_sentence_passes_as_descriptive_prose() {
        let s = "Run the command that installs the tool and configures it and verifies it and prints the version and exits cleanly now.";
        assert!(run_on(&format!("# T\n\n{s}\n")).is_empty());
    }

    #[test]
    fn a_code_span_counts_as_one_word() {
        let s = "Run `a b c d e f g h i j k l m n o p q r s t u v w` once more.";
        assert!(run_on(&format!("# T\n\n{s}\n")).is_empty());
    }

    #[test]
    fn an_uppercase_rfc_keyword_is_not_a_modal() {
        assert!(run_on("# T\n\nThe author MUST keep it exact.\n").is_empty());
        assert_eq!(run_on("# T\n\nThe author should keep it exact.\n").len(), 1);
    }

    #[test]
    fn a_contraction_and_a_semicolon_each_fail() {
        let out = run_on("# T\n\nYou're done; the tool exits.\n");
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn a_range_is_not_a_logic_dash_but_an_em_dash_is() {
        assert!(run_on("# T\n\nThe window is 5 - 10 minutes wide.\n").is_empty());
        assert_eq!(
            run_on("# T\n\nThe deploy failed — the disk was full.\n").len(),
            1
        );
    }

    #[test]
    fn a_reasoned_exception_region_suppresses_a_finding() {
        let long = "This one sentence runs on and on and on and on and on and on and on and on and on and on and on and on well past the limit here.";
        let text = format!(
            "# T\n\n<!-- simple-english-disable: marketing copy -->\n\n{long}\n\n<!-- simple-english-enable -->\n"
        );
        assert!(run_on(&text).is_empty());
    }

    #[test]
    fn an_exception_without_a_reason_fails() {
        let text = "# T\n\n<!-- simple-english-disable -->\n\nplain text here.\n\n<!-- simple-english-enable -->\n";
        let out = run_on(text);
        assert!(out.iter().any(|v| v.contains("carries no reason")));
    }

    #[test]
    fn an_unclosed_exception_fails() {
        let long = "This runs on and on and on and on and on and on and on and on and on and on and on and on well past the descriptive limit here now.";
        let text = format!("# T\n\n<!-- simple-english-disable: reason -->\n\n{long}\n");
        let out = run_on(&text);
        assert!(out.iter().any(|v| v.contains("never closed")));
    }

    #[test]
    fn an_unused_exception_region_fails() {
        let text = "# T\n\n<!-- simple-english-disable: reason -->\n\nplain short text.\n\n<!-- simple-english-enable -->\n";
        let out = run_on(text);
        assert!(out.iter().any(|v| v.contains("reports nothing")));
    }

    #[test]
    fn a_directive_inside_a_fence_is_text() {
        let text = "# T\n\n```text\n<!-- simple-english-disable -->\n```\n\nplain text.\n";
        assert!(run_on(text).is_empty());
    }
}
