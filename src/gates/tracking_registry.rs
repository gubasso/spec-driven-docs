//! Gate: the tracking registry is valid, current, and its dependents exist.
//!
//! The registry is `<docs_root>/reference/tracking.yaml`. This gate parses
//! and bounds it, resolves every declared path, and fails an overdue entry
//! naming the due date and the recovery steps. It reads current bytes only
//! and makes no claim about edit history, and it needs no network — comparing
//! a pinned revision to its upstream is `sdd track check`. A repository with
//! no registry has nothing to check.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::docs_root;
use crate::gates::{GateCtx, GateResult, Violation};
use crate::services::tracking::{evaluate, today_utc};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[
    RuleId::RegistryHasOneReadableShape,
    RuleId::PerishableSourceIsRegistered,
    RuleId::UpstreamDerivationPinsARevision,
    RuleId::EntryDeclaresHowToRevalidate,
    RuleId::OverdueEntryBlocks,
    RuleId::DeclaredDependentExists,
];

/// Judge the tracking registry.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when the registry is present but cannot be
/// read.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    let root = docs_root(ctx);
    let registry = ctx.path(&root).join("reference/tracking.yaml");
    if !registry.is_file() {
        return Ok(vec![]);
    }
    let report = evaluate(&ctx.repo_root, &root, today_utc()).map_err(|source| {
        crate::gates::GateError::io(root.join("reference/tracking.yaml"), source)
    })?;

    let mut violations = Vec::new();
    if let Some(fatal) = &report.fatal {
        violations.push(Violation::Finding(Finding::global(
            fatal.rule,
            fatal.detail.clone(),
        )));
        return Ok(violations);
    }
    for assessed in &report.entries {
        // Overdue and every other problem is already carried on the entry,
        // each citing its own rule; a current, clean entry adds nothing.
        for problem in &assessed.problems {
            violations.push(Violation::Finding(Finding::global(
                problem.rule,
                problem.detail.clone(),
            )));
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_in(dir: &tempfile::TempDir) -> Vec<String> {
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn write(dir: &tempfile::TempDir, rel: &str, text: &str) {
        let p = dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn a_repository_without_a_registry_passes() {
        let dir = tempfile::tempdir().unwrap();
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn an_overdue_entry_fails_naming_the_rule() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            "_docs/reference/tracking.yaml",
            "schema_version: 1\ntracked:\n  - id: sample\n    path: _docs/reference/x.md\n    last_checked: 2000-01-01\n    cadence_days: 30\n    why: it moves\n    revalidate:\n      - re-fetch it\n    dependents: []\n",
        );
        write(&dir, "_docs/reference/x.md", "# x\n");
        let out = run_in(&dir);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("tracking:an-overdue-entry-blocks"));
    }

    #[test]
    fn a_broken_registry_fails_once() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            "_docs/reference/tracking.yaml",
            "schema_version: 1\ntracked: &all []\nother: *all\n",
        );
        let out = run_in(&dir);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("tracking:the-registry-has-one-readable-shape"));
    }
}
