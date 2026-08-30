//! Canon invariants: checks only this repository has.
//!
//! These never reach an instance — an instance holding them would be gated
//! on a release process it does not run. The delivered-set wiring check
//! lives in `cmd_hooks.rs`; here live the license split, the version
//! alignment, the boundary keeping a canon check out of the delivery, and
//! the two obligations this repository carries because it is an instance of
//! itself: its record is regenerated rather than owned, and its managed
//! block is hand-maintained rather than installed.

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
///
/// The block an instance receives is rendered at install time and committed
/// nowhere, so the render itself is what this reads.
#[test]
fn the_delivered_block_carries_no_canon_check() {
    use spec_driven_docs::services::hooks_render::{RenderOptions, render_block};

    let delivered = render_block(&RenderOptions::default());
    for canon_only in ["cargo-test", "cargo-clippy", "cargo-fmt", "self-manifest"] {
        assert!(
            !delivered.contains(&format!("- id: {canon_only}")),
            "{canon_only} is a canon-side check and must not be delivered"
        );
    }
}

fn digest(relative: &str) -> String {
    let bytes = std::fs::read(canon().join(relative))
        .unwrap_or_else(|_| panic!("{relative} is recorded but missing"));
    spec_driven_docs::domain::ownership::Sha256::of(&bytes).to_string()
}

fn recorded_manifest() -> serde_json::Value {
    serde_json::from_str(&read(".spec-driven-docs/manifest.json")).unwrap()
}

fn recorded_destinations(manifest: &serde_json::Value, class: &str) -> Vec<String> {
    manifest[class]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["destination"].as_str().unwrap().to_string())
        .collect()
}

/// SATISFIES release:the-canon-record-describes-its-tree
///
/// `sdd verify` reports an adopted edit as a note rather than a failure,
/// which is right for an instance that owns its specs and wrong here: this
/// record is generated from the tree, so a difference means `sdd
/// self-manifest` was not run.
#[test]
fn the_canon_record_hashes_every_file_the_tree_carries() {
    let manifest = recorded_manifest();
    for class in ["managed_files", "adopted_files"] {
        for entry in manifest[class].as_array().unwrap() {
            let destination = entry["destination"].as_str().unwrap();
            assert_eq!(
                entry["sha256"].as_str().unwrap(),
                digest(destination),
                "{destination} differs from its record; run 'just manifest'"
            );
        }
    }

    let mut specs: Vec<String> = std::fs::read_dir(canon().join("_docs/specs"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_str().unwrap().to_string())
        .filter(|name| name.starts_with("SPEC-") && name.ends_with(".md"))
        .map(|name| format!("_docs/specs/{name}"))
        .collect();
    specs.sort();
    let adopted = recorded_destinations(&manifest, "adopted_files");
    for spec in &specs {
        assert!(
            adopted.contains(spec),
            "{spec} is on disk but absent from the record; run 'just manifest'"
        );
    }
    for destination in &adopted {
        if destination.starts_with("_docs/specs/") {
            assert!(
                specs.contains(destination),
                "{destination} is recorded but gone from the tree; run 'just manifest'"
            );
        }
    }

    let block = manifest["integration_blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == ".pre-commit-config.yaml")
        .expect("no integration block recorded for .pre-commit-config.yaml");
    let hashed = spec_driven_docs::domain::marker::block_hash(&read(".pre-commit-config.yaml"))
        .expect("no managed block in .pre-commit-config.yaml");
    assert_eq!(
        block["marker_hash"].as_str().unwrap(),
        hashed.to_string(),
        "the managed block differs from its record; run 'just manifest'"
    );
}

/// SATISFIES release:the-delivered-gate-set-is-declared-once
///
/// This repository is an instance of itself, but the one whose managed block
/// is maintained by hand rather than rendered by an installer, so nothing but
/// this holds that copy to the registry it is a copy of.
#[test]
fn the_canon_managed_block_wires_every_registered_gate() {
    let (_, block) =
        spec_driven_docs::domain::marker::split_block(&read(".pre-commit-config.yaml"))
            .expect("malformed managed markers");
    let block = block.expect("no managed block in .pre-commit-config.yaml");
    for gate in spec_driven_docs::gates::GATES {
        assert!(
            block.contains(&format!("- id: {}\n", gate.id)),
            "{} is registered but this repository's managed block does not wire it",
            gate.id
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
fn the_sdd_setup_skill_quotes_the_agents_snippet_verbatim() {
    let snippet = read("instance/snippets/AGENTS-docs.md");
    let skill = read("skills/sdd-setup/SKILL.md");
    assert!(
        skill.contains(snippet.trim_end()),
        "skills/sdd-setup/SKILL.md no longer quotes instance/snippets/AGENTS-docs.md"
    );
}

/// The embedded payload roots, read from the one declaration the binary and
/// `build.rs` also read. The license files carry no method content and are
/// embedded individually rather than as a root, so they are not scanned.
use spec_driven_docs::payload_roots::PAYLOAD_ROOTS;

/// Projects, products, and organizations outside this one that the payload
/// must not name.
///
/// This list is a denylist rather than a judgement: it holds the specific
/// names a reader of this repository alone could not resolve. The forges,
/// agents, and reference works the method genuinely documents — GitHub,
/// Claude, Bugzilla, Diátaxis — are integrations this framework describes,
/// and naming them is what makes the chapter useful. What may not appear is
/// a sibling project of this author's, because a reader who lacks it meets
/// a reference they cannot follow.
const FOREIGN_PROJECTS: &[&str] = &["release-kit", "release_kit", "exobrain", "gubasso"];

/// Planning tools and planning frameworks the payload must not name.
///
/// Generic English that some tool also uses as its name — linear, shortcut,
/// pivotal alone — is left out: this list holds terms whose appearance can
/// only mean the tool. Jira is judged separately below, because it is also a
/// tracker whose comment markup this framework documents for filing.
const PLANNING_TOOLS: &[&str] = &[
    "wipctl",
    "trello",
    "asana",
    "clickup",
    "youtrack",
    "redmine",
    "basecamp",
    "pivotal tracker",
    "taiga",
    "scrum",
    "kanban",
    "burndown",
    "standup",
    "sprint",
    "story point",
];

fn walk_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            walk_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

/// Every file under every declared payload root, with its repository-relative
/// path, ready to be scanned line by line.
fn payload_files() -> Vec<(String, String)> {
    let mut paths = Vec::new();
    for root in PAYLOAD_ROOTS {
        let path = canon().join(root);
        assert!(path.exists(), "{root} is not on disk; the payload moved");
        if path.is_dir() {
            walk_files(&path, &mut paths);
        } else {
            paths.push(path);
        }
    }
    assert!(!paths.is_empty(), "the payload scan matched no file");
    paths
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let relative = path.strip_prefix(canon()).unwrap().display().to_string();
            Some((relative, text))
        })
        .collect()
}

/// SATISFIES distribution:the-payload-names-no-planning-tool
#[test]
fn the_embedded_payload_names_no_planning_tool() {
    for (relative, text) in payload_files() {
        for (index, line) in text.lines().enumerate() {
            let number = index + 1;
            let lower = line.to_lowercase();
            for tool in PLANNING_TOOLS {
                assert!(
                    !lower.contains(tool),
                    "{relative}:{number}: the payload names the planning tool '{tool}'"
                );
            }
            assert!(
                !lower.contains("jira") || lower.contains("tracker-markup"),
                "{relative}:{number}: the payload names Jira outside a tracker-markup reference"
            );
        }
    }
}

/// SATISFIES distribution:the-payload-names-no-other-project
#[test]
fn the_embedded_payload_names_no_other_project() {
    for (relative, text) in payload_files() {
        for (index, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            for project in FOREIGN_PROJECTS {
                assert!(
                    !lower.contains(project),
                    "{relative}:{}: the payload names the outside project '{project}'",
                    index + 1
                );
            }
        }
    }
}

/// The canon's own build drivers. The installer wires neither into an
/// adopting project: `sdd`, `pre-commit`, and plain shell are what an
/// instance actually has.
const CANON_ONLY_COMMANDS: &[&str] = &["cargo", "just"];

/// The canon-only driver a shell command names, if any.
///
/// Whole words, not substrings, and not command position: deciding which
/// token a shell would execute needs a shell parser, and this rejects the
/// word wherever it appears instead. That over-rejects — `rg -q cargo x`
/// names no invocation — and the trade is deliberate: rewording a
/// verification line is cheap and visible, while a canon command reaching
/// every adopter is neither. `/` separates, so `/usr/bin/cargo` is caught;
/// `-` and `_` do not, so `cargo-audit` and `just-in-time` are words of
/// their own.
fn names_a_canon_command(command: &str) -> Option<&'static str> {
    let is_word = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    command
        .split(|c: char| !is_word(c))
        .find_map(|token| CANON_ONLY_COMMANDS.iter().copied().find(|d| *d == token))
}

/// A `Verify:` value that is a shell command, or `None` for a named human
/// procedure, which carries no command and is held by the unenforced table.
fn verification_command(verification: &str) -> Option<&str> {
    verification
        .starts_with('`')
        .then(|| verification.trim_matches('`'))
}

#[test]
fn a_canon_command_is_recognized_by_token() {
    assert_eq!(names_a_canon_command("cargo nextest run"), Some("cargo"));
    assert_eq!(names_a_canon_command("just"), Some("just"));
    assert_eq!(names_a_canon_command("  just   check  "), Some("just"));
    assert_eq!(names_a_canon_command("sh -c 'cargo build'"), Some("cargo"));
    // A path-qualified invocation is still the command.
    assert_eq!(names_a_canon_command("/usr/bin/cargo check"), Some("cargo"));
    // Deliberate over-rejection: the word is rejected wherever it appears,
    // because command position needs a shell parser.
    assert_eq!(
        names_a_canon_command("rg -q cargo README.md"),
        Some("cargo")
    );
    // The word only as part of a longer token is not the word.
    assert_eq!(names_a_canon_command("rg cargo-audit ."), None);
    assert_eq!(names_a_canon_command("pre-commit run adr-word-cap"), None);
    assert_eq!(names_a_canon_command("just-in-time"), None);
    assert_eq!(names_a_canon_command("sdd verify --target ."), None);
}

#[test]
fn a_human_procedure_carries_no_command() {
    assert_eq!(
        verification_command("reviewer confirms just the diff"),
        None
    );
    assert_eq!(verification_command("`true`"), Some("true"));
    assert_eq!(verification_command("``rg -o 'x' .``"), Some("rg -o 'x' ."));
}

/// SATISFIES distribution:a-seeded-rule-runs-no-canon-command
#[test]
fn a_seeded_rule_runs_no_canon_command() {
    let seeds: Vec<&str> = spec_driven_docs::domain::profile::ProfileId::KnowledgeBase
        .profile()
        .adopted
        .iter()
        .map(|entry| entry.source)
        .filter(|source| source.contains("/SPEC-"))
        .collect();
    assert!(!seeds.is_empty(), "the profile seeds no spec");
    let mut commands = 0usize;
    for source in seeds {
        let path = canon().join(source);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{source} is seeded but not on disk"));
        for (index, line) in text.lines().enumerate() {
            let Some(verification) = line.strip_prefix("Verify: ") else {
                continue;
            };
            let Some(command) = verification_command(verification) else {
                continue;
            };
            commands += 1;
            assert!(
                names_a_canon_command(command).is_none(),
                "{source}:{}: a seeded rule is verified by `{}`, which no instance runs",
                index + 1,
                names_a_canon_command(command).unwrap_or_default()
            );
        }
    }
    assert!(commands > 0, "no seeded rule carries a command to judge");
}
