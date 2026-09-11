//! Where an instance keeps the documents the gates read.
//!
//! Two of these locations are declared rather than fixed: the plan zone and
//! the docs scratch. Each is recorded in the manifest and named by one
//! environment variable, and the variable wins where it is set.
//!
//! The documentation root comes from the manifest when one exists, is
//! discovered from the conventional layouts when none does, and defaults to
//! `_docs`. Known-issue roots follow the same ladder, and explicit arguments
//! win over all of it — a repository keeping records outside the root passes
//! the directories holding them. Nothing here judges content; that is each
//! gate's business.

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::manifest::{DOCS_SCRATCH_VAR, MANIFEST_PATH, PLAN_ZONE_VAR};
use crate::gates::{GateCtx, GateError};

/// One field of the instance manifest, read permissively.
///
/// A gate reads the record as free JSON rather than through `Manifest::parse`
/// on purpose: a record of another schema version is a reason to upgrade, and
/// a gate that went blind there would report a clean tree it never read.
fn manifest_field(ctx: &GateCtx, key: &str) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(ctx.path(MANIFEST_PATH)).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get(key).cloned().filter(|found| !found.is_null())
}

/// What a variable carries here, trimmed, or `None` when it is unset or blank.
fn variable(name: &str) -> Option<Utf8PathBuf> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(Utf8PathBuf::from)
}

/// Where a gate may look for entry documents, and what named the place.
///
/// The distinction is what keeps the check honest. A path a command may read
/// carries the name of whoever declared it, so an absent directory is
/// reported against that declaration. Every other case reads as no zone: an
/// untracked zone and an unset variable are absent on a fresh clone, and
/// failing there would judge a layout the project never promised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanZoneTarget {
    /// The variable named this path.
    Variable(Utf8PathBuf),
    /// The manifest recorded this tracked path.
    Tracked(Utf8PathBuf),
    /// The record declares a gated zone the reader cannot resolve.
    Broken(String),
    /// Nothing a command may check.
    Unchecked,
}

/// Resolve the plan zone: the variable first, then the recorded kind.
#[must_use]
pub fn plan_zone(ctx: &GateCtx) -> PlanZoneTarget {
    plan_zone_with(ctx, variable(PLAN_ZONE_VAR))
}

/// The resolution, with the variable's value supplied.
///
/// The environment is read at one boundary and passed in, so every case is
/// reachable from a test. This crate forbids unsafe code, and setting a
/// variable is unsafe from the 2024 edition on.
#[must_use]
pub fn plan_zone_with(ctx: &GateCtx, named: Option<Utf8PathBuf>) -> PlanZoneTarget {
    if let Some(path) = named {
        return PlanZoneTarget::Variable(path);
    }
    let Some(recorded) = manifest_field(ctx, "plan_zone") else {
        return PlanZoneTarget::Unchecked;
    };
    if recorded.get("kind").and_then(serde_json::Value::as_str) != Some("tracked") {
        return PlanZoneTarget::Unchecked;
    }
    // A tracked kind whose path is missing, empty, or not a string is a
    // broken declaration, never an absent one. Reading it as `Unchecked`
    // would skip a zone the project declared gated, which is the state
    // `07-lifecycle.md` forbids a gate from reaching.
    recorded
        .get("path")
        .and_then(serde_json::Value::as_str)
        .map_or_else(
            || {
                PlanZoneTarget::Broken(
                    "the recorded plan zone is tracked and carries no path".to_string(),
                )
            },
            |path| {
                if path.trim().is_empty() {
                    PlanZoneTarget::Broken(
                        "the recorded plan zone is tracked and its path is empty".to_string(),
                    )
                } else {
                    PlanZoneTarget::Tracked(Utf8PathBuf::from(path))
                }
            },
        )
}

/// Resolve the docs scratch: the variable first, then the recorded path.
///
/// `None` means the project declared none. A caller with a discovery
/// candidate of its own supplies it; there is none here, because a gate that
/// guessed the location would judge a directory nobody declared.
#[must_use]
pub fn docs_scratch(ctx: &GateCtx) -> Option<Utf8PathBuf> {
    docs_scratch_with(ctx, variable(DOCS_SCRATCH_VAR))
}

/// What [`DOCS_SCRATCH_VAR`] carries here, or `None` when it is unset.
///
/// The one place the environment is read for this value, so a caller that
/// resolves it can be tested by supplying the answer instead.
#[must_use]
pub fn docs_scratch_variable() -> Option<Utf8PathBuf> {
    variable(DOCS_SCRATCH_VAR)
}

/// The resolution, with the variable's value supplied.
#[must_use]
pub fn docs_scratch_with(ctx: &GateCtx, named: Option<Utf8PathBuf>) -> Option<Utf8PathBuf> {
    named.or_else(|| {
        manifest_field(ctx, "docs_scratch")
            .and_then(|value| value.as_str().map(Utf8PathBuf::from))
            .filter(|path| !path.as_str().is_empty())
    })
}

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
        if discovered(ctx, &Utf8Path::new(candidate).join("specs")) {
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
        .filter(|root| discovered(ctx, root))
        .collect()
}

/// Whether a discovered candidate is a directory the caller must read.
///
/// A candidate whose metadata cannot be read is kept rather than dropped.
/// `is_dir` answers false for a directory the process cannot stat, so
/// dropping it there would report an unreadable layout as a layout the
/// repository does not keep. Kept, it reaches the reader, which raises the
/// failure or reports the layout as moved rather than judging a tree it
/// never opened.
fn discovered(ctx: &GateCtx, root: &Utf8Path) -> bool {
    match std::fs::metadata(ctx.path(root)) {
        Ok(metadata) => metadata.is_dir(),
        Err(source) => source.kind() != std::io::ErrorKind::NotFound,
    }
}

/// Every known-issue record under the resolved roots, repository-relative.
///
/// A root that is not there is a zone the repository does not keep, and it
/// is skipped. Every other failure is raised: a directory the process
/// cannot read holds records this returns none of, and reporting that as an
/// empty zone would read as a clean review.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a present root cannot be listed.
/// The records this gate judges, filtered.
///
/// [`ki_records`] stays unfiltered because the same records are support for
/// `suppression-names-its-case`, which resolves a case id against them
/// rather than judging their contents. A gate that judges a record's own
/// text calls this one.
///
/// # Errors
///
/// See [`ki_records`].
pub fn ki_records_judged(ctx: &GateCtx, args: &[String]) -> Result<Vec<Utf8PathBuf>, GateError> {
    Ok(ctx.retained(ki_records(ctx, args)?))
}

/// Every known-issue record the instance carries, unfiltered.
///
/// Unfiltered because the same records are support for
/// `suppression-names-its-case`. A gate judging a record's own text calls
/// [`ki_records_judged`].
///
/// # Errors
///
/// [`GateError::Io`] naming the root that could not be read.
pub fn ki_records(ctx: &GateCtx, args: &[String]) -> Result<Vec<Utf8PathBuf>, GateError> {
    let mut records = Vec::new();
    for root in ki_record_roots(ctx, args) {
        let entries = match ctx.path(&root).read_dir_utf8() {
            Ok(entries) => entries,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => return Err(GateError::io(&root, source)),
        };
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| GateError::io(&root, source))?;
            if !entry
                .file_type()
                .map_err(|source| GateError::io(&root, source))?
                .is_file()
            {
                continue;
            }
            let name = entry.file_name().to_string();
            if name
                .strip_prefix("KI-")
                .and_then(|rest| rest.strip_suffix(".md"))
                .is_some_and(|slug| !slug.is_empty())
            {
                names.push(name);
            }
        }
        names.sort();
        records.extend(names.into_iter().map(|name| root.join(name)));
    }
    Ok(records)
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
    fn the_plan_zone_resolves_only_what_a_command_may_check() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = ctx(&dir);
        // No record at all.
        assert_eq!(plan_zone_with(&ctx, None), PlanZoneTarget::Unchecked);

        for (recorded, expected) in [
            (
                "{\"kind\": \"tracked\", \"path\": \"docs/plan\"}",
                PlanZoneTarget::Tracked(Utf8PathBuf::from("docs/plan")),
            ),
            // A tracked kind whose path is missing or empty is a broken
            // declaration, never an absent one: reading it as `Unchecked`
            // would skip a zone the project declared gated.
            (
                "{\"kind\": \"tracked\"}",
                PlanZoneTarget::Broken(
                    "the recorded plan zone is tracked and carries no path".to_string(),
                ),
            ),
            (
                "{\"kind\": \"tracked\", \"path\": \"  \"}",
                PlanZoneTarget::Broken(
                    "the recorded plan zone is tracked and its path is empty".to_string(),
                ),
            ),
            (
                "{\"kind\": \"untracked\", \"path\": \"docs/plan\"}",
                PlanZoneTarget::Unchecked,
            ),
            ("{\"kind\": \"env\"}", PlanZoneTarget::Unchecked),
            ("{\"kind\": \"none\"}", PlanZoneTarget::Unchecked),
        ] {
            write(
                &dir,
                ".spec-driven-docs/manifest.json",
                &format!("{{\"plan_zone\": {recorded}}}\n"),
            );
            assert_eq!(plan_zone_with(&ctx, None), expected, "{recorded}");
            // The variable wins over every recorded kind.
            assert_eq!(
                plan_zone_with(&ctx, Some(Utf8PathBuf::from("elsewhere"))),
                PlanZoneTarget::Variable(Utf8PathBuf::from("elsewhere")),
                "{recorded}"
            );
        }
    }

    #[test]
    fn the_docs_scratch_takes_the_variable_then_the_record() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = ctx(&dir);
        assert_eq!(docs_scratch_with(&ctx, None), None);

        write(
            &dir,
            ".spec-driven-docs/manifest.json",
            "{\"docs_scratch\": \"../beside\"}\n",
        );
        assert_eq!(
            docs_scratch_with(&ctx, None),
            Some(Utf8PathBuf::from("../beside"))
        );
        assert_eq!(
            docs_scratch_with(&ctx, Some(Utf8PathBuf::from("inside"))),
            Some(Utf8PathBuf::from("inside"))
        );
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
            ki_records(&ctx(&dir), &[]).unwrap(),
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
            ki_records(&ctx(&dir), &[]).unwrap(),
            vec![
                Utf8PathBuf::from("docs/reference/known-issues/KI-a.md"),
                Utf8PathBuf::from("docs/reference/known-issues/KI-b.md"),
            ]
        );
    }

    #[test]
    fn an_unreadable_layout_is_not_read_as_an_absent_one() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("docs/specs")).unwrap();
        assert_eq!(docs_root(&ctx(&dir)), "docs");

        let specs = dir.path().join("docs/specs");
        let mut mode = std::fs::metadata(&specs).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o000);
        std::fs::set_permissions(dir.path().join("docs"), mode.clone()).unwrap();
        let resolved = docs_root(&ctx(&dir));
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
        std::fs::set_permissions(dir.path().join("docs"), mode).unwrap();
        assert_eq!(
            resolved, "docs",
            "an unreadable layout fell through to the default root"
        );
    }

    #[test]
    fn an_unsearchable_ancestor_is_raised_rather_than_discovered_away() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("docs/reference/known-issues")).unwrap();
        let ancestor = dir.path().join("docs/reference");
        let mut mode = std::fs::metadata(&ancestor).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o000);
        std::fs::set_permissions(&ancestor, mode.clone()).unwrap();
        let raised = ki_records(&ctx(&dir), &[]).is_err();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
        std::fs::set_permissions(&ancestor, mode).unwrap();
        assert!(raised, "an unsearchable ancestor listed as no zone");
    }

    #[test]
    fn an_absent_zone_is_skipped_and_an_unreadable_one_is_raised() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir, "docs/specs/SPEC-a.md", "# A\n");
        assert!(ki_records(&ctx(&dir), &[]).unwrap().is_empty());

        let zone = dir.path().join("docs/reference/known-issues");
        std::fs::create_dir_all(&zone).unwrap();
        let mut mode = std::fs::metadata(&zone).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o000);
        std::fs::set_permissions(&zone, mode.clone()).unwrap();
        let raised = ki_records(&ctx(&dir), &[]).is_err();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
        std::fs::set_permissions(&zone, mode).unwrap();
        assert!(raised, "an unreadable zone listed as empty");
    }
}
