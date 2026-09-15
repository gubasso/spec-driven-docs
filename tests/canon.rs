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
    clippy::case_sensitive_file_extension_comparisons,
    reason = "the panic is this suite's failure signal, not control flow"
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
        manifest["schema_version"], 3,
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

    let managed = recorded_destinations(&manifest, "managed_files");
    for (path, _) in spec_driven_docs::embedded::shared_artifacts() {
        let path = format!("skill-shared/{path}");
        assert!(
            managed.contains(&path),
            "{path} is in the payload but absent from the record; run 'just manifest'"
        );
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

    for template in spec_driven_docs::domain::profile::CANON_TEMPLATES.iter() {
        assert!(
            canon().join(template).is_file(),
            "{template} is declared but gone from the tree"
        );
        assert!(
            adopted.contains(&(*template).to_string()),
            "{template} is on disk but absent from the record; run 'just manifest'"
        );
    }
    for destination in &adopted {
        if destination
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("TEMPLATE-"))
        {
            assert!(
                spec_driven_docs::domain::profile::CANON_TEMPLATES.contains(&destination.as_str()),
                "{destination} is recorded but no longer declared; a canon-side template has one owner"
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

/// SATISFIES release:a-delivered-gate-reads-what-the-convention-owns
///
/// Every row states the subject paths it judges. The v0.6.5 rule exempted any
/// row resolving its own scope, which was 19 of the 30, so it constrained 3.
#[test]
fn every_gate_declares_what_it_judges() {
    // A row with no include judges everything its excludes leave, and four
    // do. What a row may not do is leave the question unanswered, which is
    // what the v0.6.5 rule's third clause permitted for 19 of the 30 rows.
    const JUDGES_WHAT_ITS_EXCLUDES_LEAVE: &[&str] = &[
        "gate-message-cites-a-rule",
        "no-personal-path",
        "suppression-names-its-case",
    ];

    for gate in spec_driven_docs::gates::GATES {
        let id = gate.id.to_string();
        // Every declared pattern is held to the filter grammar, on a row
        // that states an include and on one that states only excludes.
        // Branching before this check let an empty-include row ship a
        // pattern nothing validated.
        for pattern in gate.include.iter().chain(gate.exclude) {
            assert!(
                !pattern.starts_with('/') && !pattern.starts_with('!'),
                "{id} states the pattern {pattern}, which the filter grammar refuses"
            );
        }
        assert!(
            !gate.include.is_empty() || JUDGES_WHAT_ITS_EXCLUDES_LEAVE.contains(&id.as_str()),
            "{id} states no include patterns, so it judges every file its excludes \
             leave. A row that means that says so in JUDGES_WHAT_ITS_EXCLUDES_LEAVE \
             here and in a comment on its row; a row that does not states its scope."
        );
    }
}

/// Every route by which a subject path reaches a gate passes through the
/// filter, so a gate cannot judge a path the project excluded.
///
/// The inventory is the proof behind
/// `release:a-delivered-gate-reads-what-the-convention-owns`. A fourth route
/// added without a filter is a hole this test names.
///
/// SATISFIES release:a-delivered-gate-reads-what-the-convention-owns
#[test]
fn every_subject_producer_is_filter_aware() {
    let gates_rs = std::fs::read_to_string(canon().join("src/gates.rs")).unwrap();
    assert!(
        gates_rs.contains("ctx.subjects(files)"),
        "walk_files stopped filtering its result"
    );

    let command = std::fs::read_to_string(canon().join("src/commands/gate.rs")).unwrap();
    assert!(
        command.contains("subjects("),
        "the command path stopped filtering the paths pre-commit passes"
    );

    // Every gate that discovers its own subjects passes them through
    // `retained`, where the discovery is the include and the declaration's
    // exclusions still bind. This list is the inventory; a gate added to it
    // without a filtering call is the hole the source-text check cannot see,
    // which is why `cmd_gate` also asserts the behaviour per gate.
    for (file, verb) in [
        ("src/gates/paths.rs", "ctx.retained("),
        ("src/gates/spec_rule_id_unique.rs", "ctx.retained("),
        ("src/gates/spec_verify_hooks_exist.rs", "ctx.retained("),
        ("src/gates/adr_cites_a_live_rule.rs", "ctx.retained("),
        ("src/gates/adr_word_cap.rs", ".retained("),
        ("src/gates/instance_manifest.rs", "ctx.retained("),
        ("src/gates/tracking_registry.rs", ".retained("),
    ] {
        let text = std::fs::read_to_string(canon().join(file)).unwrap();
        assert!(
            text.contains(verb),
            "{file} discovers its own subjects and stopped filtering them"
        );
    }

    // No gate reaches `walkdir` directly: the one traversal is `walk_files`,
    // and it filters.
    let mut unfiltered = Vec::new();
    for entry in std::fs::read_dir(canon().join("src/gates"))
        .unwrap()
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        if text.contains("walkdir::") {
            unfiltered.push(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    assert!(
        unfiltered.is_empty(),
        "these gates walk the tree outside walk_files, so their subjects are \
         unfiltered: {unfiltered:?}"
    );
}

/// A row's `discovers` matches the route its implementation takes.
///
/// `--explain` reads the field, so a wrong value makes the public
/// diagnostic contradict the gate it describes.
///
/// SATISFIES release:a-delivered-gate-reads-what-the-convention-owns
#[test]
fn every_row_declaring_discovery_takes_the_retained_route() {
    // Every row that declares `discovers` uses the retained route, and no
    // row that does not. `--explain` reads the field, so a wrong value
    // makes the diagnostic contradict the gate it describes.
    let retained_route: &[(&str, &[&str])] = &[
        (
            "src/gates/paths.rs",
            &[
                "ki-bugzilla-report-width",
                "ki-checked-date",
                "ki-mechanism-walkthrough",
                "ki-report-body",
                "ki-retire-when",
                "ki-filing",
                "ki-state",
            ],
        ),
        (
            "src/gates/spec_rule_id_unique.rs",
            &[
                "spec-rule-id-unique",
                "spec-size-cap",
                "spec-verify-hooks-exist",
            ],
        ),
        (
            "src/gates/adr_cites_a_live_rule.rs",
            &["adr-cites-a-live-rule"],
        ),
        ("src/gates/adr_word_cap.rs", &["adr-word-cap"]),
        ("src/gates/instance_manifest.rs", &["instance-manifest"]),
        ("src/gates/tracking_registry.rs", &["tracking-registry"]),
    ];
    let declared: std::collections::BTreeSet<String> = spec_driven_docs::gates::GATES
        .iter()
        .filter(|row| row.discovers)
        .map(|row| row.id.to_string())
        .collect();
    let routed: std::collections::BTreeSet<String> = retained_route
        .iter()
        .flat_map(|(_, ids)| ids.iter().map(ToString::to_string))
        .collect();
    assert_eq!(
        declared, routed,
        "a row's `discovers` disagrees with the route its implementation takes"
    );
    for (file, _) in retained_route {
        let text = std::fs::read_to_string(canon().join(file)).unwrap();
        assert!(
            text.contains("retained("),
            "{file} serves a row declaring `discovers` and stopped using the retained route"
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

/// The documentation subtrees this repository authors. A numbered document
/// in any of them is the merge collision the naming rule exists to prevent.
const DOCUMENT_SUBTREES: &[&str] = &[
    "method",
    "comparison-docs",
    "templates",
    "reference",
    "_docs",
    "instance",
    "skills",
    "skill-shared",
];

/// SATISFIES docs-foundations:a-kind-prefix-carries-a-slug
///
/// The chapter shelves are canon subtrees no instance edits, so this is a
/// canon check and not a delivered gate.
#[test]
fn no_authored_document_carries_an_ordinal_prefix() {
    let mut markdown = Vec::new();
    for subtree in DOCUMENT_SUBTREES {
        walk_markdown(&canon().join(subtree), &mut markdown);
    }
    assert!(
        !markdown.is_empty(),
        "the document subtrees hold no Markdown"
    );
    for path in markdown {
        let name = path.file_name().unwrap().to_string_lossy();
        // A digit run followed by a hyphen at the front of the slug, after
        // any uppercase kind prefix, is the allocated form. A digit inside a
        // slug, as in `2fa-setup.md`, names a subject and passes.
        let slug = match name.split_once('-') {
            Some((prefix, rest))
                if !prefix.is_empty() && prefix.bytes().all(|b| b.is_ascii_uppercase()) =>
            {
                rest
            }
            _ => name.as_ref(),
        };
        let bytes = slug.as_bytes();
        let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
        let numbered = digits > 0 && bytes.get(digits) == Some(&b'-');
        assert!(
            !numbered,
            "{}: a document is named by a slug, not by a number; the reading order belongs in the directory's README.md",
            path.strip_prefix(canon()).unwrap().display()
        );
    }
}

/// SATISFIES docs-foundations:a-document-directory-explains-itself
///
/// The file exists and nothing more. Reading inside it would make the prose
/// a contract, which defeats the reason the rule asks for prose.
#[test]
fn every_directory_of_slug_named_documents_has_a_readme() {
    for directory in [
        "method",
        "comparison-docs",
        "reference/prior-art",
        "reference/tracker-markup",
    ] {
        assert!(
            canon().join(directory).join("README.md").is_file(),
            "{directory}/ holds slug-named documents and no README.md explains them"
        );
    }
}

/// The RFC 2119 spellings `method/rules.md` declines to declare. Each is a
/// synonym of a declared keyword, and `ADR-declare-one-spelling-per-requirement-level`
/// states why a synonym costs a reader a decision.
const RETIRED_KEYWORDS: &[&str] = &["SHALL", "REQUIRED", "RECOMMENDED", "OPTIONAL"];

/// Files quoting another project's terms, which keep their own words.
const QUOTED_TERMS: &[&str] = &[
    "THIRD_PARTY_NOTICES.md",
    "LICENSE",
    "LICENSE-MIT",
    "LICENSE-CC-BY-4.0",
];

/// Whether `line` carries `word` as a whole uppercase word.
fn names_uppercase_word(line: &str, word: &str) -> bool {
    line.match_indices(word).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + word.len()..].chars().next();
        !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            && !after.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// SATISFIES docs-specs:statement-uses-an-ears-pattern
///
/// The chapter declares the set, the gate snippets and the spec's `Verify:`
/// commands run it, and this holds the authored tree to the declaration so
/// the three copies cannot drift apart in silence.
#[test]
fn no_authored_document_uses_a_retired_keyword() {
    let mut markdown = Vec::new();
    for subtree in DOCUMENT_SUBTREES {
        walk_markdown(&canon().join(subtree), &mut markdown);
    }
    for root_file in ["README.md", "AGENTS.md"] {
        markdown.push(canon().join(root_file));
    }
    for path in markdown {
        let relative = path.strip_prefix(canon()).unwrap().display().to_string();
        if QUOTED_TERMS.contains(&path.file_name().unwrap().to_str().unwrap()) {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for (index, line) in text.lines().enumerate() {
            for word in RETIRED_KEYWORDS {
                assert!(
                    !names_uppercase_word(line, word),
                    "{relative}:{}: '{word}' is a synonym of a declared keyword; method/rules.md declares one spelling per level",
                    index + 1
                );
            }
        }
    }
}

#[test]
fn a_retired_keyword_is_matched_as_an_uppercase_word() {
    assert!(names_uppercase_word("The author SHALL cite it.", "SHALL"));
    assert!(names_uppercase_word("(SHALL)", "SHALL"));
    assert!(!names_uppercase_word("the author shall cite it", "SHALL"));
    assert!(!names_uppercase_word("MARSHALL", "SHALL"));
    assert!(!names_uppercase_word("OPTIONAL_FLAG", "OPTIONAL"));
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

/// SATISFIES distribution:a-skill-plans-before-it-acts
#[test]
fn every_skill_routes_to_the_plan_gate_before_acting() {
    for (dir, text) in skill_dirs() {
        let (_, body) = split_frontmatter(&text);
        let sections: Vec<usize> = body
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with("## "))
            .map(|(index, _)| index)
            .collect();
        let first = *sections
            .first()
            .unwrap_or_else(|| panic!("{dir}: the body has no sections"));
        assert_eq!(
            body[first], "## Before acting",
            "{dir}: the plan gate does not lead every other section"
        );
        let end = sections.get(1).copied().unwrap_or(body.len());
        let section = body[first..end].join("\n");
        // Both gates are named by the path they carry inside the package,
        // relative to the skill's own root, which is what every documented
        // host resolves a supporting file against.
        for artifact in ["pre-flight-gate.md", "plan-gate.md"] {
            let named = format!("references/{artifact}");
            assert!(
                section.contains(&named),
                "{dir}: the gate section does not name {named}"
            );
        }
        // The spec binds the order too: the pre-flight is read first,
        // because the plan gate takes its findings as inputs.
        let pre_flight = section
            .find("references/pre-flight-gate.md")
            .unwrap_or_default();
        let plan = section.find("references/plan-gate.md").unwrap_or_default();
        assert!(
            pre_flight < plan,
            "{dir}: the gate section names the plan gate before the pre-flight gate"
        );
        assert!(
            section.contains("--no-plan"),
            "{dir}: the gate section does not state the --no-plan rule"
        );
        assert!(
            section.contains("No flag skips it"),
            "{dir}: the gate section does not state that the pre-flight is unconditional"
        );
    }
}

/// SATISFIES distribution:a-skill-checks-its-host-before-it-plans
///
/// The shared artifacts are files, carried by the payload and installed
/// once, so correcting one corrects every skill under every agent root.
/// Each has its own duty and the payload carries both.
#[test]
fn the_payload_carries_both_shared_gates() {
    let plan = read("skill-shared/plan-gate.md");
    for phase in ["## 1. Plan", "## 2. Validate", "## 3. Execute"] {
        assert!(plan.contains(phase), "the plan gate lost {phase}");
    }
    assert!(
        plan.contains("--no-plan"),
        "the plan gate does not state the --no-plan rule"
    );
    assert!(
        plan.contains("pre-flight-gate.md"),
        "the plan gate does not take the pre-flight's findings as inputs"
    );

    let pre_flight = read("skill-shared/pre-flight-gate.md");
    assert!(
        pre_flight.contains("sdd doctor"),
        "the pre-flight gate does not run the probe catalog"
    );
    // Read from the catalog's own declaration, not listed here: a skill
    // probe added without a line in the pre-flight gate is a probe no agent
    // following it ever reads.
    for id in spec_driven_docs::probes::SKILL_PROBES {
        assert!(
            pre_flight.contains(id),
            "the pre-flight gate does not read the {id} probe"
        );
    }
    // The pre-flight runs whatever the request carries; only the plan gate
    // has a flag. A pre-flight that could be waived is one no skill can
    // rely on having run.
    assert!(
        pre_flight.contains("No flag skips it"),
        "the pre-flight gate does not state that it is unconditional"
    );
    assert!(
        pre_flight.contains("plan-gate.md"),
        "the pre-flight gate does not hand the task to the plan gate"
    );
}

/// SATISFIES distribution:a-landing-classifies-its-target-first
#[test]
fn the_pre_flight_gate_invokes_no_planner_or_skill() {
    let gate = read("skill-shared/pre-flight-gate.md");
    for named in ["sdd assess", "sdd stage", "migration skill"] {
        assert!(
            !gate.contains(named),
            "the pre-flight gate names '{named}'; it hands its findings back and routes nothing"
        );
    }
}

/// SATISFIES staging:a-stage-writes-only-the-stage
#[test]
fn the_setup_skill_stages_before_it_compares() {
    let setup = read("skills/sdd-setup/SKILL.md");
    assert!(
        setup.contains("sdd stage --target . --json"),
        "the router does not stage the candidate"
    );
    assert!(
        setup.contains("The stage writes nothing into the target"),
        "the router does not say that staging leaves the target alone"
    );
    assert!(
        !setup.contains("sdd assess"),
        "the router classifies for itself instead of letting the verb refuse"
    );
}

/// SATISFIES staging:the-operator-owns-acquisition
#[test]
fn the_setup_skill_leaves_acquisition_to_the_operator() {
    let setup = read("skills/sdd-setup/SKILL.md");
    assert!(
        setup.contains("Acquisition is the operator's"),
        "the router does not say who installs the version"
    );
    for absent in ["--to <version>", "--to latest", "sdd payload", "plan-id"] {
        assert!(
            !setup.contains(absent),
            "the router still names {absent}, which no verb offers"
        );
    }
}

/// SATISFIES distribution:a-landing-classifies-its-target-first
#[test]
fn the_setup_skill_routes_each_classification_to_its_chapter() {
    let setup = read("skills/sdd-setup/SKILL.md");
    for classification in [
        "setup",
        "migration",
        "upgrade",
        "drift",
        "current",
        "invalid",
    ] {
        assert!(
            setup.contains(classification),
            "the router does not name the {classification} classification"
        );
    }
    for chapter in ["sdd docs migration", "sdd docs reconcile"] {
        assert!(
            setup.contains(chapter),
            "the router does not route a classification to {chapter}"
        );
    }
}

#[test]
fn the_setup_skill_names_the_five_steps() {
    let setup = read("skills/sdd-setup/SKILL.md");
    for step in [
        "## 1. Observe",
        "## 2. Acquire the version, then stage it",
        "## 3. Compare and prepare",
        "## 4. Land",
        "## 5. Verify, then clean",
    ] {
        assert!(setup.contains(step), "the router has no '{step}' section");
    }
}

#[test]
fn the_setup_skill_declares_its_gated_steps() {
    let setup = read("skills/sdd-setup/SKILL.md");
    assert!(setup.contains("## What waits for the operator"));
    for gated in [
        "ignore entry for the docs scratch",
        "sdd stage clean <path>",
        "before its entry retires anything",
        "Every retirement of a file the project authored",
        "Every disposition question",
        "migration directory at the close",
    ] {
        assert!(
            setup.contains(gated),
            "the router does not gate '{gated}' for the operator"
        );
    }
}

/// VERIFIES acquisition:the-setup-path-offers-the-wire
#[test]
fn the_setup_skill_offers_the_freshness_wire() {
    let setup = read("skills/sdd-setup/SKILL.md");
    let readme = read("instance/README.md");
    assert!(
        setup.contains("## Offer the freshness wire"),
        "the router has no freshness-wire step"
    );
    for held in [
        // The observation the offer rests on.
        "sdd self-depend status --target . --json",
        "Hold `wired` and `envrc_sync`",
        // Silence where nothing is wired, and where the wire is landed.
        "Where `wired` is `null`, there is no pin to keep fresh",
        "`envrc_sync` is `true`, the wire is landed",
        // The offer, with its three facts.
        "`envrc_sync` is `false`, ask with `AskUserQuestion`",
        "sdd self-depend sync --apply || true",
        "attempts at most one bump a day",
        "leaves a diff for the operator to review and commit",
        // Landing the line is gated, and a no is recorded.
        "landing the line is a gated step",
        "sdd self-depend add --target . --manager <wired>",
        "re-observe with the status verb until `envrc_sync` reads `true`",
        "the landing is complete without the wire",
        "Record the answer in the close of the task",
    ] {
        assert!(setup.contains(held), "the router does not state '{held}'");
    }
    // The operator's path asks the same question in the same words.
    for shared in [
        "does not close until the operator has answered about the freshness wire",
        "Where `wired` is `null`, there is no pin to keep fresh",
        "attempts at most one bump a day",
        "leaves a diff for the operator to review and commit",
        "seeds `.envrc` only where the target has none",
    ] {
        assert!(
            readme.contains(shared),
            "instance/README.md does not carry '{shared}'"
        );
    }
}

#[test]
fn the_setup_skill_offers_incremental_only_as_the_plan_does() {
    let setup = read("skills/sdd-setup/SKILL.md");
    assert!(
        setup.contains("Author no checklist, because nothing retires"),
        "the router states a scope rule of its own instead of deferring to the plan"
    );
}

/// SATISFIES docs-discovery:an-instance-is-routed-to-the-index
#[test]
fn every_skill_routes_to_a_topic_and_restates_no_catalog_entry() {
    use spec_driven_docs::domain::docs_catalog::{CATALOG, Target};

    for (dir, text) in skill_dirs() {
        assert!(
            text.contains("sdd docs"),
            "{dir}: names no reader verb for the corpus"
        );
        for topic in &CATALOG.topics {
            let copied =
                matches!(topic.target, Target::Document { .. }) && text.contains(&topic.summary);
            assert!(
                !copied,
                "{dir}: copies the catalog summary of '{}'",
                topic.id
            );
        }
    }
}

#[test]
fn the_payload_carries_two_skills() {
    let names = spec_driven_docs::embedded::skill_names();
    assert_eq!(
        names,
        vec!["sdd-setup", "sdd-write-docs"],
        "the payload carries {names:?}"
    );
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
///
/// The install manages the `AGENTS.md` documentation block now, so the skill
/// defers to `sdd init` rather than hand-copying a section that would drift.
#[test]
fn the_sdd_setup_skill_defers_the_agents_block_to_the_installer() {
    let skill = read("skills/sdd-setup/SKILL.md");
    assert!(
        skill.contains("`sdd init` manages this"),
        "skills/sdd-setup/SKILL.md no longer defers the AGENTS block to the installer"
    );
    assert!(
        skill.contains("`sdd method writing-style`"),
        "the skill does not name the writing-style command the block installs"
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

/// SATISFIES distribution:the-payload-names-no-other-project
#[test]
fn the_embedded_payload_names_no_other_project() {
    // This project's own home is not another project. The owner's account
    // name is part of that URL, and the payload carries the URL wherever it
    // has to identify the canon it came from.
    let own = spec_driven_docs::domain::manifest::CANON_SOURCE.to_lowercase();
    for (relative, text) in payload_files() {
        for (index, line) in text.lines().enumerate() {
            let lower = line.to_lowercase().replace(&own, "");
            for project in FOREIGN_PROJECTS {
                assert!(
                    !lower.contains(project),
                    "{relative}:{}: the payload names the outside project '{project}'",
                    index + 1
                );
            }
            // Jira is a documented integration, because the payload carries
            // its comment markup under `reference/tracker-markup/`. The
            // name is legitimate inside that shelf, and elsewhere only on a
            // line that says which shelf it is pointing at.
            assert!(
                !lower.contains("jira")
                    || lower.contains("tracker-markup")
                    || relative.starts_with("reference/tracker-markup/"),
                "{relative}:{}: the payload names Jira outside a tracker-markup reference",
                index + 1
            );
        }
    }
}

/// Terms the tracked tree carries nowhere outside the immutable records and
/// the generated changelog, as word pairs and the separators that join them.
///
/// Spelled as pairs so this file does not carry the terms it forbids: the
/// sweep below reads this file too, and a literal here would fail it.
const ABSENT_PAIRS: &[(&str, &str)] = &[
    ("plan", "zone"),
    ("entry", "document"),
    ("entry", "file"),
    ("planning", "tool"),
];

/// Every spelling of the absent pairs, joined by a space, a hyphen, or an
/// underscore, lowercase.
fn absent_terms() -> Vec<String> {
    ABSENT_PAIRS
        .iter()
        .flat_map(|(head, tail)| [" ", "-", "_"].map(|join| format!("{head}{join}{tail}")))
        .collect()
}

/// Every file the tree carries, with its repository-relative path, that is
/// UTF-8. The walk skips what git ignores here, because the Nix build
/// sandbox this test also runs in has no git to ask.
fn tracked_text_files() -> Vec<(String, String)> {
    walkdir::WalkDir::new(canon())
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            name != ".git" && name != "target" && name != ".direnv" && name != ".docs-scratch"
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let relative = entry
                .path()
                .strip_prefix(canon())
                .ok()?
                .to_string_lossy()
                .to_string();
            let text = std::fs::read_to_string(entry.path()).ok()?;
            Some((relative, text))
        })
        .collect()
}

/// The tree states the context entrypoint and nothing older.
///
/// A decision record is the one document class that carries history, and the
/// changelog is generated from it, so those two are the only carriers.
#[test]
fn the_tracked_tree_carries_no_retired_entry_term() {
    // The compatibility read of a record written before the removal names
    // the retired field once, and the tests that prove the removal name what
    // they remove. Nothing else may.
    const REMOVAL_CARRIERS: &[&str] = &[
        "src/domain/manifest.rs",
        "tests/cmd_init.rs",
        "tests/cmd_status.rs",
        "tests/cmd_upgrade.rs",
    ];
    let terms = absent_terms();
    let tree = tracked_text_files();

    // An exemption for a file that no longer names a retired term exempts
    // nothing, and the comment above the list then claims something untrue.
    // The list is small and hand-kept, so it states what it still covers.
    for carrier in REMOVAL_CARRIERS {
        let text = tree
            .iter()
            .find(|(relative, _)| relative == carrier)
            .map(|(_, text)| text.to_lowercase());
        let Some(text) = text else {
            panic!("{carrier} is exempted from the retired terms and is not in the tree");
        };
        assert!(
            terms.iter().any(|term| text.contains(term.as_str())),
            "{carrier} is exempted from the retired terms and carries none"
        );
    }

    for (relative, text) in tree {
        if relative.starts_with("_docs/decisions/")
            || relative == "CHANGELOG.md"
            || REMOVAL_CARRIERS.contains(&relative.as_str())
        {
            continue;
        }
        for (index, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            for term in &terms {
                assert!(
                    !lower.contains(term.as_str()),
                    "{relative}:{}: the tree carries '{term}'",
                    index + 1
                );
            }
        }
    }
}

/// The agent-context chapter frames retrieval around a session and its
/// working subject, and lets a subject touch more than one domain.
#[test]
fn the_agent_context_chapter_owns_the_context_entrypoint() {
    let chapter = read("method/agent-context.md");
    let lower = chapter.to_lowercase();
    assert!(
        !lower.contains("unit of work"),
        "method/agent-context.md names another tool's structure"
    );
    assert!(
        lower.contains("one context entrypoint for each domain"),
        "method/agent-context.md stopped stating the per-domain cardinality"
    );
    assert!(
        chapter.contains("./specs.md#the-reference-runs-one-way"),
        "method/agent-context.md stopped linking the owner of the decision-record prohibition"
    );
    let normative = chapter
        .lines()
        .filter(|line| line.contains("MUST NOT") && line.contains("name a decision record"))
        .count();
    assert_eq!(
        normative, 0,
        "method/agent-context.md restates the prohibition the specs chapter owns"
    );
    let specs = read("method/specs.md");
    let owner = specs
        .lines()
        .filter(|line| line.contains("MUST NOT link or name a decision record"))
        .count();
    assert_eq!(
        owner, 1,
        "method/specs.md states the decision-record prohibition {owner} times"
    );
    let format = read("method/format.md");
    assert!(
        format.contains("The other is not a count"),
        "method/format.md declares more than one ungated budget"
    );
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
        .map(|entry| entry.source.as_str())
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

/// One row of the chapter's sources table: what was read, and on what terms.
struct Source {
    repository: String,
    revision: String,
    terms: String,
}

/// Every source `method/writing-style.md` declares, read from its table.
///
/// The chapter is the one owner of what the style draws on, so the notice is
/// judged against whatever that table says today. Naming the sources here
/// instead would be a second list, and a source added to the chapter and
/// forgotten in the notice would pass.
fn declared_sources() -> Vec<Source> {
    let style = read("method/writing-style.md");
    let sources = style
        .split_once("## Sources")
        .expect("the chapter has no sources section")
        .1;
    let rows: Vec<Source> = sources
        .lines()
        .take_while(|line| !line.starts_with("## "))
        .filter(|line| line.starts_with('|'))
        // The header row and the delimiter row carry no source.
        .filter(|line| !line.contains("| ---") && !line.contains("Revision read"))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert!(
                cells.len() >= 3,
                "a sources row has too few columns: {line}"
            );
            let repository = cells[0]
                .split_once('(')
                .expect("a sources row names no repository link")
                .1
                .trim_end_matches(')')
                .to_string();
            Source {
                repository,
                revision: cells[1].trim_matches('`').to_string(),
                terms: cells[2].to_string(),
            }
        })
        .collect();
    assert!(!rows.is_empty(), "the sources table declares no source");
    for row in &rows {
        // An empty cell would assert nothing against the notice.
        assert!(
            !row.repository.is_empty() && !row.revision.is_empty() && !row.terms.is_empty(),
            "a sources row leaves a cell empty: {}",
            row.repository
        );
    }
    rows
}

/// SATISFIES release:third-party-notices-travel-with-the-payload
#[test]
fn the_notice_names_the_resolved_revision_and_terms() {
    let notice = read("THIRD_PARTY_NOTICES.md");
    for source in declared_sources() {
        let repository = &source.repository;
        assert!(
            notice.contains(repository),
            "the notice omits the source repository {repository}"
        );
        assert!(
            notice.contains(&source.revision),
            "the notice omits the revision read for {repository}"
        );
        assert!(
            notice.contains(&source.terms),
            "the notice omits the terms of {repository}"
        );
    }
    // The binary carries the notice byte-for-byte.
    assert_eq!(
        spec_driven_docs::embedded::THIRD_PARTY_NOTICES,
        notice,
        "the embedded notice differs from the file"
    );
}

/// SATISFIES release:third-party-notices-travel-with-the-payload
///
/// The crate excludes neither the notice nor the license, so both ship in the
/// packaged crate.
#[test]
fn the_package_carries_the_notice_and_license() {
    let cargo = read("Cargo.toml");
    for kept in ["/THIRD_PARTY_NOTICES.md", "/LICENSE"] {
        assert!(
            !cargo.contains(&format!("\"{kept}\"")),
            "Cargo.toml excludes {kept}, which must travel with the payload"
        );
    }
}

/// The two retired names for the docs scratch, and how each is matched.
///
/// `.draft` was the fixed path. `workshop` was the term. Both are held out
/// here rather than by review, because one reintroduction re-fixes the
/// location this framework just stopped fixing.
///
/// `workshop` is ordinary English, so it is matched as a whole word only.
/// A substring match on a common word blocks prose it was never about, so
/// the whole-word flag exists.
const RETIRED_TERMS: &[(&str, bool)] = &[(".draft", false), ("workshop", true)];

/// Everything this repository authors that the retired-term sweep reads.
///
/// The payload roots plus the two root files that carry the same rule for this
/// repository's own agents.
fn authored_prose() -> Vec<(String, String)> {
    let mut files = payload_files();
    for root_file in ["AGENTS.md", ".gitignore"] {
        files.push((root_file.to_string(), read(root_file)));
    }
    files
}

/// Whether `line` carries `term` as a whole word, case-insensitively.
fn names_whole_word(line: &str, term: &str) -> bool {
    let lower = line.to_lowercase();
    lower.match_indices(term).any(|(at, _)| {
        let before = lower[..at].chars().next_back();
        let after = lower[at + term.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// SATISFIES distribution:a-declared-location-is-named-by-its-variable
///
/// The corpus names each declared location by its variable and nothing else.
#[test]
fn the_authored_corpus_names_no_retired_location_term() {
    for (relative, text) in authored_prose() {
        for (index, line) in text.lines().enumerate() {
            for (term, whole_word) in RETIRED_TERMS {
                let named = if *whole_word {
                    names_whole_word(line, term)
                } else {
                    line.to_lowercase().contains(term)
                };
                assert!(
                    !named,
                    "{relative}:{}: the corpus names '{term}'; the docs scratch is named by SDD_DOCS_SCRATCH",
                    index + 1
                );
            }
        }
    }
}

#[test]
fn a_retired_word_is_matched_as_a_word_rather_than_a_substring() {
    assert!(names_whole_word("run a Workshop with the team", "workshop"));
    assert!(names_whole_word("the workshop.", "workshop"));
    assert!(!names_whole_word("workshopping the idea", "workshop"));
    assert!(!names_whole_word("a preworkshop note", "workshop"));
}

/// SATISFIES distribution:a-declared-location-is-named-by-its-variable
///
/// A concrete candidate path has one home: the paths section of the status
/// report, which proposes a directory only where the target already holds
/// it. Nothing authored spells one, so changing what is offered never means
/// editing a chapter, a spec, a template, or a skill.
#[test]
fn a_concrete_declared_path_appears_in_no_authored_prose() {
    for (relative, text) in authored_prose() {
        for (index, line) in text.lines().enumerate() {
            if !line.contains(".docs-scratch") {
                continue;
            }
            // `.gitignore` is this repository's own declaration rather than
            // payload prose, so it carries the path it declared.
            assert_eq!(
                relative,
                ".gitignore",
                "{relative}:{}: a concrete docs-scratch path outside this repository's own declaration",
                index + 1
            );
        }
    }
}

/// SATISFIES release:the-canon-record-describes-its-tree
///
/// This repository dogfoods the declared location it delivers. Without this,
/// a later regeneration could leave the ignore entry naming a scratch the
/// record no longer declares.
#[test]
fn the_canon_declares_its_own_docs_scratch() {
    let manifest = recorded_manifest();
    let scratch = manifest["docs_scratch"]
        .as_str()
        .expect("this repository declares no docs scratch");
    assert!(
        read(".gitignore")
            .lines()
            .any(|line| line.trim_end_matches('/') == scratch.trim_end_matches('/')),
        ".gitignore does not carry the docs scratch the record declares: {scratch}"
    );
}

/// Every path the status report carries, spelled as a skill would spell it.
///
/// Derived from the report rather than listed here, so a field added to
/// `Paths` joins the invariant without an edit to this file.
fn reported_paths() -> std::collections::BTreeSet<String> {
    use spec_driven_docs::domain::paths::{Paths, UserEnv, candidates, proposals};

    let env = UserEnv {
        home: Some(camino::Utf8PathBuf::from("~")),
        ..UserEnv::default()
    };
    let report = Paths {
        user: env.user_paths(),
        active: None,
        candidates: candidates(),
        // Every proposal leaf is held, so every path the report can ever
        // carry is in the set a skill must not spell.
        proposals: proposals(&env, |_| true),
    };
    let value = serde_json::to_value(&report).unwrap();
    let mut found = std::collections::BTreeSet::new();
    collect_paths(&value, &mut found);

    let mut spellings = std::collections::BTreeSet::new();
    for path in found {
        // A documentation root is a bare word, and the prose needs the
        // word. What it may not spell is the directory, which carries its
        // separator.
        if path.contains('/') || path.contains('.') {
            spellings.insert(path.clone());
        } else {
            spellings.insert(format!("{path}/"));
            continue;
        }
        // A user-scope path is written either way, so both are held.
        if let Some(relative) = path.strip_prefix("~/") {
            spellings.insert(relative.to_string());
        }
    }
    spellings
}

fn collect_paths(value: &serde_json::Value, out: &mut std::collections::BTreeSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if key == "path"
                    && let Some(text) = child.as_str()
                {
                    out.insert(text.to_string());
                }
                collect_paths(child, out);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_paths(item, out);
            }
        }
        _ => {}
    }
}

/// Every authored skill file, skill by skill and shared file by shared file.
fn skill_prose() -> Vec<(String, String)> {
    let mut files = skill_dirs()
        .into_iter()
        .map(|(name, text)| (format!("skills/{name}/SKILL.md"), text))
        .collect::<Vec<_>>();
    for entry in std::fs::read_dir(canon().join("skill-shared")).unwrap() {
        let name = entry.unwrap().file_name().to_str().unwrap().to_string();
        let relative = format!("skill-shared/{name}");
        let text = read(&relative);
        files.push((relative, text));
    }
    files
}

/// Drop every fenced block the report itself produces.
///
/// A JSON example is the report's own output, so the paths in it are quoted
/// rather than restated.
fn without_json_examples(text: &str) -> String {
    let mut kept = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("```") {
            if inside {
                inside = false;
                continue;
            }
            inside = line.trim_start_matches('`').trim() == "json";
            if inside {
                continue;
            }
        }
        if !inside {
            kept.push(line);
        }
    }
    kept.join("\n")
}

/// SATISFIES distribution:a-declared-location-is-named-by-its-variable
#[test]
fn no_skill_spells_a_path_the_binary_reports() {
    let spellings = reported_paths();
    for (relative, text) in skill_prose() {
        let scanned = without_json_examples(&text);
        for spelling in &spellings {
            assert!(
                !scanned.contains(spelling.as_str()),
                "{relative} spells {spelling}; read it from the paths section of 'sdd status --json' instead"
            );
        }
    }
}

/// Every control path `domain::paths` declares, as a literal.
fn declared_control_paths() -> std::collections::BTreeSet<String> {
    let text = read("src/domain/paths.rs");
    let mut found = std::collections::BTreeSet::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("pub const ") else {
            continue;
        };
        let Some((_, value)) = rest.split_once("= \"") else {
            continue;
        };
        let Some((value, _)) = value.split_once('"') else {
            continue;
        };
        // A bare word is a directory name the rest of the tree composes
        // with, not a path a second module could redeclare.
        if value.contains('/') || value.contains('.') {
            found.insert(value.to_string());
        }
    }
    assert!(
        found.contains(".spec-driven-docs/manifest.json"),
        "the declaration scan found no control paths; its parse is stale"
    );
    found
}

/// Every production Rust file, with its test module cut off.
fn production_rust() -> Vec<(String, String)> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(canon().join("src"))
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() || entry.path().extension() != Some("rs".as_ref()) {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(canon())
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let text = std::fs::read_to_string(entry.path()).unwrap();
        let production = text
            .split_once("#[cfg(test)]")
            .map_or_else(|| text.clone(), |(before, _)| before.to_string());
        files.push((relative, production));
    }
    files
}

/// SATISFIES distribution:a-declared-location-is-named-by-its-variable
#[test]
fn every_control_path_constant_has_one_declaration() {
    let declared = declared_control_paths();
    for (relative, text) in production_rust() {
        // The projection declares where a release lands each payload file,
        // which is versioned data rather than a control path. The bundle
        // phase moves it out of Rust and into the release's own
        // declaration; until then it is the one file that carries a
        // destination literal.
        if relative == "src/domain/paths.rs" || relative == "src/domain/profile.rs" {
            continue;
        }
        for path in &declared {
            let literal = format!("\"{path}\"");
            assert!(
                !text.contains(&literal),
                "{relative} declares {path} a second time; read it from domain::paths"
            );
        }
    }
}

/// SATISFIES distribution:a-skill-package-is-self-contained
///
/// A skill names a supporting file the way the package lands it, relative to
/// the skill's own root. A shared artifact renamed without its references
/// fails here rather than at the first agent that cannot open a gate.
#[test]
fn every_skill_names_its_gates_relative_to_its_own_root() {
    use spec_driven_docs::domain::paths::{SKILL_FILE, SKILL_REFERENCES_DIR};

    let package =
        spec_driven_docs::embedded::skill_package("sdd-setup").expect("sdd-setup is embedded");
    let references: Vec<String> = package
        .iter()
        .map(|(path, _)| path.clone())
        .filter(|path| path != SKILL_FILE)
        .collect();
    assert!(!references.is_empty(), "the package carries no references");
    for (relative, _) in &package {
        assert!(
            relative == SKILL_FILE || relative.starts_with(&format!("{SKILL_REFERENCES_DIR}/")),
            "{relative} is neither the manual nor a reference"
        );
    }

    for (dir, text) in skill_dirs() {
        for reference in &references {
            assert!(
                text.contains(reference.as_str()),
                "{dir}: does not name {reference}, which its package carries"
            );
        }
        assert!(
            !text.contains("skills/shared"),
            "{dir}: still names the retired shared root"
        );
    }
}

/// SATISFIES bundle:a-release-declares-what-it-lands
///
/// The declaration is a snapshot: it replaced constants a compiler held to
/// their shape, so a test holds it to its shape instead. A row added or
/// dropped fails here and is reviewed as the projection change it is.
#[test]
fn the_declaration_is_the_projection_this_release_lands() {
    use spec_driven_docs::domain::profile::{DECLARATION, DocsRoot, ProfileId};

    assert_eq!(
        DECLARATION.docs_root(ProfileId::Codebase),
        Some(DocsRoot::Docs)
    );
    assert_eq!(
        DECLARATION.docs_root(ProfileId::KnowledgeBase),
        Some(DocsRoot::UnderscoreDocs)
    );
    assert_eq!(DECLARATION.managed.len(), 3, "the managed set moved");
    assert_eq!(DECLARATION.adopted.len(), 21, "the adopted set moved");
    assert_eq!(DECLARATION.canon_templates.len(), 2);
    assert_eq!(DECLARATION.sentinels.len(), 2);
    for entry in &DECLARATION.sentinels {
        assert!(
            DECLARATION
                .adopted
                .iter()
                .any(|adopted| adopted.source == entry.source),
            "the sentinel {} is owned by no adopted projection",
            entry.rule
        );
    }
}

/// SATISFIES bundle:a-release-declares-what-it-lands
#[test]
fn the_declaration_roots_equal_the_payload_roots() {
    use spec_driven_docs::domain::profile::DECLARATION;

    for entry in DECLARATION.managed.iter().chain(&DECLARATION.adopted) {
        assert!(
            PAYLOAD_ROOTS
                .iter()
                .any(|root| entry.source.starts_with(&format!("{root}/"))),
            "{} is projected from outside every declared payload root",
            entry.source
        );
        assert!(
            canon().join(&entry.source).is_file(),
            "{} is projected and not on disk",
            entry.source
        );
    }
}

/// SATISFIES staging:the-operator-owns-acquisition
///
/// A landing verb that took a release as an argument would be answering a
/// question the operator's own project manager already answered.
#[test]
fn no_landing_verb_takes_a_release() {
    const SIGNATURES: &[(&str, &str)] = &[
        ("src/services/installer.rs", "pub fn init("),
        ("src/services/upgrader.rs", "pub fn upgrade("),
        ("src/services/verifier.rs", "pub fn verify("),
        ("src/services/assess.rs", "pub fn assess("),
        ("src/stage.rs", "pub fn create("),
    ];
    for (relative, signature) in SIGNATURES {
        let text = read(relative);
        let start = text
            .find(signature)
            .unwrap_or_else(|| panic!("{relative} no longer declares {signature}"));
        let end = text[start..]
            .find(") ->")
            .unwrap_or_else(|| panic!("{relative}: {signature} has no return type"));
        let head = &text[start..start + end];
        assert!(
            !head.contains("ReleaseBundle"),
            "{relative}: {signature} still takes a release bundle"
        );
    }
}

/// SATISFIES staging:the-operator-owns-acquisition
///
/// One function owns every byte a landing writes, and it reads this
/// binary's own sources. A service that rendered bytes of its own would be
/// a second answer to the one question the candidate exists to answer.
#[test]
fn only_the_candidate_renders_the_bytes_a_landing_writes() {
    const LANDING: &[&str] = &[
        "src/services/installer.rs",
        "src/services/upgrader.rs",
        "src/landing/apply.rs",
    ];
    for relative in LANDING {
        let text = read(relative);
        let production = text
            .split_once("#[cfg(test)]")
            .map_or_else(|| text.clone(), |(before, _)| before.to_string());
        assert!(
            !production.contains("crate::embedded::asset("),
            "{relative} reads the payload itself rather than through the candidate"
        );
    }
    assert!(
        read("src/candidate.rs").contains("crate::embedded::asset("),
        "the candidate no longer reads this binary's own sources"
    );
}

/// SATISFIES bundle:a-pre-schema-release-is-cataloged-or-unavailable
///
/// The packaging configuration is what decides this, and it is readable
/// without running anything: a path the manifest excludes is a path the
/// published crate does not carry. `cargo package --list` answers the same
/// question and is consulted where it can, which is a checkout with its
/// git directory. Without one, cargo walks the filesystem instead and
/// skips every hidden entry, so it would report `.markdownlint` missing
/// from a crate that carries it.
#[test]
fn the_published_crate_carries_the_payload() {
    let wanted: Vec<String> = PAYLOAD_ROOTS
        .iter()
        .map(|root| (*root).to_string())
        .collect();

    let manifest = read("Cargo.toml");
    let excluded: Vec<&str> = manifest
        .lines()
        .skip_while(|line| !line.starts_with("exclude = ["))
        .skip(1)
        .take_while(|line| !line.starts_with(']'))
        .map(|line| line.trim().trim_matches(|held| held == '"' || held == ','))
        .collect();
    for root in &wanted {
        let named = format!("/{root}");
        assert!(
            !excluded.contains(&named.as_str()),
            "Cargo.toml excludes {root}, so the published crate would not carry it"
        );
    }

    if !canon().join(".git").exists() {
        return;
    }
    let listed = std::process::Command::new(env!("CARGO"))
        .args(["package", "--list", "--allow-dirty", "--quiet"])
        .current_dir(canon())
        .output()
        .expect("cargo package --list runs");
    assert!(
        listed.status.success(),
        "cargo package --list failed: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    let files = String::from_utf8_lossy(&listed.stdout);
    for root in &wanted {
        assert!(
            files
                .lines()
                .any(|line| line.starts_with(&format!("{root}/"))),
            "the published crate carries no {root}"
        );
    }
}

/// SATISFIES distribution:a-landing-classifies-its-target-first
#[test]
fn the_reading_order_and_the_digest_route_to_landing() {
    assert!(
        read("method/README.md").contains("[Landing](./landing.md)"),
        "the reading order does not name the landing chapter"
    );
    assert!(
        read("method/AGENTS.md").contains("`landing.md`"),
        "the method digest routes nothing to the landing chapter"
    );
    assert!(
        read("AGENTS.md").contains("method/landing.md"),
        "the root digest routes nothing to the landing chapter"
    );
}

/// SATISFIES distribution:a-landing-classifies-its-target-first
#[test]
fn the_migration_chapter_sends_an_installed_instance_to_the_landing_chapter() {
    let chapter = read("method/migration.md");
    assert!(
        chapter.contains("[Landing](./landing.md)"),
        "the migration chapter keeps an installed instance to itself"
    );
}

#[test]
fn the_migration_chapter_names_the_two_proved_finding_kinds_the_unjudged_style_candidates_and_the_scope_decision()
 {
    let chapter = read("method/migration.md");
    for phrase in [
        "structural finding",
        "budget finding",
        "style candidate",
        "`migration-scope`",
        "compliant or noncompliant",
    ] {
        assert!(
            chapter.contains(phrase),
            "the migration chapter does not state '{phrase}'"
        );
    }
}

/// SATISFIES staging:production-reads-no-staged-byte
#[test]
fn the_landing_chapter_states_what_a_stage_is_for() {
    let chapter = read("method/landing.md");
    assert!(
        chapter.contains("The stage is for reading"),
        "the landing chapter does not say what a stage is for"
    );
    assert!(
        chapter.contains("renders the candidate again from scratch"),
        "the landing chapter does not say that production re-renders"
    );
}

/// Every glossary row resolves: the owner column names a document that
/// exists, so a term can never point at a chapter nobody wrote.
#[test]
fn every_glossary_term_names_its_owning_chapter() {
    let glossary = read("method/glossary.md");
    let mut rows = 0;
    for line in glossary.lines() {
        let Some(owner) = line.rsplit('|').nth(1) else {
            continue;
        };
        let owner = owner.trim().trim_matches('`');
        if !owner.ends_with(".md") {
            continue;
        }
        let relative = if owner.contains('/') {
            owner.to_string()
        } else {
            format!("method/{owner}")
        };
        assert!(
            canon().join(&relative).exists(),
            "the glossary names {relative}, which does not exist"
        );
        rows += 1;
    }
    assert!(rows > 40, "the glossary rows did not parse: {rows} found");
}

/// SATISFIES docs-discovery:one-catalog-describes-every-served-document
#[test]
fn every_shelf_document_has_exactly_one_catalog_entry() {
    use spec_driven_docs::domain::docs_catalog::CATALOG;
    use spec_driven_docs::services::reader;

    for shelf in [&reader::METHOD, &reader::SPECS, &reader::TEMPLATES] {
        for name in reader::list(shelf) {
            let held: Vec<&str> = CATALOG
                .topics
                .iter()
                .filter(|topic| {
                    matches!(&topic.target,
                        spec_driven_docs::domain::docs_catalog::Target::Document { shelf: id, name: held }
                            if *id == shelf.id && *held == name)
                })
                .map(|topic| topic.id.as_str())
                .collect();
            assert_eq!(
                held.len(),
                1,
                "{}/{name} has {} catalog entries: {held:?}",
                shelf.id.as_str(),
                held.len()
            );
        }
    }
}

/// SATISFIES docs-discovery:one-catalog-describes-every-served-document
#[test]
fn no_catalog_entry_is_orphaned() {
    use spec_driven_docs::domain::docs_catalog::{CATALOG, Target};
    use spec_driven_docs::services::reader;

    for topic in &CATALOG.topics {
        match &topic.target {
            Target::Document { shelf, name } => {
                let held = match shelf {
                    spec_driven_docs::domain::docs_catalog::ShelfId::Method => &reader::METHOD,
                    spec_driven_docs::domain::docs_catalog::ShelfId::Spec => &reader::SPECS,
                    spec_driven_docs::domain::docs_catalog::ShelfId::Template => &reader::TEMPLATES,
                };
                assert!(
                    reader::get(held, name).is_some(),
                    "{} names {}/{name}, which no shelf serves",
                    topic.id,
                    shelf.as_str()
                );
            }
            Target::Command(argv) => {
                let names = spec_driven_docs::cli::subcommand_names();
                assert!(
                    argv.first().is_some_and(|verb| names.contains(verb)),
                    "{} names an argv whose verb the parser does not offer",
                    topic.id
                );
            }
        }
    }
}

/// SATISFIES docs-discovery:an-instance-is-routed-to-the-index
#[test]
fn the_managed_documentation_block_names_the_index_verb_and_stays_within_its_budget() {
    let snippet = read("instance/snippets/AGENTS-docs.md");
    assert!(
        snippet.contains("sdd docs"),
        "the managed documentation block names no reader verb"
    );
    let lines = snippet.lines().filter(|line| !line.is_empty()).count();
    assert!(
        lines <= 10,
        "the managed documentation block runs {lines} lines against its 10-line budget"
    );
}

/// SATISFIES release:the-binary-builds-for-every-declared-target
///
/// One target is declared, and the ordinary required pull-request job
/// compiles it, so the compiler is the evidence that every declared target
/// builds. What this test holds is the premise that argument rests on: the
/// declaration still names one target and one installer, and the crate root
/// still refuses a build for another operating system.
///
/// A lexical scan used to stand in for the compiler here, looking 20 lines
/// above a `std::os::*` call for a `#[cfg]`. It proved no compilation: a
/// guarded call that would not build still passed, and a correct call whose
/// guard sat 21 lines up still failed. It is gone
/// (ADR-linux-is-the-only-supported-target).
#[test]
fn the_declaration_names_one_target_and_the_crate_refuses_the_rest() {
    let text = std::fs::read_to_string(canon().join("dist-workspace.toml")).unwrap();
    let declaration: toml::Value = toml::from_str(&text).unwrap();
    let dist = declaration
        .get("dist")
        .expect("dist-workspace.toml carries no [dist] table");

    let list = |key: &str| -> Vec<String> {
        dist.get(key)
            .and_then(toml::Value::as_array)
            .unwrap_or_else(|| panic!("dist-workspace.toml declares no {key}"))
            .iter()
            .map(|held| held.as_str().unwrap_or_default().to_string())
            .collect()
    };

    assert_eq!(
        list("targets"),
        vec!["x86_64-unknown-linux-gnu".to_string()],
        "dist-workspace.toml declares a target set other than the one supported target"
    );
    assert_eq!(
        list("installers"),
        vec!["shell".to_string()],
        "dist-workspace.toml declares an installer set other than the one supported installer"
    );

    // Two independent substring searches used to stand here, and both stayed
    // true when the whole refusal was commented out: `//` in front of a line
    // does not remove its text. The attribute and the macro are matched as
    // adjacent live lines instead, which is what `#[cfg]` means — it governs
    // the item that follows it.
    //
    // The line above the attribute is checked too. A form may carry several
    // `cfg` attributes and Rust removes it when any predicate is false, so a
    // `#[cfg(target_os = "linux")]` stacked on top would delete the refusal
    // on every operating system while leaving the pair below it intact.
    let root = std::fs::read_to_string(canon().join("src/lib.rs")).unwrap();
    let live: Vec<&str> = root
        .lines()
        .map(str::trim_start)
        .filter(|line| !line.is_empty())
        .collect();
    let attached = live.windows(2).enumerate().any(|(at, pair)| {
        let stacked = at
            .checked_sub(1)
            .and_then(|above| live.get(above))
            .is_some_and(|above| above.starts_with("#[cfg"));
        pair[0] == r#"#[cfg(not(target_os = "linux"))]"#
            && pair[1].starts_with("compile_error!")
            && !stacked
    });
    assert!(
        attached,
        "src/lib.rs carries no live compile-time refusal for an unsupported \
         operating system: the #[cfg(not(target_os = \"linux\"))] attribute \
         must sit directly on a compile_error! invocation, with no further \
         cfg above it that could remove the item"
    );
}

/// SATISFIES release:the-binary-builds-for-every-declared-target
///
/// The Nix output set is the second half of the support boundary, and the
/// document scan cannot hold it: a reintroduced `eachDefaultSystem` names no
/// triple, and `packages.aarch64-linux.default` spells its system
/// differently from any Rust target. `nix flake check` does not close the
/// gap either, because without `--all-systems` it checks the system it runs
/// on and says nothing about the others the flake exposes.
///
/// So the flake is read here. One system, bound once, named on every output
/// through that one binding, and no mapping over a system set.
///
/// This holds one canonical authored form and nothing more. It reads text,
/// so a nested attribute set — `packages = { ${other}.default = ...; }` —
/// declares an output this scan never sees, and no amount of text matching
/// closes that, because Nix has more than one spelling for the same tree.
///
/// The evaluated guarantee lives in CI, where Nix exists:
/// `scripts/check-one-system.sh` asks the evaluator which systems the flake
/// exposes and refuses any set but the one supported system. That is the
/// check that holds the boundary. This one keeps the authored file in the
/// shape a reader expects, and fails fast without Nix installed.
#[test]
fn the_flake_declares_one_system() {
    /// The output families whose first key is a system.
    const FAMILIES: [&str; 5] = ["packages.", "devShells.", "checks.", "formatter.", "apps."];
    /// The one key any of those families may carry.
    const ONLY: &str = "${system}";

    let flake = std::fs::read_to_string(canon().join("flake.nix")).unwrap();

    assert!(
        flake.contains(r#"system = "x86_64-linux";"#),
        "flake.nix does not bind the one supported Nix system"
    );
    for mapping in [
        "eachDefaultSystem",
        "eachSystem",
        "forAllSystems",
        "genAttrs",
    ] {
        assert!(
            !flake.contains(mapping),
            "flake.nix maps its outputs over a system set through {mapping}; \
             one system is declared, so the outputs name it directly"
        );
    }

    // Every system key is compared against the one binding, rather than
    // filtered for literals. An interpolation used to be skipped outright,
    // so `packages.${otherSystem}.default` passed while exposing a second
    // system; Nix takes an interpolated attribute name as ordinary syntax.
    // A key ends at the next `.`, at whitespace, or at the `=` that opens its
    // value, so `checks.${system} = {` yields `${system}` and not the rest of
    // the line.
    let keys: std::collections::BTreeSet<&str> = FAMILIES
        .iter()
        .flat_map(|family| flake.match_indices(family))
        .map(|(at, family)| {
            let rest = &flake[at + family.len()..];
            let end = rest
                .find(['.', ' ', '\t', '\n', '=', ';'])
                .unwrap_or(rest.len());
            &rest[..end]
        })
        .filter(|held| *held != ONLY)
        .collect();
    assert!(
        keys.is_empty(),
        "flake.nix keys an output by something other than {ONLY}, so it can \
         expose a system beside the declared one: {keys:?}"
    );
}

/// SATISFIES release:the-binary-builds-for-every-declared-target
///
/// The support boundary is also a claim, and a claim outlives the code that
/// made it. This walks the live product surface for an artifact the project
/// used to build, so a README line or a spec sentence cannot keep
/// advertising one after its target left.
///
/// The list is finite and closed, and the name says so. It holds the four
/// retired triples and the two spellings of the retired installer, which is
/// every artifact this project ever shipped and no longer does. A document
/// inventing support for a target the project never built is a different
/// defect, which no list of retired names can catch and which review does.
///
/// The exemptions each have their own reason. Decision records and the
/// changelog are history, and are the only zones allowed to name what was
/// retired. `Cargo.lock` and `flake.lock` record a resolved graph rather
/// than a support claim, and `deny.toml` names a transitive crate inside
/// it. This file names the claims it searches for. The generated workflow
/// has its own check below.
#[test]
fn no_live_document_advertises_a_retired_artifact() {
    const CLAIMS: [&str; 6] = [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "aarch64-unknown-linux-gnu",
        "installer.ps1",
        "powershell",
    ];
    const EXEMPT: [&str; 5] = [
        "CHANGELOG.md",
        "Cargo.lock",
        "flake.lock",
        "deny.toml",
        "tests/canon.rs",
    ];

    for entry in walkdir::WalkDir::new(canon())
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            name != ".git" && name != "target" && name != ".direnv" && name != ".docs-scratch"
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let relative = entry
            .path()
            .strip_prefix(canon())
            .unwrap_or_else(|_| entry.path())
            .to_string_lossy()
            .to_string();
        if relative.starts_with("_docs/decisions/")
            || relative.starts_with(".github/workflows/release.yml")
            || EXEMPT.contains(&relative.as_str())
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let lowered = text.to_lowercase();
        for claim in CLAIMS {
            assert!(
                !lowered.contains(claim),
                "{relative} names {claim}, a platform this project does not support"
            );
        }
    }
}

/// SATISFIES release:the-binary-builds-for-every-declared-target
///
/// The generated workflow is the file the forge executes, so it gets its own
/// check rather than an exemption. The pinned generator computes the build
/// matrix at run time from the committed declaration, so the workflow names
/// no triple and no installer artifact of its own, and the platform words
/// its boilerplate does carry — a `core.longpaths` git config and a comment
/// about GitHub's env-var syntax — advertise nothing. A regenerated file
/// that named an artifact below would mean the declaration grew a target.
#[test]
fn the_generated_workflow_names_no_unsupported_artifact() {
    let workflow = std::fs::read_to_string(canon().join(".github/workflows/release.yml")).unwrap();
    for claim in [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "aarch64-unknown-linux-gnu",
        "installer.ps1",
    ] {
        assert!(
            !workflow.contains(claim),
            ".github/workflows/release.yml names {claim}; regenerate it from dist-workspace.toml"
        );
    }
}

/// SATISFIES release:a-machine-scope-recipe-drops-a-session-variable
///
/// The whole behavior is two words in a recipe, so a refactor that tidies the
/// recipe removes it silently and every other test stays green: the binary
/// tests prove the opposite default on purpose, because the verb honours the
/// variable everywhere else. This is the only thing standing between a
/// machine-scope install and one terminal's isolated configuration directory.
///
/// The variable is matched by name rather than by the whole `env` prefix, so
/// a later recipe that drops it another way still passes. What must not
/// happen is the skill step reaching `sdd` with the value inherited.
#[test]
fn the_install_recipes_drop_the_session_configuration_variable() {
    let justfile = read("justfile");
    for verb in ["install", "uninstall"] {
        let line = justfile
            .lines()
            .find(|line| line.contains(&format!("sdd skill {verb}")))
            .unwrap_or_else(|| panic!("the {verb} recipe no longer runs sdd skill {verb}"));
        assert!(
            line.contains("CLAUDE_CONFIG_DIR"),
            "the {verb} recipe inherits CLAUDE_CONFIG_DIR, so a run inside a session \
             wrapper would target that one terminal rather than the user's own roots: {line}"
        );
    }
}
