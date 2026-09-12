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
//!
//! # The writing style is the project's to select
//!
//! `writing_style` states where the writing convention comes from: this
//! convention's chapter, a document of the project's own, or none. The
//! managed documentation block routes authors to the selection, and `none`
//! installs no route and imposes no conversion obligation. The combination
//! is validated, not just the field: `project` without a `path` names
//! nothing, and a `path` beside another source is a value nothing reads,
//! which is a value that drifts.

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

/// Where a project's writing style comes from.
#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WritingSource {
    /// This convention's chapter, served by `sdd method writing-style`.
    #[default]
    Builtin,
    /// A document of the project's own, named by `path`.
    Project,
    /// No route and no conversion obligation.
    None,
}

/// The writing-style selection as written.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WritingStyle {
    /// Which source.
    #[serde(default)]
    pub source: WritingSource,
    /// The project's own document, relative to the repository root, where
    /// the source is `project`.
    #[serde(default)]
    pub path: Option<String>,
}

impl WritingStyle {
    /// Read the `--writing-style` argument: `builtin`, `none`, or
    /// `project:<path>`.
    ///
    /// # Errors
    ///
    /// [`ConfigError::WritingStyle`] for any other form, or a path the
    /// selection refuses.
    pub fn parse_flag(value: &str) -> Result<Self, ConfigError> {
        let selection = match value.trim() {
            "builtin" => Self::default(),
            "none" => Self {
                source: WritingSource::None,
                path: None,
            },
            other => match other.strip_prefix("project:") {
                Some(path) => Self {
                    source: WritingSource::Project,
                    path: Some(path.trim().to_string()),
                },
                None => {
                    return Err(ConfigError::WritingStyle(format!(
                        "`{other}` is not a selection; write `builtin`, `none`, or `project:<path>`"
                    )));
                }
            },
        };
        selection.check()?;
        Ok(selection)
    }

    /// Hold the combination, not just each field.
    fn check(&self) -> Result<(), ConfigError> {
        match (self.source, self.path.as_deref()) {
            (WritingSource::Project, None | Some("")) => Err(ConfigError::WritingStyle(
                "`source: project` names no `path`".to_string(),
            )),
            (WritingSource::Builtin | WritingSource::None, Some(path)) if !path.is_empty() => {
                Err(ConfigError::WritingStyle(format!(
                    "`path: {path}` is set and the source is not `project`, so nothing reads it"
                )))
            }
            (WritingSource::Project, Some(path)) => {
                let candidate = Utf8Path::new(path);
                if candidate.is_absolute() {
                    return Err(ConfigError::WritingStyle(format!(
                        "`path: {path}` is absolute; name the document relative to the repository"
                    )));
                }
                if candidate.components().any(|part| part.as_str() == "..") {
                    return Err(ConfigError::WritingStyle(format!(
                        "`path: {path}` leaves the repository"
                    )));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// The document authors are routed to, relative to the repository, or
    /// `None` where the selection installs no route.
    #[must_use]
    pub fn route(&self) -> Option<String> {
        match self.source {
            WritingSource::Builtin => Some("`sdd method writing-style`".to_string()),
            WritingSource::Project => self.path.as_ref().map(|path| format!("`{path}`")),
            WritingSource::None => None,
        }
    }

    /// The selection as the declaration file spells it.
    #[must_use]
    pub fn render(&self) -> String {
        let source = match self.source {
            WritingSource::Builtin => "builtin",
            WritingSource::Project => "project",
            WritingSource::None => "none",
        };
        let path = self
            .path
            .as_deref()
            .filter(|path| !path.is_empty())
            .map_or_else(|| "null".to_string(), quoted);
        format!("writing_style:\n  source: {source}\n  path: {path}\n")
    }
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
    /// Where the writing style comes from. Absent, the convention's own
    /// chapter.
    #[serde(default)]
    pub writing_style: WritingStyle,
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
    /// A writing-style selection whose fields do not agree.
    #[error("{CONFIG_PATH}: writing_style: {0}")]
    WritingStyle(String),
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
        // Compile every pattern here, at the one boundary, rather than
        // where a gate runs. A malformed pattern that reached `render_block`
        // would be written into the managed block and blessed by `verify`,
        // then fail separately from every gate that ran.
        parsed.check_patterns()?;
        parsed.writing_style.check()?;
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

    /// Compile every declared pattern, so a bad one fails once and here.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Pattern`] naming the pattern the grammar refuses.
    fn check_patterns(&self) -> Result<(), ConfigError> {
        let every = self.reserved.iter().chain(
            self.gates
                .values()
                .flat_map(|filters| filters.include.iter().chain(&filters.exclude)),
        );
        for glob in every {
            PathFilter::build(Vec::new(), vec![Pattern::new(glob.clone(), Layer::Project)])?;
        }
        Ok(())
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

/// One glob as a YAML scalar that reads back as itself.
///
/// A glob and YAML disagree about several leading characters: `*` opens an
/// alias, `[` and `{` open a flow collection, and `#` opens a comment. The
/// filter grammar accepts `**/generated.md`, so writing it bare would
/// produce a file the next read refuses. Single quotes make every glob a
/// scalar, with an apostrophe doubled.
fn quoted(glob: &str) -> String {
    format!("'{}'", glob.replace('\'', "''"))
}

/// Record a writing-style selection in a declaration, keeping every comment.
///
/// The `writing_style:` key and its indented children are replaced as a
/// block; a declaration written before the key existed gets it appended.
#[must_use]
pub fn with_writing_style(text: &str, selection: &WritingStyle) -> String {
    let block = selection.render();
    let mut out = String::new();
    let mut wrote = false;
    let mut skipping = false;
    for line in text.lines() {
        if skipping {
            if line.starts_with(' ') || line.starts_with('\t') {
                continue;
            }
            skipping = false;
        }
        if !wrote && line.starts_with("writing_style:") {
            out.push_str(&block);
            wrote = true;
            skipping = true;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if !wrote {
        if !out.is_empty() && !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str("# Where the writing style comes from: `builtin` routes authors to\n");
        out.push_str("# `sdd method writing-style`, `project` routes them to the document\n");
        out.push_str("# `path` names, and `none` installs no route and imposes no conversion\n");
        out.push_str("# obligation.\n");
        out.push_str(&block);
    }
    out
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
        .map(|path| format!("  - {}\n", quoted(path)))
        .chain(added.iter().map(|path| format!("  - {}\n", quoted(path))))
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
        assert!(
            out.contains("  - 'AGENTS.md'"),
            "the path is missing:\n{out}"
        );
        assert_eq!(
            InstanceConfig::parse(&out).expect("still parses").reserved,
            vec!["AGENTS.md".to_string()]
        );
    }

    #[test]
    fn reserving_a_recorded_path_changes_nothing() {
        let text = "reserved:\n  - 'AGENTS.md'\ngates: {}\n";
        assert_eq!(with_reserved(text, &["AGENTS.md".to_string()]), text);
    }

    #[test]
    fn a_glob_is_written_as_a_yaml_scalar_that_reads_back() {
        // `*`, `[`, `{`, and `#` all open something in YAML, and the filter
        // grammar accepts globs starting with the first three.
        for glob in ["**/generated.md", "[ab]/x.md", "{a,b}/x.md", "it's/x.md"] {
            let out = with_reserved("reserved: []\ngates: {}\n", &[glob.to_string()]);
            assert_eq!(
                InstanceConfig::parse(&out)
                    .unwrap_or_else(|e| panic!("{glob} did not read back: {e}"))
                    .reserved,
                vec![glob.to_string()],
                "for {glob}"
            );
        }
    }

    #[test]
    fn a_refused_pattern_fails_at_the_declaration_boundary() {
        // Not where a gate runs, and not after the managed block carries it.
        let error = InstanceConfig::parse("reserved:\n  - '!negated'\ngates: {}\n")
            .expect_err("a negation is refused at parse");
        assert!(matches!(error, ConfigError::Pattern(_)));
        assert!(
            InstanceConfig::parse("gates:\n  no-personal-path:\n    exclude: ['a[']\n").is_err()
        );
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
    fn an_absent_key_is_builtin() {
        assert_eq!(
            config("reserved: []\ngates: {}\n").writing_style,
            WritingStyle::default()
        );
        assert_eq!(
            config("writing_style:\n  source: builtin\n  path: null\n")
                .writing_style
                .route(),
            Some("`sdd method writing-style`".to_string())
        );
    }

    #[test]
    fn project_without_a_path_is_an_error_naming_the_key() {
        let error = InstanceConfig::parse("writing_style:\n  source: project\n")
            .expect_err("a project source needs a path");
        assert!(matches!(error, ConfigError::WritingStyle(_)));
        assert!(error.to_string().contains("writing_style"), "{error}");
        assert!(error.to_string().contains("path"), "{error}");
    }

    #[test]
    fn a_path_without_the_project_source_is_an_error() {
        for source in ["builtin", "none"] {
            let error = InstanceConfig::parse(&format!(
                "writing_style:\n  source: {source}\n  path: docs/style.md\n"
            ))
            .expect_err("a path nothing reads is refused");
            assert!(error.to_string().contains("nothing reads it"), "{error}");
        }
    }

    #[test]
    fn a_writing_style_path_leaving_the_repository_is_refused() {
        for path in ["/etc/style.md", "../style.md", "docs/../../style.md"] {
            assert!(
                WritingStyle::parse_flag(&format!("project:{path}")).is_err(),
                "{path} was accepted"
            );
        }
        assert_eq!(
            WritingStyle::parse_flag("project:docs/STYLE.md")
                .unwrap()
                .route(),
            Some("`docs/STYLE.md`".to_string())
        );
        assert_eq!(WritingStyle::parse_flag("none").unwrap().route(), None);
        assert!(WritingStyle::parse_flag("house").is_err());
    }

    #[test]
    fn a_selection_is_written_into_the_declaration_and_reads_back() {
        let seed = "reserved: []\n\n# how to write\nwriting_style:\n  source: builtin\n  path: null\n\ngates: {}\n";
        let selection = WritingStyle::parse_flag("project:docs/STYLE.md").unwrap();
        let out = with_writing_style(seed, &selection);
        assert!(out.contains("# how to write"), "a comment was lost:\n{out}");
        assert!(out.contains("gates: {}"), "a later key was lost:\n{out}");
        assert_eq!(config(&out).writing_style, selection);

        // A declaration written before the key existed gets it appended.
        let older = "reserved: []\ngates: {}\n";
        let out = with_writing_style(older, &WritingStyle::parse_flag("none").unwrap());
        assert_eq!(config(&out).writing_style.source, WritingSource::None);
        assert!(config(&out).reserved.is_empty());
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
