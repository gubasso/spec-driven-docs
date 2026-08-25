//! Gate: every rule ID a gate can print resolves to a requirement in the
//! local specs.
//!
//! The id in a failure message is an address: the reader follows it to the
//! sentence that binds, and a message naming an id no spec defines sends
//! them nowhere. The citable set comes from the compiled gate registry —
//! every rule a delivered gate declares — and the defined set from the
//! instance's own specs, so a spec rewrite that renames a rule fails here
//! before a gate ever prints the stale address.

use std::collections::BTreeSet;

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::docs_root;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::GateMessageCitesTheRule];

/// Judge the registry's citable set against the local specs.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a local spec cannot be read.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    let specs = docs_root(ctx).join("specs");
    let Ok(entries) = ctx.path(&specs).read_dir_utf8() else {
        return Ok(vec![Violation::Layout(format!(
            "no specs directory at {specs}; the layout moved"
        ))]);
    };
    let mut defined: BTreeSet<String> = BTreeSet::new();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        // The corpus convention is lowercase; `.MD` is not a spec.
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        if !name.ends_with(".md") {
            continue;
        }
        defined.extend(crate::embedded::rule_ids_in(&read_text(
            ctx,
            specs.join(name),
        )?));
    }

    let cited: BTreeSet<RuleId> = crate::gates::GATES
        .iter()
        .flat_map(|gate| gate.cites.iter().copied())
        .collect();
    Ok(cited
        .into_iter()
        .filter(|rule| !defined.contains(rule.as_str()))
        .map(|rule| {
            Violation::Finding(Finding::global(
                RuleId::GateMessageCitesTheRule,
                format!("{rule} resolves to no requirement"),
            ))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    fn write_spec(dir: &tempfile::TempDir, name: &str, text: &str) {
        let specs = dir.path().join("_docs/specs");
        std::fs::create_dir_all(&specs).unwrap();
        std::fs::write(specs.join(name), text).unwrap();
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
    fn accepts_specs_defining_every_citable_rule() {
        let dir = tempfile::tempdir().unwrap();
        let cited: BTreeSet<RuleId> = crate::gates::GATES
            .iter()
            .flat_map(|gate| gate.cites.iter().copied())
            .collect();
        let mut text = String::new();
        for rule in &cited {
            let _ = write!(text, "### `{rule}` — Rule\n\n");
        }
        write_spec(&dir, "SPEC-all.md", &text);
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn rejects_a_citable_rule_the_local_specs_lost() {
        let dir = tempfile::tempdir().unwrap();
        write_spec(&dir, "SPEC-thin.md", "### `sample:works` — Works\n");
        let out = run_in(&dir);
        assert!(!out.is_empty());
        assert!(out.iter().all(|line| {
            line.starts_with("FAIL spec-to-code:a-gate-message-cites-the-rule: ")
                && line.ends_with(" resolves to no requirement")
        }));
    }

    #[test]
    fn a_missing_specs_directory_is_a_layout_failure() {
        let dir = tempfile::tempdir().unwrap();
        let out = run_in(&dir);
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("FAIL no specs directory at "));
        assert!(out[0].ends_with("; the layout moved"));
    }
}
