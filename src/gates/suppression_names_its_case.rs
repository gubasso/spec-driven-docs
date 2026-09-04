//! Gate: every markdown suppression names the case that justifies it, and
//! that case resolves to a record.
//!
//! A suppression with no case behind it becomes permanent by default: the
//! next reader takes it for a design choice and nothing says what would
//! retire it. Scoped to the HTML-comment form, which is the suppression a
//! document can carry; prose about a suppression is content. Binary files
//! and vendored trees are skipped, and the known-issues directory is exempt
//! — a record may legitimately discuss suppressions.

use std::collections::BTreeSet;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, GateError, GateResult, Violation, walk_files};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::SuppressionNamesItsCase];

const RULE: RuleId = RuleId::SuppressionNamesItsCase;

fn is_suppression(line: &str) -> bool {
    let mut rest = line;
    while let Some(index) = rest.find("<!--") {
        let after = rest[index + 4..].trim_start_matches(' ');
        if after.starts_with("dprint-ignore") || after.starts_with("markdownlint-disable") {
            return true;
        }
        rest = &rest[index + 4..];
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

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(4096).any(|&b| b == 0)
}

/// Judge every suppression in the repository.
///
/// # Errors
///
/// [`GateError::Io`] when a candidate file cannot be read.
pub fn run(ctx: &GateCtx, args: &[String]) -> GateResult {
    let mut suppressions: Vec<(String, usize, String)> = Vec::new();
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
        for (number, line) in text.lines().enumerate() {
            if is_suppression(line) && !is_closing(line) {
                suppressions.push((
                    file.as_str().trim_start_matches("./").to_string(),
                    number + 1,
                    line.to_string(),
                ));
            }
        }
    }

    let mut violations = Vec::new();
    let caseless: Vec<&(String, usize, String)> = suppressions
        .iter()
        .filter(|(_, _, line)| cited_cases(line).next().is_none())
        .collect();
    if !caseless.is_empty() {
        violations.push(Violation::Finding(Finding::global(RULE, "")));
        for (file, number, line) in caseless {
            violations.push(Violation::Note(format!("./{file}:{number}:{line}")));
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
    let cited: BTreeSet<String> = suppressions
        .iter()
        .flat_map(|(_, _, line)| cited_cases(line))
        .collect();
    for case in cited {
        if !known.contains(&case) {
            violations.push(Violation::Finding(Finding::global(
                RULE,
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

    fn run_in(dir: &tempfile::TempDir) -> Vec<String> {
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn a_repository_without_suppressions_passes() {
        assert!(run_in(&fixture()).is_empty());
    }

    #[test]
    fn a_suppression_naming_a_record_passes() {
        let dir = fixture();
        std::fs::write(
            dir.path().join("local.md"),
            format!(
                "<!-- markdownlint-{} MD013 KI-vendor-quirk -->\n",
                "disable"
            ),
        )
        .unwrap();
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn a_caseless_suppression_is_rejected_with_its_line() {
        let dir = fixture();
        std::fs::write(
            dir.path().join("local.md"),
            format!("<!-- markdownlint-{} -->\n", "disable"),
        )
        .unwrap();
        let out = run_in(&dir);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], "FAIL spec-to-code:a-suppression-names-its-case");
        assert!(out[1].contains("local.md:1"));
    }

    #[test]
    fn a_case_resolving_to_no_record_is_rejected() {
        let dir = fixture();
        std::fs::write(
            dir.path().join("local.md"),
            format!("<!-- markdownlint-{} KI-absent-record -->\n", "disable"),
        )
        .unwrap();
        let out = run_in(&dir);
        assert_eq!(
            out,
            vec![
                "FAIL spec-to-code:a-suppression-names-its-case: KI-absent-record resolves to no record"
                    .to_string()
            ]
        );
    }

    #[test]
    fn closing_markers_are_not_suppressions() {
        let dir = fixture();
        std::fs::write(
            dir.path().join("local.md"),
            "<!-- dprint-ignore-end -->\n<!-- markdownlint-enable -->\n",
        )
        .unwrap();
        assert!(run_in(&dir).is_empty());
    }
}
