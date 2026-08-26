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

fn skill_dirs() -> Vec<(String, String)> {
    let mut skills = Vec::new();
    for entry in std::fs::read_dir(canon().join("skills")).unwrap() {
        let entry = entry.unwrap();
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_str().unwrap().to_string();
        let text = read(&format!("skills/{name}/SKILL.md"));
        skills.push((name, text));
    }
    assert!(!skills.is_empty(), "no skills authored under skills/");
    skills
}

fn split_frontmatter(text: &str) -> (Vec<&str>, Vec<&str>) {
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("---"), "SKILL.md opens with frontmatter");
    let mut frontmatter = Vec::new();
    for line in lines.by_ref() {
        if line == "---" {
            return (frontmatter, lines.collect());
        }
        frontmatter.push(line);
    }
    panic!("frontmatter never closes");
}

/// SATISFIES distribution:a-skill-obeys-the-portable-format
#[test]
fn every_skill_carries_the_portable_frontmatter_and_stays_within_budget() {
    let allowed = [
        "name",
        "description",
        "license",
        "compatibility",
        "metadata",
        "allowed-tools",
    ];
    for (dir, text) in skill_dirs() {
        let (frontmatter, body) = split_frontmatter(&text);
        let mut fields = std::collections::BTreeMap::new();
        for line in &frontmatter {
            if line.starts_with([' ', '\t']) {
                continue;
            }
            let (key, value) = line
                .split_once(':')
                .unwrap_or_else(|| panic!("{dir}: not a key line: {line}"));
            assert!(
                allowed.contains(&key),
                "{dir}: field '{key}' is not in the portable Agent Skills format"
            );
            let value = value.trim();
            // A YAML plain scalar never opens with '|' or '>', so this
            // rejects every block-scalar header form and nothing valid.
            assert!(
                !value.starts_with(['|', '>']),
                "{dir}: '{key}' uses a block scalar; keep portable values on one plain line"
            );
            assert!(
                fields.insert(key, value).is_none(),
                "{dir}: '{key}' appears twice in the frontmatter"
            );
        }
        assert_eq!(
            fields.get("name"),
            Some(&dir.as_str()),
            "{dir}: name differs"
        );
        assert!(
            dir.len() <= 64
                && !dir.starts_with('-')
                && !dir.ends_with('-')
                && !dir.contains("--")
                && dir
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
            "{dir}: name breaks the spec grammar"
        );
        let description = fields.get("description").copied().unwrap_or_default();
        assert!(
            !description.is_empty() && description.len() <= 1024,
            "{dir}: description is empty or over 1024 characters"
        );
        if let Some(compatibility) = fields.get("compatibility") {
            assert!(
                !compatibility.is_empty() && compatibility.len() <= 500,
                "{dir}: compatibility is empty or over 500 characters"
            );
        }
        assert!(
            body.len() <= 150,
            "{dir}: body is {} lines, budget is 150",
            body.len()
        );
    }
}

/// SATISFIES distribution:skills-are-part-of-the-payload
#[test]
fn every_sdd_invocation_a_skill_cites_names_a_real_subcommand() {
    let subcommands = spec_driven_docs::cli::subcommand_names();
    for (dir, text) in skill_dirs() {
        let mut fenced = false;
        for line in text.lines() {
            if line.starts_with("```") {
                fenced = !fenced;
                continue;
            }
            for (index, _) in line.match_indices("sdd ") {
                // Only a command position counts: a fenced line opening
                // with it, or a code span opening with it.
                let opens_fenced_line = fenced && index == 0;
                let opens_code_span = index > 0 && line.as_bytes()[index - 1] == b'`';
                if !opens_fenced_line && !opens_code_span {
                    continue;
                }
                let word: String = line[index + 4..]
                    .chars()
                    .take_while(|c| c.is_ascii_lowercase() || *c == '-')
                    .collect();
                if word.is_empty() {
                    continue;
                }
                assert!(
                    subcommands.contains(&word),
                    "{dir}: cites unknown subcommand 'sdd {word}' in: {line}"
                );
            }
        }
    }
}

/// SATISFIES distribution:skills-are-part-of-the-payload
#[test]
fn the_sdd_docs_skill_quotes_the_agents_snippet_verbatim() {
    let snippet = read("instance/snippets/AGENTS-docs.md");
    let skill = read("skills/sdd-docs/SKILL.md");
    assert!(
        skill.contains(snippet.trim_end()),
        "skills/sdd-docs/SKILL.md no longer quotes instance/snippets/AGENTS-docs.md"
    );
}
