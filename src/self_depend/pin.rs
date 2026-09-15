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

/// The revision `flake.lock` holds for this tool's input, where it holds one.
#[must_use]
pub fn locked_rev(lock: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(lock).ok()?;
    let (owner, repo) = crate::self_depend::coordinates();
    let nodes = value.get("nodes")?.as_object()?;
    nodes.values().find_map(|node| {
        let original = node.get("original")?;
        let same = |key: &str, want: &str| {
            original
                .get(key)
                .and_then(serde_json::Value::as_str)
                .is_some_and(|held| held.eq_ignore_ascii_case(want))
        };
        (same("owner", owner) && same("repo", repo))
            .then(|| node.get("locked")?.get("rev")?.as_str().map(str::to_string))
            .flatten()
    })
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
}
