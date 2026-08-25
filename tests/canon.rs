//! Canon invariants: checks only this repository has.
//!
//! These never reach an instance — an instance holding them would be gated
//! on a release process it does not run. The delivered-set drift check
//! lives in `cmd_hooks.rs`; here live the license split and the version
//! alignment.

// Integration tests: assertion style is the point, so the production
// restrictions on unwrap/panic and string building do not apply here.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect,
    clippy::case_sensitive_file_extension_comparisons
)]

use std::path::{Path, PathBuf};

fn canon() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(canon().join(relative))
        .unwrap_or_else(|_| panic!("{relative} is missing"))
}

/// SATISFIES release:license-declares-both-halves
#[test]
fn the_license_declares_both_halves_and_the_crate_agrees() {
    let license = read("LICENSE");
    for half in ["LICENSE-MIT", "LICENSE-CC-BY-4.0"] {
        assert!(license.contains(half), "LICENSE does not name {half}");
        assert!(!read(half).is_empty(), "{half} is empty");
    }
    assert!(
        license.contains("MIT AND CC-BY-4.0"),
        "LICENSE lost its SPDX expression"
    );
    assert!(
        read("Cargo.toml").contains("license = \"MIT AND CC-BY-4.0\""),
        "Cargo.toml license disagrees with LICENSE"
    );
}

/// SATISFIES release:versions-are-semantic-and-aligned
#[test]
fn the_canon_manifest_carries_this_crate_version() {
    let manifest: serde_json::Value =
        serde_json::from_str(&read(".spec-driven-docs/manifest.json")).unwrap();
    assert_eq!(
        manifest["schema_version"], 2,
        "regenerate with 'sdd self-manifest'"
    );
    assert_eq!(
        manifest["canon_version"],
        env!("CARGO_PKG_VERSION"),
        "regenerate with 'sdd self-manifest'"
    );
}

/// SATISFIES release:a-canon-gate-is-not-delivered
#[test]
fn the_published_manifest_carries_no_canon_check() {
    let published = read(".pre-commit-hooks.yaml");
    for canon_only in ["cargo-test", "cargo-clippy", "cargo-fmt", "self-manifest"] {
        assert!(
            !published.contains(&format!("- id: {canon_only}")),
            "{canon_only} is a canon-side check and must not be published"
        );
    }
}

fn walk_markdown(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if name != "target" && name != ".git" && name != "node_modules" {
                walk_markdown(&path, files);
            }
        } else if path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
}

/// SATISFIES spec-to-code:a-gate-message-cites-the-rule
#[test]
fn every_spec_defined_rule_cited_by_the_registry_resolves() {
    let mut markdown = Vec::new();
    walk_markdown(&canon().join("_docs/specs"), &mut markdown);
    let mut defined = std::collections::BTreeSet::new();
    for path in markdown {
        let text = std::fs::read_to_string(path).unwrap();
        defined.extend(spec_driven_docs::embedded::rule_ids_in(&text));
    }
    for gate in spec_driven_docs::gates::GATES {
        for rule in gate.cites {
            assert!(
                defined.contains(rule.as_str()),
                "{rule} resolves to no requirement on disk"
            );
        }
    }
}
