//! Where an instance keeps the documents the gates read.
//!
//! The documentation root comes from the manifest when one exists, is
//! discovered from the conventional layouts when none does, and defaults to
//! `_docs`. Known-issue roots follow the same ladder, and explicit arguments
//! win over all of it — a repository keeping records outside the root passes
//! the directories holding them. Nothing here judges content; that is each
//! gate's business.

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::manifest::MANIFEST_PATH;
use crate::gates::GateCtx;

/// The instance's documentation root, relative to the repository.
#[must_use]
pub fn docs_root(ctx: &GateCtx) -> Utf8PathBuf {
    if let Ok(text) = std::fs::read_to_string(ctx.path(MANIFEST_PATH))
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(root) = value.get("docs_root").and_then(serde_json::Value::as_str)
        && !root.is_empty()
    {
        return Utf8PathBuf::from(root);
    }
    for candidate in ["_docs", "docs"] {
        if ctx.path(candidate).join("specs").is_dir() {
            return Utf8PathBuf::from(candidate);
        }
    }
    Utf8PathBuf::from("_docs")
}

/// The directories that may hold known-issue records, relative to the
/// repository. Arguments win; a manifest names one root; a bare consumer's
/// roots are discovered.
#[must_use]
pub fn ki_record_roots(ctx: &GateCtx, args: &[String]) -> Vec<Utf8PathBuf> {
    if !args.is_empty() {
        return args.iter().map(Utf8PathBuf::from).collect();
    }
    if ctx.path(MANIFEST_PATH).is_file() {
        return vec![docs_root(ctx).join("reference/known-issues")];
    }
    ["_docs", "docs"]
        .into_iter()
        .map(|candidate| Utf8Path::new(candidate).join("reference/known-issues"))
        .filter(|root| ctx.path(root).is_dir())
        .collect()
}

/// Every known-issue record under the resolved roots, repository-relative.
#[must_use]
pub fn ki_records(ctx: &GateCtx, args: &[String]) -> Vec<Utf8PathBuf> {
    let mut records = Vec::new();
    for root in ki_record_roots(ctx, args) {
        let Ok(entries) = ctx.path(&root).read_dir_utf8() else {
            continue;
        };
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .map(|entry| entry.file_name().to_string())
            .filter(|name| {
                name.strip_prefix("KI-")
                    .and_then(|rest| rest.strip_suffix(".md"))
                    .is_some_and(|slug| !slug.is_empty())
            })
            .collect();
        names.sort();
        records.extend(names.into_iter().map(|name| root.join(name)));
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dir: &tempfile::TempDir) -> GateCtx {
        GateCtx::new(dir.path().to_str().unwrap())
    }

    fn write(dir: &tempfile::TempDir, path: &str, text: &str) {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn manifest_root_wins() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            ".spec-driven-docs/manifest.json",
            "{\n  \"docs_root\": \"docs\"\n}\n",
        );
        assert_eq!(docs_root(&ctx(&dir)), "docs");
    }

    #[test]
    fn roots_are_discovered_without_a_manifest() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "docs/specs/SPEC-sample.md", "# S\n");
        assert_eq!(docs_root(&ctx(&dir)), "docs");

        let both = tempfile::tempdir().unwrap();
        write(&both, "_docs/specs/SPEC-sample.md", "# S\n");
        write(&both, "docs/specs/SPEC-sample.md", "# S\n");
        assert_eq!(docs_root(&ctx(&both)), "_docs");

        let neither = tempfile::tempdir().unwrap();
        assert_eq!(docs_root(&ctx(&neither)), "_docs");
    }

    #[test]
    fn record_arguments_win_over_discovery() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "docs/reference/known-issues/KI-real.md", "# R\n");
        let roots = ki_record_roots(&ctx(&dir), &["tests/fixtures".to_string()]);
        assert_eq!(roots, vec![Utf8PathBuf::from("tests/fixtures")]);
    }

    #[test]
    fn records_follow_the_manifest_root() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            ".spec-driven-docs/manifest.json",
            "{\n  \"docs_root\": \"docs\"\n}\n",
        );
        write(&dir, "docs/reference/known-issues/KI-vendor.md", "# V\n");
        write(&dir, "docs/reference/known-issues/KI-.md", "# empty slug\n");
        write(
            &dir,
            "docs/reference/known-issues/notes.md",
            "# not a record\n",
        );
        assert_eq!(
            ki_records(&ctx(&dir), &[]),
            vec![Utf8PathBuf::from(
                "docs/reference/known-issues/KI-vendor.md"
            )]
        );
    }

    #[test]
    fn bare_consumer_roots_are_discovered() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "docs/reference/known-issues/KI-a.md", "# A\n");
        write(&dir, "docs/reference/known-issues/KI-b.md", "# B\n");
        assert_eq!(
            ki_records(&ctx(&dir), &[]),
            vec![
                Utf8PathBuf::from("docs/reference/known-issues/KI-a.md"),
                Utf8PathBuf::from("docs/reference/known-issues/KI-b.md"),
            ]
        );
    }
}
