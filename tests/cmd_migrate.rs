//! Integration: `sdd migrate` archives a corpus, tracks its rebuild, and
//! closes on a declaration a greenfield landing would have written.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "the panic is this suite's failure signal, not control flow"
)]

mod support;

use std::path::{Path, PathBuf};

use support::Fixture;

/// Run git in the fixture alone.
fn git(fixture: &Fixture, args: &[&str]) {
    let out = support::git(fixture.path()).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?} failed");
}

/// A fixture holding a decision corpus and root metadata, as one commit.
fn corpus() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("README.md", "# Demo\n");
    fixture.write("docs/adr/0003-use-postgres.md", "# Use Postgres\n");
    fixture.write("docs/adr/0007-cache-reads.md", "# Cache reads\n");
    fixture.write("docs/adr/0012-shard-writes.md", "# Shard writes\n");
    fixture.write("docs/intro.md", "# Intro\n");
    std::fs::remove_dir_all(fixture.path().join(".git")).unwrap();
    git(&fixture, &["init", "-q"]);
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "x"]);
    // The remote's HEAD names the trunk, as a clone of a forge would.
    git(&fixture, &["branch", "-M", "trunk"]);
    git(
        &fixture,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/trunk",
        ],
    );
    fixture
}

/// The registry of the open migration under `workspace`.
fn registry(workspace: &Path) -> serde_json::Value {
    yaml_serde::from_str(&std::fs::read_to_string(workspace.join("registry.yaml")).unwrap())
        .unwrap()
}

/// Every source id the registry under `workspace` holds, by path.
fn source_ids(workspace: &Path) -> Vec<(String, String)> {
    registry(workspace)["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|source| {
            (
                source["path"].as_str().unwrap().to_string(),
                source["id"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// Assert `archive --apply` refuses with `code`, naming `path`, and that
/// nothing in the target or the state root changed.
fn refuses_the_archive(fixture: &Fixture, extra: &[&str], path: &str) {
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["migrate", "archive", "--apply"])
        .args(extra)
        .assert()
        .code(73)
        .stderr(predicates::str::contains(path));
    assert_eq!(digest, fixture.tree_digest());
    assert!(
        !fixture
            .state_root()
            .join("spec-driven-docs/migrations")
            .exists()
    );
}

/// One `sdd migrate` verb's JSON report, asserting success.
fn json(fixture: &Fixture, args: &[&str]) -> serde_json::Value {
    let out = fixture
        .cmd()
        .current_dir(fixture.path())
        .arg("migrate")
        .args(args)
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&out).unwrap()
}

/// Every file under `dir`, relative to it, sorted.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| !entry.path().starts_with(dir.join(".git")))
        .map(|entry| entry.path().strip_prefix(dir).unwrap().to_path_buf())
        .collect();
    found.sort();
    found
}

/// The classification `sdd status` reports for the fixture.
fn classification(fixture: &Fixture) -> String {
    let out = fixture
        .cmd()
        .args(["status", "--target", &fixture.target(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&out).unwrap();
    report["classification"].as_str().unwrap().to_string()
}

#[test]
fn inventory_lists_each_document_with_its_kind_and_placement() {
    let fixture = corpus();
    let report = json(&fixture, &["inventory"]);
    let documents = report["documents"].as_array().unwrap();
    assert_eq!(documents.len(), 5);
    let readme = documents
        .iter()
        .find(|entry| entry["path"] == "README.md")
        .unwrap();
    assert_eq!(readme["kind"], "metadata");
    assert_eq!(readme["placement"], "in-place");
    let record = documents
        .iter()
        .find(|entry| entry["path"] == "docs/adr/0003-use-postgres.md")
        .unwrap();
    assert_eq!(record["kind"], "decision");
    assert_eq!(record["placement"], "archived");
}

/// VERIFIES setup:an-archive-moves-only-committed-documents
#[test]
fn archive_refuses_an_uncommitted_document_and_writes_nothing() {
    let fixture = corpus();
    fixture.write("docs/adr/0007-cache-reads.md", "# Cache reads, edited\n");
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["migrate", "archive", "--apply"])
        .assert()
        .code(73)
        .stderr(predicates::str::contains("docs/adr/0007-cache-reads.md"));
    assert_eq!(digest, fixture.tree_digest());
    assert!(
        !fixture
            .state_root()
            .join("spec-driven-docs/migrations")
            .exists()
    );
}

/// VERIFIES setup:an-archive-moves-only-committed-documents
#[test]
fn an_ignored_document_refuses_the_archive() {
    let fixture = corpus();
    fixture.write(".gitignore", "docs/private.md\n");
    git(&fixture, &["add", ".gitignore"]);
    git(&fixture, &["commit", "-q", "-m", "ignore"]);
    fixture.write("docs/private.md", "# Private\n");
    refuses_the_archive(&fixture, &[], "docs/private.md");
}

/// VERIFIES setup:an-archive-moves-only-committed-documents
#[test]
fn a_document_that_differs_from_the_trunk_refuses_the_archive() {
    let fixture = corpus();
    git(&fixture, &["checkout", "-q", "-b", "work"]);
    fixture.write("docs/intro.md", "# Intro, edited on a branch\n");
    git(&fixture, &["commit", "-q", "-am", "edit"]);
    refuses_the_archive(&fixture, &[], "docs/intro.md");
    // Named explicitly, the branch is the trunk the bytes must match.
    json(&fixture, &["archive", "--trunk", "work", "--apply"]);
}

/// VERIFIES setup:an-archive-moves-only-committed-documents
#[test]
fn a_document_deleted_off_the_trunk_refuses_the_archive() {
    let fixture = corpus();
    git(&fixture, &["checkout", "-q", "-b", "work"]);
    git(&fixture, &["rm", "-q", "docs/intro.md"]);
    git(&fixture, &["commit", "-q", "-m", "delete"]);
    refuses_the_archive(&fixture, &[], "docs/intro.md");
}

/// VERIFIES setup:an-archive-moves-only-committed-documents
#[test]
fn a_checkout_that_converts_line_endings_refuses_the_archive() {
    let fixture = corpus();
    fixture.write(".gitattributes", "*.md text eol=crlf\n");
    git(&fixture, &["add", ".gitattributes"]);
    git(&fixture, &["commit", "-q", "-m", "attributes"]);
    std::fs::remove_file(fixture.path().join("docs/intro.md")).unwrap();
    git(&fixture, &["checkout", "--", "docs/intro.md"]);
    assert_eq!(fixture.read("docs/intro.md"), "# Intro\r\n");
    refuses_the_archive(&fixture, &[], "docs/intro.md");
}

/// VERIFIES setup:an-archive-moves-only-committed-documents
#[test]
fn an_archive_with_no_trunk_to_compare_refuses() {
    let fixture = corpus();
    git(
        &fixture,
        &["symbolic-ref", "--delete", "refs/remotes/origin/HEAD"],
    );
    refuses_the_archive(&fixture, &[], "--trunk");
    refuses_the_archive(&fixture, &["--trunk", "nowhere"], "nowhere");
}

#[test]
fn a_linked_instance_directory_refuses_the_archive() {
    let fixture = corpus();
    let elsewhere = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), fixture.path().join(".spec-driven-docs")).unwrap();
    refuses_the_archive(&fixture, &[], ".spec-driven-docs/config.yaml");
    assert_eq!(std::fs::read_dir(elsewhere.path()).unwrap().count(), 0);
}

#[test]
fn colliding_source_paths_take_distinct_ids() {
    let fixture = corpus();
    fixture.write("docs/a/b.md", "# Nested\n");
    fixture.write("docs/a-b.md", "# Flat\n");
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "collide"]);
    let report = json(&fixture, &["archive", "--apply"]);
    let ids = source_ids(Path::new(report["workspace"].as_str().unwrap()));
    let id_of = |path: &str| {
        ids.iter()
            .find(|(listed, _)| listed == path)
            .unwrap()
            .1
            .clone()
    };
    // Byte order puts `-` before `/`, so the flat path keeps the slug.
    assert_eq!(id_of("docs/a-b.md"), "docs-a-b-md");
    assert_eq!(id_of("docs/a/b.md"), "docs-a-b-md-2");
}

/// VERIFIES setup:an-open-migration-is-declared
#[test]
fn archive_leaves_only_root_metadata_and_the_registry_matches_the_bytes() {
    let fixture = corpus();
    let preview = json(&fixture, &["archive"]);
    assert_eq!(preview["applied"], false);
    assert!(fixture.path().join("docs/intro.md").is_file());

    let report = json(&fixture, &["archive", "--apply"]);
    let workspace = PathBuf::from(report["workspace"].as_str().unwrap());
    assert_eq!(
        files(fixture.path()),
        [
            PathBuf::from(".spec-driven-docs/config.yaml"),
            PathBuf::from("README.md")
        ]
    );
    let registry = registry(&workspace);
    for source in registry["sources"].as_array().unwrap() {
        let path = source["path"].as_str().unwrap();
        let held = if source["placement"] == "archived" {
            workspace.join("sources").join(path)
        } else {
            fixture.path().join(path)
        };
        let digest = spec_driven_docs::domain::ownership::Sha256::of(&std::fs::read(held).unwrap());
        assert_eq!(
            source["sha256"].as_str().unwrap(),
            digest.to_string(),
            "{path}"
        );
        assert_eq!(source["status"], "pending");
    }
    assert!(
        fixture
            .read(".spec-driven-docs/config.yaml")
            .contains(report["id"].as_str().unwrap())
    );
    assert_eq!(classification(&fixture), "migration");
}

/// VERIFIES setup:an-archive-rebuilds-from-git
#[test]
fn rebuild_recreates_the_same_bytes_from_git_with_or_without_a_registry() {
    let fixture = corpus();
    let report = json(&fixture, &["archive", "--apply"]);
    let workspace = PathBuf::from(report["workspace"].as_str().unwrap());
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "archive"]);
    let before: Vec<(PathBuf, Vec<u8>)> = files(&workspace.join("sources"))
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(workspace.join("sources").join(&path)).unwrap();
            (path, bytes)
        })
        .collect();
    assert_eq!(before.len(), 4);

    std::fs::remove_dir_all(workspace.join("sources")).unwrap();
    json(&fixture, &["archive", "--rebuild", "--apply"]);
    for (path, bytes) in &before {
        assert_eq!(
            &std::fs::read(workspace.join("sources").join(path)).unwrap(),
            bytes
        );
    }

    // A clone that never held the workspace finds the base commit from
    // the commit that recorded the migration.
    std::fs::remove_dir_all(&workspace).unwrap();
    let rebuilt = json(&fixture, &["archive", "--rebuild", "--apply"]);
    assert_eq!(rebuilt["base_commit"], report["base_commit"]);
    for (path, bytes) in &before {
        assert_eq!(
            &std::fs::read(workspace.join("sources").join(path)).unwrap(),
            bytes
        );
    }
    assert!(workspace.join("registry.yaml").is_file());
}

/// VERIFIES setup:an-archive-rebuilds-from-git
#[test]
fn a_rebuild_after_a_squash_merged_branch_restores_the_archived_bytes() {
    let fixture = corpus();
    git(&fixture, &["checkout", "-q", "-b", "migrate"]);
    let report = json(&fixture, &["archive", "--apply"]);
    let workspace = PathBuf::from(report["workspace"].as_str().unwrap());
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "archive"]);
    let before: Vec<(PathBuf, Vec<u8>)> = files(&workspace.join("sources"))
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(workspace.join("sources").join(&path)).unwrap();
            (path, bytes)
        })
        .collect();

    // The branch lands as one squash commit and leaves no trace behind.
    git(&fixture, &["checkout", "-q", "trunk"]);
    git(&fixture, &["merge", "-q", "--squash", "migrate"]);
    git(&fixture, &["commit", "-q", "-m", "squash"]);
    git(&fixture, &["branch", "-q", "-D", "migrate"]);
    git(&fixture, &["reflog", "expire", "--expire=now", "--all"]);
    git(&fixture, &["gc", "-q", "--prune=now"]);

    for lost in [workspace.join("sources"), workspace.clone()] {
        std::fs::remove_dir_all(&lost).unwrap();
        json(&fixture, &["archive", "--rebuild", "--apply"]);
        for (path, bytes) in &before {
            assert_eq!(
                &std::fs::read(workspace.join("sources").join(path)).unwrap(),
                bytes
            );
        }
    }
}

/// VERIFIES setup:an-archive-rebuilds-from-git
#[test]
fn a_key_first_recorded_in_a_merge_commit_is_found() {
    let fixture = corpus();
    git(&fixture, &["checkout", "-q", "-b", "side"]);
    fixture.write("src.txt", "side work\n");
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "side"]);
    git(&fixture, &["checkout", "-q", "trunk"]);
    git(&fixture, &["merge", "-q", "--no-ff", "--no-commit", "side"]);
    // The merge commit itself records the migration.
    let report = json(&fixture, &["archive", "--apply"]);
    let workspace = PathBuf::from(report["workspace"].as_str().unwrap());
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "merge and archive"]);
    let before = files(&workspace.join("sources"));

    std::fs::remove_dir_all(&workspace).unwrap();
    let rebuilt = json(&fixture, &["archive", "--rebuild", "--apply"]);
    assert_eq!(rebuilt["base_commit"], report["base_commit"]);
    assert_eq!(files(&workspace.join("sources")), before);
}

/// VERIFIES setup:an-archive-rebuilds-from-git
#[test]
fn a_key_recorded_on_a_merged_branch_takes_the_trunks_bytes_at_the_merge() {
    let fixture = corpus();
    git(&fixture, &["checkout", "-q", "-b", "migrate"]);
    let report = json(&fixture, &["archive", "--apply"]);
    let workspace = PathBuf::from(report["workspace"].as_str().unwrap());
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "archive"]);
    // The trunk edits a document while the branch is open, and the merge
    // keeps the branch's archive of it.
    git(&fixture, &["checkout", "-q", "trunk"]);
    fixture.write("docs/intro.md", "# Intro, edited on the trunk\n");
    git(&fixture, &["commit", "-q", "-am", "edit"]);
    let _ = support::git(fixture.path())
        .args(["merge", "-q", "--no-ff", "--no-commit", "migrate"])
        .output()
        .unwrap();
    git(&fixture, &["rm", "-q", "docs/intro.md"]);
    git(&fixture, &["commit", "-q", "-m", "merge"]);

    std::fs::remove_dir_all(&workspace).unwrap();
    json(&fixture, &["archive", "--rebuild", "--apply"]);
    assert_eq!(
        std::fs::read_to_string(workspace.join("sources/docs/intro.md")).unwrap(),
        "# Intro, edited on the trunk\n"
    );
}

/// VERIFIES setup:an-archive-rebuilds-from-git
#[test]
fn a_rebuild_finds_the_key_line_and_not_an_earlier_mention_of_the_id() {
    let fixture = corpus();
    let id = "20261009T213000Z";
    // An earlier commit names the id in a comment and edits a document.
    fixture.write(
        ".spec-driven-docs/config.yaml",
        &format!("# {id} was planned\n"),
    );
    fixture.write("docs/intro.md", "# Intro, edited\n");
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "plan"]);
    // The commit that records the migration removes the corpus.
    fixture.write(
        ".spec-driven-docs/config.yaml",
        &format!("migration: '{id}'\n"),
    );
    for path in [
        "docs/adr/0003-use-postgres.md",
        "docs/adr/0007-cache-reads.md",
        "docs/adr/0012-shard-writes.md",
        "docs/intro.md",
    ] {
        git(&fixture, &["rm", "-q", path]);
    }
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "archive"]);

    let rebuilt = json(&fixture, &["archive", "--rebuild", "--apply"]);
    let workspace = PathBuf::from(rebuilt["workspace"].as_str().unwrap());
    assert_eq!(
        std::fs::read_to_string(workspace.join("sources/docs/intro.md")).unwrap(),
        "# Intro, edited\n"
    );
}

/// VERIFIES setup:an-archive-rebuilds-from-git
#[test]
fn a_rebuild_with_its_registry_recreates_its_sources_alone() {
    let fixture = corpus();
    // The base commit holds a draft the archive never takes.
    fixture.write(".docs-scratch/draft.md", "# Draft\n");
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "draft"]);
    let report = json(&fixture, &["archive", "--apply"]);
    let workspace = PathBuf::from(report["workspace"].as_str().unwrap());
    let taken = files(&workspace.join("sources"));
    std::fs::remove_dir_all(workspace.join("sources")).unwrap();
    json(&fixture, &["archive", "--rebuild", "--apply"]);
    assert_eq!(files(&workspace.join("sources")), taken);
}

/// VERIFIES setup:an-archive-rebuilds-from-git
#[test]
fn a_rebuild_without_a_registry_selects_the_sources_the_archive_took() {
    let fixture = corpus();
    fixture.write(".docs-scratch/draft.md", "# Draft\n");
    std::os::unix::fs::symlink("intro.md", fixture.path().join("docs/link.md")).unwrap();
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "scratch and link"]);
    let report = json(&fixture, &["archive", "--apply"]);
    let workspace = PathBuf::from(report["workspace"].as_str().unwrap());
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "archive"]);
    let taken = source_ids(&workspace);
    assert!(
        taken
            .iter()
            .all(|(path, _)| path != "docs/link.md" && !path.starts_with(".docs-scratch"))
    );

    std::fs::remove_dir_all(&workspace).unwrap();
    json(&fixture, &["archive", "--rebuild", "--apply"]);
    assert_eq!(source_ids(&workspace), taken);
}

/// The declaration a greenfield landing of the same choices writes.
fn greenfield_declaration() -> String {
    let fixture = Fixture::new();
    fixture.write("README.md", "# Demo\n");
    let stage = json_stage(&fixture);
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["init", "--stage", &stage, "--apply"])
        .assert()
        .success();
    fixture.read(".spec-driven-docs/config.yaml")
}

/// Stage the fixture with the choices both landings take, and return the
/// stage root.
fn json_stage(fixture: &Fixture) -> String {
    let out = fixture
        .cmd()
        .args([
            "stage",
            "--target",
            &fixture.target(),
            "--profile",
            "codebase",
            "--docs-scratch",
            "none",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let receipt: serde_json::Value = serde_json::from_slice(&out).unwrap();
    receipt["root"].as_str().unwrap().to_string()
}

/// VERIFIES setup:close-refuses-an-unfinished-source
/// VERIFIES setup:mark-appends-and-never-rewrites
#[test]
fn a_tagged_group_rebuilds_into_one_record_and_the_close_leaves_a_greenfield_declaration() {
    let fixture = corpus();
    json(&fixture, &["archive", "--apply"]);
    let stage = json_stage(&fixture);
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["init", "--stage", &stage, "--apply"])
        .assert()
        .success();
    assert_eq!(classification(&fixture), "migration");

    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["migrate", "close"])
        .assert()
        .code(73)
        .stderr(predicates::str::contains("pending"));

    let records = [
        "docs-adr-0003-use-postgres-md",
        "docs-adr-0007-cache-reads-md",
        "docs-adr-0012-shard-writes-md",
    ];
    for record in records {
        json(
            &fixture,
            &[
                "mark", record, "mapped", "--group", "storage", "--tag", "storage",
            ],
        );
    }
    fixture.write(
        "docs/decisions/ADR-store-state-in-postgres.md",
        "# Store state in Postgres\n\n## Considered Options\n\n- `use postgres` (0003)\n- `cache reads` (0007)\n- `shard writes` (0012)\n",
    );
    json(
        &fixture,
        &[
            "mark",
            "storage",
            "verified",
            "--destination",
            "docs/decisions/ADR-store-state-in-postgres.md",
        ],
    );
    json(
        &fixture,
        &[
            "mark",
            "docs/intro.md",
            "dropped",
            "--reason",
            "the README says it",
        ],
    );
    json(
        &fixture,
        &["mark", "readme-md", "verified", "--group", "readme"],
    );
    json(
        &fixture,
        &["mark", "readme", "verified", "--destination", "README.md"],
    );

    let status = json(&fixture, &["status"]);
    assert_eq!(status["unfinished"].as_array().unwrap().len(), 0);
    let record = status["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["id"] == records[0])
        .unwrap()
        .clone();
    assert_eq!(record["group"], "storage");

    json(&fixture, &["close", "--apply"]);
    assert_eq!(classification(&fixture), "current");
    assert_eq!(
        fixture.read(".spec-driven-docs/config.yaml"),
        greenfield_declaration()
    );
    fixture
        .cmd()
        .args(["verify", "--target", &fixture.target()])
        .assert()
        .success();
    let merged = fixture.read("docs/decisions/ADR-store-state-in-postgres.md");
    for number in ["0003", "0007", "0012"] {
        assert!(
            merged.contains(number),
            "the merged record does not consider {number}"
        );
    }
}

#[test]
fn a_destination_outside_the_target_refuses() {
    let fixture = corpus();
    let report = json(&fixture, &["archive", "--apply"]);
    let workspace = PathBuf::from(report["workspace"].as_str().unwrap());
    json(
        &fixture,
        &["mark", "docs/intro.md", "mapped", "--group", "intro"],
    );
    let before = std::fs::read(workspace.join("registry.yaml")).unwrap();
    for outside in ["../elsewhere.md", "/etc/hostname", "docs/../../x.md", ""] {
        fixture
            .cmd()
            .current_dir(fixture.path())
            .args([
                "migrate",
                "mark",
                "intro",
                "mapped",
                "--destination",
                outside,
            ])
            .assert()
            .code(64)
            .stderr(predicates::str::contains("leaves the target"));
    }
    assert_eq!(
        std::fs::read(workspace.join("registry.yaml")).unwrap(),
        before
    );
}

/// VERIFIES setup:close-refuses-an-unfinished-source
#[test]
fn a_destination_reached_through_a_symlink_out_of_the_target_does_not_count() {
    let fixture = corpus();
    json(&fixture, &["archive", "--apply"]);
    for source in [
        "readme-md",
        "docs-adr-0003-use-postgres-md",
        "docs-adr-0007-cache-reads-md",
        "docs-adr-0012-shard-writes-md",
    ] {
        json(
            &fixture,
            &["mark", source, "dropped", "--reason", "covered"],
        );
    }
    json(
        &fixture,
        &["mark", "docs/intro.md", "verified", "--group", "intro"],
    );
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(elsewhere.path().join("intro.md"), "# Intro\n").unwrap();
    std::fs::create_dir_all(fixture.path().join("docs")).unwrap();
    std::os::unix::fs::symlink(
        elsewhere.path().join("intro.md"),
        fixture.path().join("docs/out.md"),
    )
    .unwrap();
    json(
        &fixture,
        &["mark", "intro", "verified", "--destination", "docs/out.md"],
    );
    let status = json(&fixture, &["status"]);
    assert!(
        status["unfinished"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason.as_str().unwrap().contains("docs/out.md"))
    );
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["migrate", "close"])
        .assert()
        .code(73)
        .stderr(predicates::str::contains("docs/out.md"));
}

#[test]
fn status_reports_an_unreadable_copy_instead_of_calling_it_changed() {
    let fixture = corpus();
    let report = json(&fixture, &["archive", "--apply"]);
    // A directory where the copy belongs cannot be read as one, whoever
    // runs the test.
    let copy = PathBuf::from(report["workspace"].as_str().unwrap()).join("sources/docs/intro.md");
    std::fs::remove_file(&copy).unwrap();
    std::fs::create_dir(&copy).unwrap();
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["migrate", "status", "--json"])
        .assert()
        .code(74)
        .stderr(predicates::str::contains("directory"));
}

/// VERIFIES setup:an-incremental-landing-is-named
#[test]
fn an_incremental_landing_keeps_the_corpus_and_records_its_debt() {
    let fixture = corpus();
    let digest = "- one line of the digest\n".repeat(150);
    fixture.write("AGENTS.md", &format!("# Agents\n\n{digest}"));
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "agents"]);
    let corpus_files = files(fixture.path());
    let stage = json_stage(&fixture);

    // The plain landing still refuses the corpus.
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["init", "--stage", &stage, "--apply"])
        .assert()
        .code(73);
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["init", "--stage", &stage, "--incremental", "--apply"])
        .assert()
        .success();
    let landed = files(fixture.path());
    for path in &corpus_files {
        assert!(landed.contains(path), "{} moved", path.display());
    }
    assert!(
        fixture
            .path()
            .join(".spec-driven-docs/manifest.json")
            .is_file()
    );
    assert!(
        fixture
            .read(".spec-driven-docs/debt.yaml")
            .contains("AGENTS.md")
    );
}

/// VERIFIES setup:an-incremental-landing-is-named
#[test]
fn an_incremental_landing_measures_the_root_it_lands() {
    let fixture = corpus();
    // Both roots exist. Discovery would read `_docs`, and the codebase
    // profile lands `docs`.
    fixture.write("_docs/specs/SPEC-small.md", "# Small\n");
    let words = "word ".repeat(400);
    fixture.write(
        "docs/decisions/ADR-long.md",
        &format!("# Long\n\n{words}\n"),
    );
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "two roots"]);
    let stage = json_stage(&fixture);
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["init", "--stage", &stage, "--incremental", "--apply"])
        .assert()
        .success();
    assert!(
        fixture
            .read(".spec-driven-docs/debt.yaml")
            .contains("docs/decisions/ADR-long.md")
    );
}

/// VERIFIES setup:an-incremental-landing-is-named
#[test]
fn an_incremental_landing_measures_every_reservation_it_lands() {
    let fixture = corpus();
    // Over the 150 lines a nested digest may hold.
    let digest = "- one line of the digest\n".repeat(200);
    fixture.write("kept/AGENTS.md", &digest);
    fixture.write("third/AGENTS.md", &digest);
    fixture.write(".spec-driven-docs/config.yaml", "reserved:\n  - kept/**\n");
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "reserved digests"]);
    // The flags name one more reservation, which the landing adds to the
    // one the declaration records.
    fixture
        .cmd()
        .args([
            "init",
            "--target",
            &fixture.target(),
            "--profile",
            "codebase",
            "--reserve",
            "third/**",
            "--incremental",
            "--apply",
        ])
        .assert()
        .success();
    let declaration = fixture.read(".spec-driven-docs/config.yaml");
    assert!(declaration.contains("kept/**") && declaration.contains("third/**"));
    let debt = std::fs::read_to_string(fixture.path().join(".spec-driven-docs/debt.yaml"))
        .unwrap_or_default();
    assert!(!debt.contains("kept/AGENTS.md"), "{debt}");
    assert!(!debt.contains("third/AGENTS.md"), "{debt}");
}

/// VERIFIES setup:an-incremental-landing-is-named
#[test]
fn an_incremental_landing_refuses_debt_it_did_not_measure() {
    let fixture = corpus();
    fixture.write("AGENTS.md", &"- one line of the digest\n".repeat(150));
    fixture.write(".spec-driven-docs/debt.yaml", "{}\n");
    git(&fixture, &["add", "-A"]);
    git(&fixture, &["commit", "-q", "-m", "debt"]);
    let stage = json_stage(&fixture);
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["init", "--stage", &stage, "--incremental", "--apply"])
        .assert()
        .code(73)
        .stderr(predicates::str::contains(
            ".spec-driven-docs/debt.yaml already records debt",
        ));
    assert_eq!(digest, fixture.tree_digest());
}

/// VERIFIES setup:an-incremental-landing-is-named
#[test]
fn an_incremental_landing_that_cannot_measure_its_debt_lands_nothing() {
    let fixture = corpus();
    std::fs::write(fixture.path().join("docs/AGENTS.md"), [0xff, 0xfe, b'\n']).unwrap();
    git(&fixture, &["add", "-A"]);
    git(
        &fixture,
        &["commit", "-q", "-m", "a digest no gate can read"],
    );
    let stage = json_stage(&fixture);
    let digest = fixture.tree_digest();
    fixture
        .cmd()
        .current_dir(fixture.path())
        .args(["init", "--stage", &stage, "--incremental", "--apply"])
        .assert()
        .failure();
    assert_eq!(digest, fixture.tree_digest());
    assert!(
        !fixture
            .path()
            .join(".spec-driven-docs/manifest.json")
            .exists()
    );
}

/// VERIFIES setup:an-incremental-landing-is-named
#[test]
fn an_incremental_landing_refuses_any_other_target() {
    let greenfield = Fixture::new();
    greenfield.write("README.md", "# Demo\n");
    let opened = corpus();
    json(&opened, &["archive", "--apply"]);
    for fixture in [&greenfield, &opened] {
        let stage = json_stage(fixture);
        let digest = fixture.tree_digest();
        fixture
            .cmd()
            .current_dir(fixture.path())
            .args(["init", "--stage", &stage, "--incremental", "--apply"])
            .assert()
            .code(73)
            .stderr(predicates::str::contains(
                "setup:an-incremental-landing-is-named",
            ));
        assert_eq!(digest, fixture.tree_digest());
    }
}

/// VERIFIES setup:an-open-migration-is-declared
#[test]
fn a_declared_id_that_is_not_a_migration_id_is_an_invalid_declaration() {
    let fixture = Fixture::new();
    fixture.write(".spec-driven-docs/config.yaml", "migration: '../x'\n");
    let out = fixture
        .cmd()
        .args(["status", "--target", &fixture.target(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(report["classification"], "invalid");
    assert!(
        report["invalid"]
            .as_str()
            .unwrap()
            .contains("setup:an-open-migration-is-declared")
    );
    assert!(report["migration"].is_null());
}
