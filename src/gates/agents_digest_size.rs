//! Gate: agent digests stay within their line budgets.
//!
//! The root `AGENTS.md` is loaded into every session, so its budget is 100
//! lines; a subtree digest gets 150. Vendored trees are pruned — a gate a
//! consumer cannot satisfy short of deleting `node_modules/` is a gate they
//! turn off. What a digest says is review's business; only size is judged.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, line_count, read_text, walk_files};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::AuthorInstructionsStayWithinBudget];

/// Judge every agent digest in the repository.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a digest cannot be read.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    if !ctx.path("AGENTS.md").is_file() {
        return Ok(vec![Violation::Layout("no root AGENTS.md".to_string())]);
    }
    let mut violations = Vec::new();
    for file in walk_files(ctx) {
        if file.file_name() != Some("AGENTS.md") {
            continue;
        }
        let cap = if file == "./AGENTS.md" { 100 } else { 150 };
        if line_count(&read_text(ctx, &file)?) > cap {
            violations.push(Violation::Finding(Finding::on_file(
                RuleId::AuthorInstructionsStayWithinBudget,
                file,
                "",
            )));
        }
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &tempfile::TempDir, path: &str, lines: usize) {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "line\n".repeat(lines)).unwrap();
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
    fn accepts_digests_within_budget() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "AGENTS.md", 100);
        write(&dir, "method/AGENTS.md", 150);
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn rejects_a_root_digest_over_its_budget() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "AGENTS.md", 101);
        let out = run_in(&dir);
        assert_eq!(
            out,
            vec!["FAIL docs-format:author-instructions-stay-within-budget ./AGENTS.md".to_string()]
        );
    }

    #[test]
    fn ignores_vendored_digests() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "AGENTS.md", 1);
        write(&dir, "node_modules/pkg/AGENTS.md", 400);
        assert!(run_in(&dir).is_empty());
    }

    #[test]
    fn a_missing_root_digest_is_a_layout_failure() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(run_in(&dir), vec!["FAIL no root AGENTS.md".to_string()]);
    }
}
