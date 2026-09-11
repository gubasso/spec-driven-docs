//! What a project declares about the files its gates judge.
//!
//! One hand-edited file, `.spec-driven-docs/config.yaml`, states which paths
//! no delivered gate judges and which filters each named gate takes. The
//! managed pre-commit block is rendered from it, so the block stays wholly
//! owned by `sdd` and an upgrade never meets a project's edit.
//!
//! # The two keys are named apart
//!
//! `ESLint`'s flat config overloads one `ignores` key whose meaning changes
//! with what else sits beside it, and it is a documented, repeated source of
//! user confusion. `reserved` and `gates` mean one thing each.
//!
//! # The composition algorithm
//!
//! Four layers, in this order, and one function implements it:
//!
//! ```text
//! layer 1  registry   the row's include and exclude in GATES
//! layer 2  project    the gates: entry for that gate
//! layer 3  flag       --include and --exclude on the command line
//! layer 4  reserved   the reserved: list, as excludes only
//! ```
//!
//! Per field, a later layer extends rather than replaces, with one stated
//! exception: a project `include` list replaces the registry `include` list.
//! A registry include is a whitelist, so extending it can only widen what a
//! gate judges, which is the opposite of what a project asking for `include`
//! wants. Every `exclude` layer extends, and `reserved` is last, so nothing
//! reopens it. No layer can reopen an exclusion at all, because the grammar
//! carries no negation: `crate::domain::path_filter` refuses a leading `!`.

use std::collections::BTreeMap;

use camino::Utf8Path;
use serde::Deserialize;

use crate::domain::gate_id::GateId;
use crate::domain::path_filter::{Layer, PathFilter, PathFilterError, Pattern};

/// Where an instance keeps its declaration.
pub const CONFIG_PATH: &str = ".spec-driven-docs/config.yaml";

/// The filters one named gate takes.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GateFilters {
    /// Replaces the registry's include list for this gate.
    #[serde(default)]
    pub include: Vec<String>,
    /// Extends the registry's exclude list for this gate.
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// The declaration as written.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InstanceConfig {
    /// Paths no delivered gate judges. Another tool owns these.
    #[serde(default)]
    pub reserved: Vec<String>,
    /// Per-gate filters, keyed by gate id. A gate not named here takes its
    /// registry default.
    #[serde(default)]
    pub gates: BTreeMap<String, GateFilters>,
}

/// A declaration that does not parse, or that names something no gate has.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file is not YAML of this shape.
    #[error("{CONFIG_PATH} does not parse: {0}")]
    Shape(String),
    /// A `gates:` key is not a delivered gate id.
    #[error(
        "{CONFIG_PATH} names the gate `{0}`, which this version does not deliver: `sdd gate --list` names every one"
    )]
    UnknownGate(String),
    /// A pattern the filter grammar refuses.
    #[error("{CONFIG_PATH}: {0}")]
    Pattern(#[from] PathFilterError),
}

impl InstanceConfig {
    /// Parse a declaration.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Shape`] when the text is not a declaration, and
    /// [`ConfigError::UnknownGate`] when a `gates:` key names no delivered
    /// gate. Neither falls back to the default: a filter that quietly stops
    /// applying is worse than one that fails loudly.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let parsed: Self =
            yaml_serde::from_str(text).map_err(|error| ConfigError::Shape(error.to_string()))?;
        for key in parsed.gates.keys() {
            if resolve_id(key).is_none() {
                return Err(ConfigError::UnknownGate(key.clone()));
            }
        }
        Ok(parsed)
    }

    /// Read the declaration an instance carries.
    ///
    /// A missing file is the empty declaration and never an error: an
    /// instance that declares nothing is a valid instance.
    ///
    /// # Errors
    ///
    /// See [`Self::parse`]. An unreadable file that exists is a shape error
    /// rather than a silent default.
    pub fn read(repo_root: &Utf8Path) -> Result<Self, ConfigError> {
        let path = repo_root.join(CONFIG_PATH);
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(ConfigError::Shape(error.to_string())),
        }
    }

    /// This gate's declared filters, if the project named it.
    #[must_use]
    pub fn for_gate(&self, id: GateId) -> Option<&GateFilters> {
        self.gates.get(&id.to_string())
    }
}

/// Map a `gates:` key onto a delivered gate.
fn resolve_id(key: &str) -> Option<GateId> {
    GateId::ALL.iter().copied().find(|id| id.to_string() == key)
}

/// Build one gate's filter from every layer.
///
/// `registry_include` and `registry_exclude` arrive already templated
/// against the instance's documentation root, because the root is the
/// caller's to resolve.
///
/// # Errors
///
/// [`ConfigError::Pattern`] naming the pattern the grammar refuses.
pub fn resolve(
    registry_include: &[String],
    registry_exclude: &[String],
    declared: Option<&GateFilters>,
    flag_include: &[String],
    flag_exclude: &[String],
    reserved: &[String],
) -> Result<PathFilter, ConfigError> {
    // A project include replaces the registry include; extending a
    // whitelist could only widen what the gate judges.
    let includes: Vec<Pattern> = declared.filter(|d| !d.include.is_empty()).map_or_else(
        || {
            registry_include
                .iter()
                .map(|glob| Pattern::new(glob.clone(), Layer::Registry))
                .collect()
        },
        |declared| {
            declared
                .include
                .iter()
                .map(|glob| Pattern::new(glob.clone(), Layer::Project))
                .collect()
        },
    );
    let includes = includes
        .into_iter()
        .chain(
            flag_include
                .iter()
                .map(|glob| Pattern::new(glob.clone(), Layer::Flag)),
        )
        .collect();

    // Every exclude layer extends, and `reserved` is last so nothing that
    // follows can reopen it.
    let excludes: Vec<Pattern> = registry_exclude
        .iter()
        .map(|glob| Pattern::new(glob.clone(), Layer::Registry))
        .chain(
            declared
                .into_iter()
                .flat_map(|d| &d.exclude)
                .map(|glob| Pattern::new(glob.clone(), Layer::Project)),
        )
        .chain(
            flag_exclude
                .iter()
                .map(|glob| Pattern::new(glob.clone(), Layer::Flag)),
        )
        .chain(
            reserved
                .iter()
                .map(|glob| Pattern::new(glob.clone(), Layer::Reserved)),
        )
        .collect();

    Ok(PathFilter::build(includes, excludes)?)
}

/// Record reserved paths in a declaration, keeping every comment.
///
/// The file is hand-edited and its comments carry the whole explanation of
/// the four layers, so this rewrites one key textually rather than
/// round-tripping the YAML. A path already recorded is left alone, which is
/// what makes a repeated `--reserve` idempotent.
#[must_use]
pub fn with_reserved(text: &str, paths: &[String]) -> String {
    if paths.is_empty() {
        return text.to_string();
    }
    let existing = InstanceConfig::parse(text).unwrap_or_default().reserved;
    let mut added: Vec<&String> = paths
        .iter()
        .filter(|path| !existing.contains(path))
        .collect();
    added.dedup();
    if added.is_empty() {
        return text.to_string();
    }

    let entries: String = existing
        .iter()
        .map(|path| format!("  - {path}\n"))
        .chain(added.iter().map(|path| format!("  - {path}\n")))
        .collect();

    let mut out = String::new();
    let mut wrote = false;
    let mut skipping = false;
    for line in text.lines() {
        if skipping {
            // Drop the previous list items, which the rewritten key carries.
            if line.starts_with("  - ") || line.trim().is_empty() && !wrote {
                continue;
            }
            skipping = false;
        }
        if !wrote && (line.starts_with("reserved:")) {
            out.push_str("reserved:\n");
            out.push_str(&entries);
            wrote = true;
            skipping = true;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if !wrote {
        out.push_str("reserved:\n");
        out.push_str(&entries);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::path_filter::Decision;

    fn config(text: &str) -> InstanceConfig {
        InstanceConfig::parse(text).expect("the fixture parses")
    }

    #[test]
    fn an_absent_file_is_the_empty_declaration() {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
            .expect("the scratch path is UTF-8");
        let read = InstanceConfig::read(&root).expect("an absent file is not an error");
        assert_eq!(read, InstanceConfig::default());
    }

    #[test]
    fn an_empty_file_is_the_empty_declaration() {
        assert_eq!(config("{}\n"), InstanceConfig::default());
    }

    #[test]
    fn a_malformed_key_is_an_error_naming_the_key() {
        let error = InstanceConfig::parse("reserved: AGENTS.md\n")
            .expect_err("a scalar where a list belongs is refused");
        assert!(
            error.to_string().contains("reserved"),
            "the error does not name the key: {error}"
        );
    }

    #[test]
    fn an_unknown_key_is_refused() {
        assert!(InstanceConfig::parse("reservd:\n  - a.md\n").is_err());
    }

    #[test]
    fn an_unknown_gate_id_is_an_error_naming_the_key() {
        let error = InstanceConfig::parse("gates:\n  no-such-gate:\n    exclude: [a]\n")
            .expect_err("an unknown gate is refused");
        assert!(matches!(error, ConfigError::UnknownGate(ref key) if key == "no-such-gate"));
    }

    #[test]
    fn a_known_gate_id_parses() {
        let parsed = config("gates:\n  no-personal-path:\n    exclude:\n      - vendor/**\n");
        assert_eq!(
            parsed
                .for_gate(GateId::NoPersonalPath)
                .map(|f| f.exclude.clone()),
            Some(vec!["vendor/**".to_string()])
        );
    }

    #[test]
    fn a_path_leaving_the_repository_is_refused() {
        let error = resolve(&[], &[], None, &[], &[], &["../outside/**".to_string()])
            .expect_err("a pattern that climbs out is refused");
        assert!(matches!(error, ConfigError::Pattern(_)));
    }

    #[test]
    fn a_project_include_replaces_the_registry_include() {
        let filter = resolve(
            &["_docs/**/*.md".to_string()],
            &[],
            Some(&GateFilters {
                include: vec!["method/**/*.md".to_string()],
                exclude: Vec::new(),
            }),
            &[],
            &[],
            &[],
        )
        .expect("resolves");
        assert_eq!(
            filter.decide(Utf8Path::new("method/08-gates.md")),
            Decision::Read
        );
        assert_eq!(
            filter.decide(Utf8Path::new("_docs/specs/SPEC-a.md")),
            Decision::NotIncluded,
            "the registry include survived a project include that replaces it"
        );
    }

    #[test]
    fn every_exclude_layer_extends() {
        let filter = resolve(
            &[],
            &["a.md".to_string()],
            Some(&GateFilters {
                include: Vec::new(),
                exclude: vec!["b.md".to_string()],
            }),
            &[],
            &["c.md".to_string()],
            &["d.md".to_string()],
        )
        .expect("resolves");
        for path in ["a.md", "b.md", "c.md", "d.md"] {
            assert!(
                matches!(filter.decide(Utf8Path::new(path)), Decision::Skipped(_)),
                "{path} survived its exclude layer"
            );
        }
    }

    #[test]
    fn reserved_wins_over_a_gate_entry_that_includes_it() {
        let filter = resolve(
            &[],
            &[],
            Some(&GateFilters {
                include: vec!["AGENTS.md".to_string()],
                exclude: Vec::new(),
            }),
            &[],
            &[],
            &["AGENTS.md".to_string()],
        )
        .expect("resolves");
        match filter.decide(Utf8Path::new("AGENTS.md")) {
            Decision::Skipped(pattern) => assert_eq!(pattern.layer, Layer::Reserved),
            other => panic!("reserved did not win: {other:?}"),
        }
    }

    #[test]
    fn reserving_a_path_keeps_every_comment() {
        let seed = "# why this file exists\nreserved: []\n\n# per gate\ngates: {}\n";
        let out = with_reserved(seed, &["AGENTS.md".to_string()]);
        assert!(
            out.contains("# why this file exists"),
            "a comment was lost:\n{out}"
        );
        assert!(out.contains("# per gate"), "a comment was lost:\n{out}");
        assert!(out.contains("  - AGENTS.md"), "the path is missing:\n{out}");
        assert_eq!(
            InstanceConfig::parse(&out).expect("still parses").reserved,
            vec!["AGENTS.md".to_string()]
        );
    }

    #[test]
    fn reserving_a_recorded_path_changes_nothing() {
        let text = "reserved:\n  - AGENTS.md\ngates: {}\n";
        assert_eq!(with_reserved(text, &["AGENTS.md".to_string()]), text);
    }

    #[test]
    fn reserving_adds_beside_what_is_recorded() {
        let text = "reserved:\n  - AGENTS.md\ngates: {}\n";
        let out = with_reserved(text, &["vendor/**".to_string()]);
        assert_eq!(
            InstanceConfig::parse(&out).expect("parses").reserved,
            vec!["AGENTS.md".to_string(), "vendor/**".to_string()]
        );
    }

    #[test]
    fn no_layer_can_reopen_an_exclusion() {
        // The grammar carries no negation, so there is nothing to write that
        // would undo an earlier exclude. This asserts the refusal rather
        // than the absence.
        let error = resolve(&[], &[], None, &[], &["!a.md".to_string()], &[])
            .expect_err("a negation is refused");
        assert!(matches!(error, ConfigError::Pattern(_)));
    }
}
