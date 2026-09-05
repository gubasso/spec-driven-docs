//! The tracking registry: the parsed, bounded shape of a perishable-facts file.
//!
//! The registry is `<root>/reference/tracking.yaml`: one YAML document,
//! `schema_version: 1`, a `tracked` array. This module owns the versioned
//! shape and the bounds that must hold before any semantic read, because the
//! file is repository-controlled input a gate parses on every commit. What a
//! path resolves to on disk, and whether an entry is overdue against a clock,
//! is `services::tracking`'s business.

use serde::Deserialize;
use thiserror::Error;

/// The schema version this binary reads.
pub const SCHEMA_VERSION: u32 = 1;

/// Bounds on untrusted input, applied before deserialization.
const MAX_BYTES: usize = 256 * 1024;
const MAX_LINES: usize = 5_000;
const MAX_LINE_LEN: usize = 4_096;
const MAX_INDENT_SPACES: usize = 64;
const MAX_ENTRIES: usize = 1_000;

/// A parsed registry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    /// Always [`SCHEMA_VERSION`] once accepted.
    pub schema_version: u32,
    /// Every tracked source.
    pub tracked: Vec<Entry>,
}

/// One tracked perishable source.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// A unique slug.
    pub id: String,
    /// The repository-relative path of the tracked document.
    pub path: String,
    /// The ISO date the source was last checked.
    pub last_checked: String,
    /// How many days between checks.
    pub cadence_days: u32,
    /// Why the source expires.
    pub why: String,
    /// Ordered steps a person follows to revalidate the source.
    pub revalidate: Vec<String>,
    /// The local files that depend on the source.
    pub dependents: Vec<String>,
    /// The upstream Git derivation, where the source is one.
    #[serde(default)]
    pub source: Option<Source>,
}

/// An upstream Git derivation an entry pins.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// Always `git` for schema version 1.
    pub kind: String,
    /// A credential-free `https://` repository URL.
    pub repository: String,
    /// A full `refs/...` reference.
    pub reference: String,
    /// A full 40- or 64-character Git object ID.
    pub revision: String,
    /// The upstream license identifier.
    pub license: String,
}

/// Why a registry cannot be accepted.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TrackingError {
    /// A bound on untrusted input was exceeded.
    #[error("{0}")]
    Bounds(String),
    /// The document does not parse as the schema-version-1 shape.
    #[error("invalid tracking registry: {0}")]
    Shape(String),
    /// A cross-field or uniqueness rule the schema cannot express.
    #[error("{0}")]
    Semantic(String),
}

/// True for a full 40- (SHA-1) or 64-character (SHA-256) hex object ID.
#[must_use]
pub fn is_object_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64)
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Reject the input before deserialization if any bound is exceeded.
///
/// The scan is line-based and quote- and comment-aware only enough to keep a
/// hazard token from hiding: a scalar's contents are not YAML control input.
fn check_bounds(text: &str) -> Result<(), TrackingError> {
    let bound = |m: String| Err(TrackingError::Bounds(m));
    if text.len() > MAX_BYTES {
        return bound(format!("registry is larger than {MAX_BYTES} bytes"));
    }
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() > MAX_LINES {
        return bound(format!("registry has more than {MAX_LINES} lines"));
    }
    // Sibling-scope duplicate-key detection: a stack of (indent, keys-seen).
    let mut scopes: Vec<(usize, std::collections::BTreeSet<String>)> = Vec::new();
    for (number, raw) in lines.iter().enumerate() {
        let line = raw.trim_end();
        if line.len() > MAX_LINE_LEN {
            return bound(format!(
                "line {} is longer than {MAX_LINE_LEN} characters",
                number + 1
            ));
        }
        let indent = line.len() - line.trim_start().len();
        if indent > MAX_INDENT_SPACES {
            return bound(format!(
                "line {} nests past {MAX_INDENT_SPACES} spaces",
                number + 1
            ));
        }
        let content = line.trim_start();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        if number > 0 && (content == "---" || content == "...") {
            return bound("registry carries more than one document".to_string());
        }
        // Anchors, aliases, merge keys, and explicit tags are control input
        // the registry has no use for and an attacker can fan out with.
        if let Some(hazard) = control_hazard(content) {
            return bound(format!("line {}: {hazard}", number + 1));
        }
        // A list item opens a fresh mapping scope for its own keys.
        let (key_indent, key_part) = if let Some(rest) = content.strip_prefix("- ") {
            (indent + 2, rest)
        } else if content == "-" {
            continue;
        } else {
            (indent, content)
        };
        let Some((key, _)) = key_part.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || key.contains(' ') {
            continue;
        }
        while scopes.last().is_some_and(|(scope, _)| *scope > key_indent) {
            scopes.pop();
        }
        if scopes.last().is_none_or(|(scope, _)| *scope != key_indent) {
            scopes.push((key_indent, std::collections::BTreeSet::new()));
        }
        if content.starts_with("- ") {
            // Each list item is its own mapping; reset the keys at this scope.
            if let Some(entry) = scopes.last_mut() {
                entry.1.clear();
            }
        }
        if let Some(entry) = scopes.last_mut()
            && !entry.1.insert(key.to_string())
        {
            return Err(TrackingError::Bounds(format!(
                "line {}: duplicate mapping key '{key}'",
                number + 1
            )));
        }
    }
    Ok(())
}

/// The control token a line carries that the registry forbids, if any.
fn control_hazard(content: &str) -> Option<&'static str> {
    // Work on the value side of `key:` so a URL fragment is not a false hit.
    let value = content.split_once(": ").map_or(content, |(_, v)| v);
    let value = value.trim();
    if value.starts_with('&') {
        return Some("a YAML anchor is not allowed");
    }
    if value.starts_with('*') {
        return Some("a YAML alias is not allowed");
    }
    if value.starts_with('!') {
        return Some("a YAML tag is not allowed");
    }
    if content.trim_start().starts_with("<<") {
        return Some("a YAML merge key is not allowed");
    }
    None
}

/// Validate the format rules the schema states but a validator must confirm
/// for a `source` object: the transport, the reference shape, and the
/// object-ID length.
fn check_source(entry: &Entry) -> Result<(), TrackingError> {
    let Some(source) = &entry.source else {
        return Ok(());
    };
    let bad = |m: String| {
        Err(TrackingError::Semantic(format!(
            "entry '{}': {m}",
            entry.id
        )))
    };
    if source.kind != "git" {
        return bad(format!("source.kind must be 'git', not '{}'", source.kind));
    }
    if !source.repository.starts_with("https://") {
        return bad("source.repository must be a credential-free https:// URL".to_string());
    }
    if source.repository.contains('@') {
        return bad("source.repository must carry no credentials".to_string());
    }
    if !source.reference.starts_with("refs/") {
        return bad("source.reference must be a full refs/... reference".to_string());
    }
    if !is_object_id(&source.revision) {
        return bad(format!(
            "source.revision must be a full 40- or 64-character object ID, not '{}'",
            source.revision
        ));
    }
    if source.license.trim().is_empty() {
        return bad("source.license must be a non-empty identifier".to_string());
    }
    Ok(())
}

/// Parse and bound a registry, then confirm the version and the source
/// formats. Path existence and freshness are the service's checks.
///
/// # Errors
///
/// [`TrackingError::Bounds`] when a bound is exceeded, [`TrackingError::Shape`]
/// when the document does not fit the schema, and [`TrackingError::Semantic`]
/// for a version mismatch, a duplicate id or dependent, or a bad source field.
pub fn parse(text: &str) -> Result<Registry, TrackingError> {
    check_bounds(text)?;
    let registry: Registry =
        yaml_serde::from_str(text).map_err(|e| TrackingError::Shape(e.to_string()))?;
    if registry.schema_version != SCHEMA_VERSION {
        return Err(TrackingError::Semantic(format!(
            "schema_version must be {SCHEMA_VERSION}, not {}",
            registry.schema_version
        )));
    }
    if registry.tracked.len() > MAX_ENTRIES {
        return Err(TrackingError::Bounds(format!(
            "registry has more than {MAX_ENTRIES} entries"
        )));
    }
    let mut ids = std::collections::BTreeSet::new();
    for entry in &registry.tracked {
        if entry.cadence_days == 0 {
            return Err(TrackingError::Semantic(format!(
                "entry '{}': cadence_days must be a positive integer",
                entry.id
            )));
        }
        if !ids.insert(entry.id.clone()) {
            return Err(TrackingError::Semantic(format!(
                "duplicate entry id '{}'",
                entry.id
            )));
        }
        let mut dependents = std::collections::BTreeSet::new();
        for dependent in &entry.dependents {
            if !dependents.insert(dependent.clone()) {
                return Err(TrackingError::Semantic(format!(
                    "entry '{}': duplicate dependent '{dependent}'",
                    entry.id
                )));
            }
        }
        check_source(entry)?;
    }
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = "schema_version: 1\ntracked:\n  - id: sample\n    path: reference/x.md\n    last_checked: 2026-01-01\n    cadence_days: 30\n    why: it moves\n    revalidate:\n      - re-fetch it\n    dependents: []\n";

    #[test]
    fn accepts_a_minimal_registry() {
        let registry = parse(OK).unwrap();
        assert_eq!(registry.schema_version, 1);
        assert_eq!(registry.tracked.len(), 1);
    }

    #[test]
    fn accepts_a_git_source() {
        let text = format!(
            "schema_version: 1\ntracked:\n  - id: sample\n    path: reference/x.md\n    last_checked: 2026-01-01\n    cadence_days: 30\n    why: it moves\n    revalidate:\n      - re-fetch it\n    dependents: []\n    source:\n      kind: git\n      repository: https://github.com/o/r\n      reference: refs/tags/v1\n      revision: {}\n      license: MIT\n",
            "a".repeat(40)
        );
        let registry = parse(&text).unwrap();
        assert_eq!(registry.tracked[0].source.as_ref().unwrap().kind, "git");
    }

    #[test]
    fn rejects_a_wrong_schema_version() {
        let text = OK.replace("schema_version: 1", "schema_version: 2");
        assert!(matches!(parse(&text), Err(TrackingError::Semantic(_))));
    }

    #[test]
    fn rejects_a_duplicate_id() {
        let text = format!(
            "{OK}  - id: sample\n    path: reference/y.md\n    last_checked: 2026-01-01\n    cadence_days: 30\n    why: also\n    revalidate:\n      - go\n    dependents: []\n"
        );
        assert!(matches!(parse(&text), Err(TrackingError::Semantic(_))));
    }

    #[test]
    fn rejects_a_duplicate_mapping_key() {
        let text = "schema_version: 1\nschema_version: 1\ntracked: []\n";
        assert!(matches!(parse(text), Err(TrackingError::Bounds(_))));
    }

    #[test]
    fn rejects_an_alias_and_an_anchor() {
        let text = "schema_version: 1\ntracked: &all []\nother: *all\n";
        assert!(matches!(parse(text), Err(TrackingError::Bounds(_))));
    }

    #[test]
    fn rejects_a_second_document() {
        let text = "schema_version: 1\ntracked: []\n---\nschema_version: 1\ntracked: []\n";
        assert!(matches!(parse(text), Err(TrackingError::Bounds(_))));
    }

    #[test]
    fn rejects_a_tag() {
        let text = "schema_version: 1\ntracked: !!seq []\n";
        assert!(matches!(parse(text), Err(TrackingError::Bounds(_))));
    }

    #[test]
    fn rejects_a_branch_revision() {
        let text = "schema_version: 1\ntracked:\n  - id: s\n    path: r/x.md\n    last_checked: 2026-01-01\n    cadence_days: 30\n    why: w\n    revalidate:\n      - go\n    dependents: []\n    source:\n      kind: git\n      repository: https://github.com/o/r\n      reference: refs/heads/main\n      revision: main\n      license: MIT\n";
        assert!(matches!(parse(text), Err(TrackingError::Semantic(_))));
    }

    #[test]
    fn rejects_credentials_in_the_repository() {
        let text = "schema_version: 1\ntracked:\n  - id: s\n    path: r/x.md\n    last_checked: 2026-01-01\n    cadence_days: 30\n    why: w\n    revalidate:\n      - go\n    dependents: []\n    source:\n      kind: git\n      repository: https://user:pass@github.com/o/r\n      reference: refs/tags/v1\n      revision: 1111111111111111111111111111111111111111\n      license: MIT\n";
        assert!(matches!(parse(text), Err(TrackingError::Semantic(_))));
    }

    #[test]
    fn rejects_zero_cadence() {
        let text = OK.replace("cadence_days: 30", "cadence_days: 0");
        assert!(matches!(parse(&text), Err(TrackingError::Semantic(_))));
    }

    #[test]
    fn rejects_an_oversized_file() {
        let text = format!(
            "schema_version: 1\ntracked: []\n# {}\n",
            "x".repeat(MAX_LINE_LEN + 1)
        );
        assert!(matches!(parse(&text), Err(TrackingError::Bounds(_))));
    }
}
