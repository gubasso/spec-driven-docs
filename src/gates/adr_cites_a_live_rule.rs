//! Gate: a record's `Enforced by` line names rules the specs still define.
//!
//! That line is the record's one forward reference: the argument is frozen,
//! but the pointer to what binds today has to keep resolving, and a rule
//! renamed or retired leaves it aimed at nothing. Only that line is judged.
//! A record naming a retired rule in its prose is describing the retirement,
//! which is the record doing its job.
//!
//! Corpus-wide by construction, like the uniqueness gate: the break happens
//! in the spec that renames a rule, not in the record that cited it, so a
//! gate reading only the files a commit touched would stay green through the
//! commit that breaks every citation.

use std::collections::BTreeSet;

use camino::Utf8PathBuf;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::docs_root;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::CitationResolvesToARule];

const RULE: RuleId = RuleId::CitationResolvesToARule;

/// The `` `domain:rule` `` tokens a line carries as inline code.
fn cited_ids(line: &str) -> Vec<&str> {
    line.split('`')
        .skip(1)
        .step_by(2)
        .filter(|token| {
            token.split_once(':').is_some_and(|(domain, name)| {
                !domain.is_empty()
                    && !name.is_empty()
                    && [domain, name].iter().all(|part| {
                        part.bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                    })
            })
        })
        .collect()
}

/// Every decision record under the resolved documentation root.
fn records(ctx: &GateCtx) -> Vec<Utf8PathBuf> {
    let decisions = docs_root(ctx).join("decisions");
    let Ok(entries) = ctx.path(&decisions).read_dir_utf8() else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.file_name().to_string())
        .filter(|name| {
            name.strip_prefix("ADR-")
                .and_then(|rest| rest.strip_suffix(".md"))
                .is_some_and(|slug| !slug.is_empty())
        })
        .collect();
    names.sort();
    names.into_iter().map(|name| decisions.join(name)).collect()
}

/// Judge the `Enforced by` line of every record under the docs root.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a record or a local spec cannot be
/// read.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    let Some(specs) = crate::gates::spec_rule_id_unique::spec_files(ctx) else {
        return Ok(vec![Violation::Layout(
            "no specs matched; the layout moved".to_string(),
        )]);
    };
    let mut defined: BTreeSet<String> = BTreeSet::new();
    for spec in specs {
        defined.extend(crate::embedded::rule_ids_in(&read_text(ctx, &spec)?));
    }

    let mut violations = Vec::new();
    for file in records(ctx) {
        let text = read_text(ctx, &file)?;
        for line in text.lines() {
            if !line.starts_with("Enforced by ") {
                continue;
            }
            for id in cited_ids(line) {
                if !defined.contains(id) {
                    violations.push(Violation::Finding(Finding::on_file(
                        RULE,
                        &file,
                        format!("`{id}` resolves to no requirement"),
                    )));
                }
            }
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_inline_code_slug_pairs() {
        assert_eq!(
            cited_ids("Enforced by `docs-format:prose-stays-unwrapped`."),
            vec!["docs-format:prose-stays-unwrapped"]
        );
        assert_eq!(
            cited_ids("Enforced by `a:b` and `c:d`."),
            vec!["a:b", "c:d"]
        );
        assert!(cited_ids("Enforced by review.").is_empty());
        assert!(cited_ids("Enforced by `cargo run`.").is_empty());
        assert!(cited_ids("Enforced by `Status: Accepted`.").is_empty());
    }

    fn write(dir: &tempfile::TempDir, path: &str, text: &str) {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// A target holding one spec and one record citing `auth:token-expiry`.
    fn instance(spec_rules: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let mut body = String::new();
        for id in spec_rules {
            body.push_str("### `");
            body.push_str(id);
            body.push_str("` — A rule\n\nVerify: `true`\n\n");
        }
        write(
            &dir,
            "_docs/specs/SPEC-auth.md",
            &format!("# Auth\n\n{body}"),
        );
        write(
            &dir,
            "_docs/decisions/ADR-bound-the-token.md",
            "# Bound the token\n\nEnforced by `auth:token-expiry`.\n",
        );
        dir
    }

    fn run_in(dir: &tempfile::TempDir) -> Vec<String> {
        run(&GateCtx::new(dir.path().to_str().unwrap()), &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn accepts_a_citation_the_specs_define() {
        assert!(run_in(&instance(&["auth:token-expiry"])).is_empty());
    }

    /// The break happens in the spec, and pre-commit hands this gate no
    /// record path when only that spec changed.
    #[test]
    fn rejects_a_citation_the_specs_stopped_defining() {
        let out = run_in(&instance(&["auth:token-refresh"]));
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("decision-records:a-citation-resolves-to-a-rule"));
        assert!(out[0].contains("`auth:token-expiry` resolves to no requirement"));
        assert!(out[0].contains("_docs/decisions/ADR-bound-the-token.md"));
    }

    #[test]
    fn judges_only_the_enforced_by_line() {
        let dir = instance(&["auth:token-refresh"]);
        write(
            &dir,
            "_docs/decisions/ADR-bound-the-token.md",
            "# Bound the token\n\nThe rule `auth:token-expiry` was retired here.\n",
        );
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn reports_a_moved_layout_rather_than_passing() {
        let dir = tempfile::tempdir().unwrap();
        let out = run_in(&dir);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("the layout moved"));
    }

    /// The real corpus is the canon's own, and it must stay clean.
    #[test]
    fn the_canon_corpus_cites_only_live_rules() {
        assert!(
            run(&GateCtx::new("."), &[]).unwrap().is_empty(),
            "a decision record in this repository cites a rule no spec defines"
        );
    }
}
