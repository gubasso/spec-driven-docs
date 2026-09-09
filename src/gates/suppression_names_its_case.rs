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
//! it. Binary files and vendored trees are skipped, and the known-issues
//! directory is exempt, because a record may discuss suppressions.
//!
//! Three surfaces stay outside a line-scoped scan: an extensionless shell
//! script, a block comment holding a suppression, and the `[lints]` table
//! of a manifest. The `simple-english-disable` marker stays outside too,
//! because `simple-english:an-exception-names-its-reason` already requires
//! a reason on it, and a second rule would name one defect twice.

use std::collections::BTreeSet;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
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
    FAMILIES
        .iter()
        .find(|family| family.suffixes.iter().any(|suffix| file.ends_with(suffix)))
        .and_then(|family| match family.opener {
            Opener::Line => Some("//"),
            Opener::After("<!--") => None,
            Opener::After(opener) => Some(opener),
        })
}

/// Every occurrence of `token` that follows `opener` on the line.
fn follows_opener(line: &str, opener: &str, token: &str) -> bool {
    let mut rest = line;
    while let Some(index) = rest.find(opener) {
        let after = rest[index + opener.len()..].trim_start_matches(' ');
        if after.starts_with(token) {
            return true;
        }
        rest = &rest[index + opener.len()..];
    }
    false
}

fn is_suppression(file: &str, line: &str) -> bool {
    for family in FAMILIES {
        if !family.suffixes.iter().any(|suffix| file.ends_with(suffix)) {
            continue;
        }
        let matched = family.tokens.iter().any(|token| match family.opener {
            Opener::Line => line.trim_start().starts_with(token),
            Opener::After(opener) => follows_opener(line, opener, token),
        });
        if matched {
            return true;
        }
    }
    false
}

fn is_closing(line: &str) -> bool {
    line.contains("dprint-ignore-end") || line.contains("markdownlint-enable")
}

fn cited_cases(line: &str) -> impl Iterator<Item = String> + '_ {
    line.match_indices("KI-").filter_map(|(index, _)| {
        let slug: String = line[index + 3..]
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        (!slug.is_empty()).then(|| format!("KI-{slug}"))
    })
}

/// The reason a permanent marker states, where the annotation carries one.
fn permanent_reason(text: &str) -> Option<String> {
    let index = text.find(MARKER)?;
    let rest = text[index + MARKER.len()..]
        .trim_end_matches("-->")
        .trim_end_matches("*/")
        .trim();
    Some(rest.to_string())
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(4096).any(|&b| b == 0)
}

/// One suppression, with the text a reader can read its reason from.
struct Site {
    file: String,
    number: usize,
    line: String,
    annotation: String,
}

/// The suppression line, plus the line above it when that line is a comment
/// in the file's own syntax. Eight of Rust's own idiomatic sites carry the
/// reason above the attribute, and a multi-line inner attribute has no room
/// on its own line.
fn annotation(file: &str, lines: &[&str], index: usize) -> String {
    let line = lines[index];
    let Some(opener) = comment_opener(file) else {
        return line.to_string();
    };
    let above = index
        .checked_sub(1)
        .map(|previous| lines[previous].trim_start())
        .filter(|previous| previous.starts_with(opener))
        .unwrap_or_default();
    format!("{above}\n{line}")
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
        for (index, line) in lines.iter().enumerate() {
            if is_suppression(&name, line) && !is_closing(line) {
                sites.push(Site {
                    file: name.clone(),
                    number: index + 1,
                    line: (*line).to_string(),
                    annotation: annotation(&name, &lines, index),
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
        let cased = cited_cases(&site.annotation).next().is_some();
        match (cased, permanent_reason(&site.annotation)) {
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
        .flat_map(|site| cited_cases(&site.annotation))
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
        let out = run_on("local.rs", "#[allow(dead_code)] // sdd: permanent\n");
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0],
            "FAIL spec-to-code:a-permanent-exception-states-its-reason local.rs:1: the permanent marker states no reason"
        );
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
