//! The pin each manager records, read and rewritten in its own form.
//!
//! ```text
//! flake input      github:<owner>/<repo>/<tag> in flake.nix, locked in flake.lock
//! mise, registry   "cargo:<crate>" = "<version>"
//! mise, release    "ubi:<owner>/<repo>" = { version = "<version>", exe = "<binary>" }
//! asdf             <tool> <version>, one line in .tool-versions
//! devbox           "github:<owner>/<repo>/<tag>#<output>", one entry in packages
//! ```
//!
//! A registry entry names the crate as the registry knows it, and an archive
//! entry names the binary inside the archive. The reader below accepts each
//! form where its manager writes it and nowhere else.

use semver::Version;
use serde::Serialize;

use crate::self_depend::manager::Manager;
use crate::self_depend::registry::CRATE_NAME;
use crate::self_depend::venue::Venue;
use crate::self_depend::{BINARY_NAME, slug};

/// One recorded pin, located in its file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pin {
    /// The released triple the pin names.
    pub version: Version,
    /// The version as the file spells it, `v` prefix included where present.
    pub spelled: String,
    /// The venue the entry's form selects, where the form says.
    pub venue: Option<Venue>,
    /// The one-based line the pin sits on.
    pub line: usize,
    /// How many lines in the file pin this tool.
    pub lines: usize,
    /// The byte span of the spelled version in the file text.
    #[serde(skip)]
    pub span: (usize, usize),
}

/// The pin a manager file records for this tool, where it records one.
#[must_use]
pub fn read(manager: Manager, text: &str) -> Option<Pin> {
    let mut found: Vec<Pin> = Vec::new();
    let mut offset = 0;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        if let Some((start, end, venue)) = locate(manager, line) {
            let spelled = line[start..end].to_string();
            let bare = spelled.strip_prefix('v').unwrap_or(&spelled);
            if let Ok(version) = bare.parse::<Version>() {
                found.push(Pin {
                    version,
                    spelled,
                    venue,
                    line: index + 1,
                    lines: 0,
                    span: (offset + start, offset + end),
                });
            }
        }
        offset += line.len();
    }
    let lines = found.len();
    let mut first = found.into_iter().next()?;
    first.lines = lines;
    Some(first)
}

/// Where one line spells the version, and which venue the form selects.
fn locate(manager: Manager, line: &str) -> Option<(usize, usize, Option<Venue>)> {
    match manager {
        Manager::Flake => {
            let needle = format!("github:{}/", slug());
            let start = line.find(&needle)? + needle.len();
            let end = start
                + line[start..]
                    .find(['"', '#', '?', '\''])
                    .unwrap_or(line.len() - start);
            (end > start).then_some((start, end, Some(Venue::Flake)))
        }
        Manager::Devbox => {
            let needle = format!("github:{}/", slug());
            let start = line.find(&needle)? + needle.len();
            let end = start + line[start..].find(['#', '"']).unwrap_or(line.len() - start);
            (end > start).then_some((start, end, Some(Venue::Flake)))
        }
        Manager::Mise => {
            let trimmed = line.trim_start();
            let registry = format!("\"cargo:{CRATE_NAME}\"");
            let archive = format!("\"ubi:{}\"", slug());
            if trimmed.starts_with(&registry) {
                let (start, end) = quoted_after(line, "=")?;
                return Some((start, end, Some(Venue::Crates)));
            }
            if trimmed.starts_with(&archive) {
                let (start, end) = quoted_after(line, "version")?;
                return Some((start, end, Some(Venue::GithubRelease)));
            }
            None
        }
        Manager::Asdf => {
            let mut parts = line.split_whitespace();
            let tool = parts.next()?;
            if tool != CRATE_NAME && tool != BINARY_NAME {
                return None;
            }
            let version = parts.next()?;
            let start = line.find(version)?;
            Some((start, start + version.len(), None))
        }
    }
}

/// The span of the first quoted string after `marker` in `line`.
fn quoted_after(line: &str, marker: &str) -> Option<(usize, usize)> {
    let at = line.find(marker)? + marker.len();
    let open = at + line[at..].find('"')? + 1;
    let close = open + line[open..].find('"')?;
    Some((open, close))
}

/// The same file text with the pin moved to `to`, in the spelling the file
/// already uses.
///
/// # Errors
///
/// A message where the file no longer records the pin it was read with.
pub fn rewrite(manager: Manager, text: &str, to: &Version) -> Result<(String, Pin), String> {
    let held = read(manager, text).ok_or_else(|| "the file no longer records a pin".to_string())?;
    let spelled = if held.spelled.starts_with('v') {
        format!("v{to}")
    } else {
        to.to_string()
    };
    let mut moved = String::with_capacity(text.len());
    moved.push_str(&text[..held.span.0]);
    moved.push_str(&spelled);
    moved.push_str(&text[held.span.1..]);
    Ok((moved, held))
}

/// The deepest chain of follows paths [`input_node`] walks before it gives
/// up: a lock that loops reads as one naming no input.
const MAX_FOLLOWS: usize = 32;

/// The revision `flake.lock` holds for this tool's input, where it holds one.
///
/// A lock names a node after the input that first introduced it, so a
/// dependency's own input on this tool can hold the plain name while the
/// project's input reads `spec-driven-docs_2`. The root node, which the
/// lock's `root` key names, is the one owner of the project's inputs, so
/// the revision is read from the node one of its inputs resolves to. That
/// input is found by the repository its node names, never by its name.
#[must_use]
pub fn locked_rev(lock: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(lock).ok()?;
    let (_, node) = own_input(&value)?;
    value["nodes"][node.as_str()]["locked"]["rev"]
        .as_str()
        .map(str::to_string)
}

/// Whether a lock node names this tool's repository.
fn names_this_tool(node: &serde_json::Value) -> bool {
    let (owner, repo) = crate::self_depend::coordinates();
    let original = &node["original"];
    let same = |key: &str, want: &str| {
        original[key]
            .as_str()
            .is_some_and(|held| held.eq_ignore_ascii_case(want))
    };
    same("owner", owner) && same("repo", repo)
}

/// The root input that resolves to this tool, and the node it resolves to.
fn own_input(value: &serde_json::Value) -> Option<(String, String)> {
    let nodes = &value["nodes"];
    let root = value["root"].as_str().unwrap_or("root");
    let inputs = nodes[root]["inputs"].as_object()?;
    inputs.keys().find_map(|input| {
        let name = input_node(nodes, root, root, input, 0)?;
        names_this_tool(&nodes[name.as_str()]).then(|| (input.clone(), name))
    })
}

/// The inputs of this tool's own flake that only its development shell reads.
///
/// The package never reads them. A consumer that lacks one points it at its
/// own flake with an empty follows, so the lock holds no node for it. Read
/// from this repository's `flake.nix`: the package output builds from
/// `nixpkgs` and `rust-overlay` alone.
pub const DEVELOPMENT_ONLY_INPUTS: [&str; 2] = ["release-kit", "exobrain-tech"];

/// One follows line a consumer's flake lacks on this tool's input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Follows {
    /// The consumer's input that pins this tool, which holds the line.
    pub input: String,
    /// The nested input the line points.
    pub nested: String,
    /// The line, as `flake.nix` carries it inside the input's attribute set.
    pub line: String,
}

/// The follows lines that remove a nested copy from a consumer's lock.
///
/// For each input of this tool's node that still holds its own node: this
/// tool's own previous release follows nothing, an input the root also
/// declares follows the root's, and a development-only input the root
/// lacks follows nothing. Every other input keeps its node, because the
/// package build needs it. The lock grammar alone decides, and nothing is
/// written.
///
/// SATISFIES acquisition:sync-advises-the-follows-lines-and-writes-none
#[must_use]
pub fn follows_advice(lock: &str) -> Vec<Follows> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(lock) else {
        return Vec::new();
    };
    let Some((input, node)) = own_input(&value) else {
        return Vec::new();
    };
    let nodes = &value["nodes"];
    let root = value["root"].as_str().unwrap_or("root");
    let declared = |name: &str| nodes[root]["inputs"].get(name).is_some();
    let Some(nested) = nodes[node.as_str()]["inputs"].as_object() else {
        return Vec::new();
    };
    nested
        .iter()
        .filter_map(|(name, held)| {
            // A follows path is a list: the line is already there.
            let own = held.as_str()?;
            let target = if names_this_tool(&nodes[own]) {
                String::new()
            } else if declared(name) {
                name.clone()
            } else if DEVELOPMENT_ONLY_INPUTS.contains(&name.as_str()) {
                String::new()
            } else {
                return None;
            };
            Some(Follows {
                input: input.clone(),
                nested: name.clone(),
                line: format!("inputs.{name}.follows = \"{target}\";"),
            })
        })
        .collect()
}

/// The node one input of `node` resolves to. The input names a node, or
/// holds a follows path: a list of input names walked from the root node.
fn input_node(
    nodes: &serde_json::Value,
    root: &str,
    node: &str,
    input: &str,
    depth: usize,
) -> Option<String> {
    match &nodes[node]["inputs"][input] {
        serde_json::Value::String(name) => Some(name.clone()),
        serde_json::Value::Array(path) if depth < MAX_FOLLOWS => {
            let mut at = root.to_owned();
            for step in path {
                at = input_node(nodes, root, &at, step.as_str()?, depth + 1)?;
            }
            Some(at)
        }
        _ => None,
    }
}

/// The name of the flake input whose URL names this tool.
///
/// The input is found by the URL it names, never by its name: a consumer
/// calls the input whatever it likes. The name is read from the attribute
/// that holds the URL, either `name.url = "..."` or `name = { url = ... }`.
#[must_use]
pub fn input_name(flake: &str) -> Option<String> {
    let needle = format!("github:{}/", slug());
    let lines: Vec<&str> = flake.lines().collect();
    let at = lines.iter().position(|line| line.contains(&needle))?;
    if let Some(name) = attribute_before(lines[at], ".url") {
        return Some(name);
    }
    lines[..at]
        .iter()
        .rev()
        .find_map(|line| attribute_before(line, " ="))
        .or_else(|| {
            lines[..at]
                .iter()
                .rev()
                .find_map(|line| attribute_before(line, "="))
        })
}

/// The identifier that opens `line` before `suffix`, where the line reads
/// `  <ident><suffix>...` and the identifier is an attribute name.
fn attribute_before(line: &str, suffix: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let end = trimmed.find(suffix)?;
    let name = &trimmed[..end];
    let is_ident = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if !is_ident || name == "url" || name == "inputs" {
        return None;
    }
    // `name = {` opens an attribute set; `name = "..."` does not hold a URL.
    if suffix != ".url"
        && !trimmed[end..]
            .trim_start_matches(suffix)
            .trim_start()
            .starts_with('{')
    {
        return None;
    }
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test panics as its failure signal")]

    use super::*;

    fn flake() -> String {
        format!(
            "{{\n  inputs = {{\n    nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\";\n    sdd = {{\n      url = \"github:{}/v0.10.1\";\n      inputs.nixpkgs.follows = \"nixpkgs\";\n    }};\n  }};\n}}\n",
            slug()
        )
    }

    /// VERIFIES acquisition:a-fragment-names-the-venue-form
    #[test]
    fn a_flake_pin_reads_and_rewrites_in_place() {
        let text = flake();
        let pin = read(Manager::Flake, &text).unwrap();
        assert_eq!(pin.version.to_string(), "0.10.1");
        assert_eq!(pin.spelled, "v0.10.1");
        assert_eq!(pin.venue, Some(Venue::Flake));
        assert_eq!(pin.line, 5);
        assert_eq!(pin.lines, 1);
        let (moved, _) = rewrite(Manager::Flake, &text, &"0.10.2".parse().unwrap()).unwrap();
        assert!(moved.contains(&format!("github:{}/v0.10.2\"", slug())));
        assert_eq!(moved.len(), text.len());
        assert_eq!(input_name(&text).as_deref(), Some("sdd"));
    }

    #[test]
    fn a_dotted_flake_input_names_itself_on_the_url_line() {
        let text = format!(
            "{{ inputs = {{\n  tool.url = \"github:{}/v0.9.0\";\n }}; }}\n",
            slug()
        );
        assert_eq!(input_name(&text).as_deref(), Some("tool"));
    }

    #[test]
    fn a_mise_pin_reads_both_forms() {
        let registry = format!("[tools]\n\"cargo:{CRATE_NAME}\" = \"0.10.1\"\n");
        let pin = read(Manager::Mise, &registry).unwrap();
        assert_eq!(pin.venue, Some(Venue::Crates));
        assert_eq!(pin.spelled, "0.10.1");
        let archive = format!(
            "[tools]\n\"ubi:{}\" = {{ version = \"0.10.1\", exe = \"{BINARY_NAME}\" }}\n",
            slug()
        );
        let pin = read(Manager::Mise, &archive).unwrap();
        assert_eq!(pin.venue, Some(Venue::GithubRelease));
        let (moved, _) = rewrite(Manager::Mise, &archive, &"0.11.0".parse().unwrap()).unwrap();
        assert!(moved.contains("version = \"0.11.0\""));
    }

    #[test]
    fn an_asdf_pin_reads_the_second_token() {
        let text = format!("nodejs 20.0.0\n{CRATE_NAME} 0.10.1\n");
        let pin = read(Manager::Asdf, &text).unwrap();
        assert_eq!(pin.version.to_string(), "0.10.1");
        assert_eq!(pin.venue, None);
        assert!(read(Manager::Asdf, "nodejs 20.0.0\n").is_none());
    }

    #[test]
    fn a_devbox_pin_reads_up_to_the_output() {
        let text = format!(
            "{{ \"packages\": [\"github:{}/v0.10.1#default\"] }}\n",
            slug()
        );
        let pin = read(Manager::Devbox, &text).unwrap();
        assert_eq!(pin.spelled, "v0.10.1");
        let (moved, _) = rewrite(Manager::Devbox, &text, &"0.10.2".parse().unwrap()).unwrap();
        assert!(moved.contains("v0.10.2#default"));
    }

    #[test]
    fn the_lock_revision_is_found_by_the_repository_it_names() {
        let (owner, repo) = crate::self_depend::coordinates();
        let lock = format!(
            "{{\"nodes\":{{\"root\":{{\"inputs\":{{\"sdd\":\"sdd\"}}}},\"sdd\":{{\"locked\":{{\"rev\":\"abc123\"}},\"original\":{{\"owner\":\"{owner}\",\"repo\":\"{repo}\",\"type\":\"github\"}}}}}},\"version\":7}}"
        );
        assert_eq!(locked_rev(&lock).as_deref(), Some("abc123"));
        assert_eq!(locked_rev("{\"nodes\":{}}"), None);
    }

    /// A node that names this tool's repository, locked at `rev`.
    fn node(rev: &str) -> String {
        let (owner, repo) = crate::self_depend::coordinates();
        format!(
            "{{\"locked\":{{\"rev\":\"{rev}\"}},\"original\":{{\"owner\":\"{owner}\",\"repo\":\"{repo}\",\"type\":\"github\"}}}}"
        )
    }

    #[test]
    fn a_nested_node_does_not_shadow_the_root_input() {
        let lock = format!(
            "{{\"nodes\":{{\"exobrain-tech\":{{\"inputs\":{{\"spec-driven-docs\":\"spec-driven-docs\"}}}},\"spec-driven-docs\":{},\"spec-driven-docs_2\":{},\"top\":{{\"inputs\":{{\"exobrain-tech\":\"exobrain-tech\",\"sdd\":\"spec-driven-docs_2\"}}}}}},\"root\":\"top\",\"version\":7}}",
            node("nested"),
            node("own")
        );
        assert_eq!(locked_rev(&lock).as_deref(), Some("own"));
    }

    #[test]
    fn a_followed_root_input_resolves_through_its_path() {
        let lock = format!(
            "{{\"nodes\":{{\"exobrain-tech\":{{\"inputs\":{{\"spec-driven-docs\":\"spec-driven-docs_2\"}}}},\"spec-driven-docs_2\":{},\"root\":{{\"inputs\":{{\"exobrain-tech\":\"exobrain-tech\",\"sdd\":[\"exobrain-tech\",\"spec-driven-docs\"]}}}}}},\"root\":\"root\",\"version\":7}}",
            node("followed")
        );
        assert_eq!(locked_rev(&lock).as_deref(), Some("followed"));
        let chained = format!(
            "{{\"nodes\":{{\"a\":{{\"inputs\":{{\"tool\":[\"b\",\"tool\"]}}}},\"b\":{{\"inputs\":{{\"tool\":\"tool_3\"}}}},\"tool_3\":{},\"root\":{{\"inputs\":{{\"a\":\"a\",\"b\":\"b\",\"sdd\":[\"a\",\"tool\"]}}}}}},\"version\":7}}",
            node("chained")
        );
        assert_eq!(locked_rev(&chained).as_deref(), Some("chained"));
    }

    #[test]
    fn a_looped_or_broken_follows_path_names_no_input() {
        let looped = format!(
            "{{\"nodes\":{{\"spec-driven-docs\":{},\"root\":{{\"inputs\":{{\"sdd\":[\"sdd\"]}}}}}},\"version\":7}}",
            node("never")
        );
        assert_eq!(locked_rev(&looped), None, "a loop names no input");
        let broken = format!(
            "{{\"nodes\":{{\"spec-driven-docs\":{},\"root\":{{\"inputs\":{{\"sdd\":[\"absent\",\"sdd\"]}}}}}},\"version\":7}}",
            node("never")
        );
        assert_eq!(locked_rev(&broken), None);
    }

    #[test]
    fn a_node_the_root_does_not_reach_is_not_the_project_input() {
        let lock = format!(
            "{{\"nodes\":{{\"spec-driven-docs\":{},\"root\":{{\"inputs\":{{\"nixpkgs\":\"nixpkgs\"}}}},\"nixpkgs\":{{\"locked\":{{\"rev\":\"n1\"}}}}}},\"version\":7}}",
            node("stray")
        );
        assert_eq!(locked_rev(&lock), None);
    }

    /// A lock whose root pins this tool as `sdd`, with `root_extra` more
    /// root inputs and `nested` the inputs of this tool's node.
    fn advice_lock(root_extra: &[&str], nested: &str, extra_nodes: &str) -> String {
        let (owner, repo) = crate::self_depend::coordinates();
        let root: String = std::iter::once("\"sdd\":\"sdd\"".to_string())
            .chain(
                root_extra
                    .iter()
                    .map(|name| format!("\"{name}\":\"{name}\"")),
            )
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"nodes\":{{\"root\":{{\"inputs\":{{{root}}}}},\"sdd\":{{\"inputs\":{{{nested}}},\"locked\":{{\"rev\":\"r\"}},\"original\":{{\"owner\":\"{owner}\",\"repo\":\"{repo}\",\"type\":\"github\"}}}}{extra_nodes}}},\"root\":\"root\",\"version\":7}}"
        )
    }

    fn lines(advice: &[Follows]) -> Vec<&str> {
        advice.iter().map(|follows| follows.line.as_str()).collect()
    }

    /// VERIFIES acquisition:sync-advises-the-follows-lines-and-writes-none
    #[test]
    fn a_nested_input_the_root_declares_follows_the_roots() {
        let lock = advice_lock(
            &["release-kit"],
            "\"release-kit\":\"release-kit_2\"",
            ",\"release-kit\":{},\"release-kit_2\":{}",
        );
        let advice = follows_advice(&lock);
        assert_eq!(
            lines(&advice),
            ["inputs.release-kit.follows = \"release-kit\";"]
        );
        assert_eq!(advice[0].input, "sdd");
    }

    /// VERIFIES acquisition:sync-advises-the-follows-lines-and-writes-none
    #[test]
    fn a_development_only_input_the_root_lacks_follows_nothing() {
        let lock = advice_lock(
            &[],
            "\"exobrain-tech\":\"exobrain-tech\",\"release-kit\":\"release-kit\"",
            ",\"exobrain-tech\":{},\"release-kit\":{}",
        );
        assert_eq!(
            lines(&follows_advice(&lock)),
            [
                "inputs.exobrain-tech.follows = \"\";",
                "inputs.release-kit.follows = \"\";"
            ]
        );
    }

    /// VERIFIES acquisition:sync-advises-the-follows-lines-and-writes-none
    #[test]
    fn a_nested_nixpkgs_the_root_lacks_gets_no_advice() {
        let lock = advice_lock(
            &[],
            "\"nixpkgs\":\"nixpkgs\",\"rust-overlay\":\"rust-overlay\"",
            ",\"nixpkgs\":{},\"rust-overlay\":{}",
        );
        assert!(follows_advice(&lock).is_empty());
    }

    /// VERIFIES acquisition:sync-advises-the-follows-lines-and-writes-none
    #[test]
    fn a_nested_older_release_of_this_tool_follows_nothing() {
        let (owner, repo) = crate::self_depend::coordinates();
        let lock = advice_lock(
            &[],
            "\"spec-driven-docs\":\"spec-driven-docs\"",
            &format!(
                ",\"spec-driven-docs\":{{\"original\":{{\"owner\":\"{owner}\",\"repo\":\"{repo}\",\"type\":\"github\"}}}}"
            ),
        );
        assert_eq!(
            lines(&follows_advice(&lock)),
            ["inputs.spec-driven-docs.follows = \"\";"]
        );
    }

    /// VERIFIES acquisition:sync-advises-the-follows-lines-and-writes-none
    #[test]
    fn a_lock_with_every_follows_line_in_place_advises_nothing() {
        let lock = advice_lock(
            &["nixpkgs", "release-kit"],
            "\"nixpkgs\":[\"nixpkgs\"],\"release-kit\":[\"release-kit\"],\"exobrain-tech\":[],\"spec-driven-docs\":[]",
            ",\"nixpkgs\":{},\"release-kit\":{}",
        );
        assert!(follows_advice(&lock).is_empty());
        assert!(follows_advice("not json").is_empty());
    }
}
