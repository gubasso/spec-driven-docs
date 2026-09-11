//! The one matcher that decides whether a gate judges a path.
//!
//! # What a filter governs
//!
//! A *subject* path is a path whose content a gate judges, and which can
//! appear in a finding. A *support* path is a path a gate reads to know what
//! to judge: the canon manifest, the known-issue records, the docs-root
//! resolution. A filter governs subject paths only. A filter that reached
//! support paths would let a project disable a gate by excluding the file
//! that configures it, which is an off switch rather than a scope control.
//!
//! # Include is a whitelist
//!
//! When any include pattern exists, a path matching none of them is not
//! judged. Include is a whitelist, not an addition. An empty include list
//! judges everything the excludes leave.
//!
//! # Path form
//!
//! Every path is matched repository-relative, with forward slashes and no
//! leading `./`. [`PathFilter::decide`] normalizes the leading `./` that
//! [`crate::gates::walk_files`] produces, so a pattern author writes
//! `src/**` and never `./src/**`.
//!
//! # The grammar is restricted
//!
//! A pattern is a [`globset`] glob built with `literal_separator(true)`, so
//! `*` does not cross a `/` and `**` is the only way to descend. A pattern
//! that is absolute, holds a `..` component, opens with `!` or `#`, or
//! carries a backslash is refused. The first three are `gitignore` control
//! syntax this grammar does not carry, and accepting them as literals would
//! silently mean something other than what the author wrote. The backslash
//! escapes a metacharacter in `globset` on Unix, and the pre-commit
//! projection cannot reproduce that faithfully, so a pattern using it would
//! break the superset contract the renderer states. A nested brace
//! alternation is refused for the same reason.
//!
//! # Why not `ignore::overrides`
//!
//! `ignore::overrides::Override` answers the same shape of question and is
//! what `fd` builds `--exclude` on, but two of its properties defeat the
//! three-way answer this module returns. An unmatched ordinary file comes
//! back as `Match::Ignore(Glob::unmatched())` rather than `Match::None` once
//! any whitelist glob exists, so an include miss and an explicit exclude
//! arrive as the same variant. And `ignore::overrides::Glob` is opaque,
//! exposing no method that returns the pattern text that `--explain` must
//! print. This module therefore keeps its own pattern list. `Override` still
//! prunes the traversal in [`crate::gates::walk_files`], where no provenance
//! is wanted.

use camino::Utf8Path;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

/// Where a pattern came from. `--explain` prints it, so a reader can tell a
/// canon default from something their own project asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// The gate's row in [`crate::gates::GATES`].
    Registry,
    /// The `gates:` entry for this gate in the project's declaration.
    Project,
    /// A `--include` or `--exclude` flag.
    Flag,
    /// The `reserved:` list in the project's declaration.
    Reserved,
}

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Registry => "registry",
            Self::Project => "project",
            Self::Flag => "flag",
            Self::Reserved => "reserved",
        };
        f.write_str(name)
    }
}

/// One glob as its author wrote it, with the layer that contributed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    /// The glob text, unmodified.
    pub glob: String,
    /// Which layer contributed it.
    pub layer: Layer,
}

impl Pattern {
    /// A pattern from the named layer.
    #[must_use]
    pub fn new(glob: impl Into<String>, layer: Layer) -> Self {
        Self {
            glob: glob.into(),
            layer,
        }
    }
}

/// Why one path is judged or is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// No exclude matched, and either an include matched or the include
    /// list is empty.
    Read,
    /// An exclude matched. Carries the pattern that decided.
    Skipped(Pattern),
    /// Include patterns exist and none matched. Carries no pattern, because
    /// no pattern decided: the whitelist did.
    NotIncluded,
}

/// A pattern this grammar refuses, or one `globset` cannot compile.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathFilterError {
    /// The pattern opens with `gitignore` control syntax this grammar does
    /// not carry.
    #[error(
        "pattern `{glob}` opens with `{prefix}`, which this grammar does not carry: write an exclude pattern instead of a negation, and a comment outside the list"
    )]
    ControlPrefix {
        /// The refused pattern.
        glob: String,
        /// The character that refused it.
        prefix: char,
    },
    /// The pattern is absolute, so it names something outside the
    /// repository or depends on where the repository sits.
    #[error("pattern `{0}` is absolute: every pattern is repository-relative")]
    Absolute(String),
    /// The pattern climbs out of the repository.
    #[error("pattern `{0}` holds a `..` component: no pattern reaches outside the repository")]
    Escape(String),
    /// The pattern escapes a metacharacter, which the pre-commit projection
    /// cannot reproduce.
    #[error(
        "pattern `{0}` carries a backslash, which this grammar does not carry: a path holding a literal glob character cannot be named here"
    )]
    Escaped(String),
    /// The pattern nests a brace alternation, which the pre-commit
    /// projection cannot reproduce.
    #[error(
        "pattern `{0}` nests a brace alternation, which this grammar does not carry: write the branches flat"
    )]
    Nested(String),
    /// `globset` could not compile it.
    #[error("pattern `{glob}` is not a valid glob: {message}")]
    Malformed {
        /// The refused pattern.
        glob: String,
        /// What `globset` said.
        message: String,
    },
}

/// The subject filter for one gate.
#[derive(Debug)]
pub struct PathFilter {
    include: GlobSet,
    include_patterns: Vec<Pattern>,
    exclude: GlobSet,
    exclude_patterns: Vec<Pattern>,
}

impl PathFilter {
    /// A filter that judges everything.
    #[must_use]
    pub const fn permissive() -> Self {
        Self {
            include: GlobSet::empty(),
            include_patterns: Vec::new(),
            exclude: GlobSet::empty(),
            exclude_patterns: Vec::new(),
        }
    }

    /// Build from resolved include and exclude patterns.
    ///
    /// Both lists arrive in precedence order, earliest layer first, because
    /// [`Self::decide`] reports the last matching exclude.
    ///
    /// # Errors
    ///
    /// [`PathFilterError`] naming the pattern that is refused or that
    /// `globset` cannot compile.
    pub fn build(include: Vec<Pattern>, exclude: Vec<Pattern>) -> Result<Self, PathFilterError> {
        Ok(Self {
            include: compile(&include)?,
            include_patterns: include,
            exclude: compile(&exclude)?,
            exclude_patterns: exclude,
        })
    }

    /// Whether this filter states any include pattern.
    #[must_use]
    pub fn has_includes(&self) -> bool {
        !self.include_patterns.is_empty()
    }

    /// Every include pattern, in precedence order.
    #[must_use]
    pub fn includes(&self) -> &[Pattern] {
        &self.include_patterns
    }

    /// Every exclude pattern, in precedence order.
    #[must_use]
    pub fn excludes(&self) -> &[Pattern] {
        &self.exclude_patterns
    }

    /// Decide one repository-relative path.
    ///
    /// Excludes are tested first, and the last matching one decides, so a
    /// later layer overrides an earlier one. An exclude always beats an
    /// include: that is one rule, and a second would be a second thing to
    /// learn.
    #[must_use]
    pub fn decide(&self, path: &Utf8Path) -> Decision {
        let candidate = normalize(path);
        if let Some(index) = self.exclude.matches(&candidate).into_iter().max() {
            return Decision::Skipped(self.exclude_patterns[index].clone());
        }
        if self.include_patterns.is_empty() || self.include.is_match(&candidate) {
            Decision::Read
        } else {
            Decision::NotIncluded
        }
    }

    /// Whether this filter judges the path.
    #[must_use]
    pub fn judges(&self, path: &Utf8Path) -> bool {
        matches!(self.decide(path), Decision::Read)
    }

    /// Whether an exclude pattern drops the path, ignoring the includes.
    ///
    /// For a subject a gate finds itself, the discovery is the include: the
    /// gate already resolved which files it is about, and a registry
    /// whitelist written for the paths pre-commit passes would narrow that
    /// set a second time. An operator naming an extra record root would
    /// then get nothing. Exclusions still bind, so a reserved path is
    /// dropped either way.
    #[must_use]
    pub fn excluded(&self, path: &Utf8Path) -> bool {
        matches!(self.decide(path), Decision::Skipped(_))
    }
}

/// Strip the `./` prefix `walk_files` produces, so one path form reaches
/// every pattern.
fn normalize(path: &Utf8Path) -> String {
    let text = path.as_str();
    text.strip_prefix("./").unwrap_or(text).to_string()
}

/// Compile one list, refusing the grammar this module does not carry.
fn compile(patterns: &[Pattern]) -> Result<GlobSet, PathFilterError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(validate(&pattern.glob)?);
    }
    builder.build().map_err(|error| PathFilterError::Malformed {
        glob: String::new(),
        message: error.to_string(),
    })
}

/// Check one pattern against the restricted grammar and compile it.
fn validate(glob: &str) -> Result<globset::Glob, PathFilterError> {
    if let Some(prefix) = glob.chars().next().filter(|c| *c == '!' || *c == '#') {
        return Err(PathFilterError::ControlPrefix {
            glob: glob.to_string(),
            prefix,
        });
    }
    if glob.starts_with('/') || glob.chars().nth(1) == Some(':') {
        return Err(PathFilterError::Absolute(glob.to_string()));
    }
    if glob.split('/').any(|component| component == "..") {
        return Err(PathFilterError::Escape(glob.to_string()));
    }
    // `globset` reads a backslash as an escape on Unix, and the pre-commit
    // projection renders it as a literal followed by a wildcard. Accepting
    // it would let the matcher judge a path the rendered selection drops,
    // which is the one direction the superset contract forbids.
    if glob.contains('\\') {
        return Err(PathFilterError::Escaped(glob.to_string()));
    }
    // `globset` nests brace alternations and the pre-commit projection does
    // not, so a nested group would render as something matching nothing the
    // matcher judges. Refusing it keeps the superset contract true.
    let mut depth = 0_i32;
    for ch in glob.chars() {
        match ch {
            '{' => {
                depth += 1;
                if depth > 1 {
                    return Err(PathFilterError::Nested(glob.to_string()));
                }
            }
            '}' => depth -= 1,
            _ => {}
        }
    }
    GlobBuilder::new(glob)
        .literal_separator(true)
        .build()
        .map_err(|error| PathFilterError::Malformed {
            glob: glob.to_string(),
            message: error.kind().to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(include: &[&str], exclude: &[&str]) -> PathFilter {
        PathFilter::build(
            include
                .iter()
                .map(|g| Pattern::new(*g, Layer::Registry))
                .collect(),
            exclude
                .iter()
                .map(|g| Pattern::new(*g, Layer::Registry))
                .collect(),
        )
        .expect("the fixture patterns compile")
    }

    fn decide(f: &PathFilter, path: &str) -> Decision {
        f.decide(Utf8Path::new(path))
    }

    #[test]
    fn an_exclude_pattern_skips_the_path_and_names_it() {
        let f = PathFilter::build(Vec::new(), vec![Pattern::new("vendor/**", Layer::Reserved)])
            .expect("compiles");
        match decide(&f, "vendor/lib.rs") {
            Decision::Skipped(pattern) => {
                assert_eq!(pattern.glob, "vendor/**");
                assert_eq!(pattern.layer, Layer::Reserved);
            }
            other => panic!("expected Skipped, got {other:?}"),
        }
    }

    #[test]
    fn an_include_pattern_admits_only_what_it_matches() {
        let f = filter(&["_docs/**/*.md"], &[]);
        assert_eq!(decide(&f, "_docs/specs/SPEC-a.md"), Decision::Read);
        assert_eq!(decide(&f, "src/main.rs"), Decision::NotIncluded);
    }

    #[test]
    fn an_include_miss_is_not_reported_as_an_exclude() {
        // The exact confusion `ignore::overrides` would have introduced:
        // there it comes back as `Match::Ignore`, indistinguishable from a
        // pattern the author wrote.
        let f = filter(&["_docs/**"], &["vendor/**"]);
        assert_eq!(decide(&f, "src/main.rs"), Decision::NotIncluded);
        assert!(matches!(decide(&f, "vendor/x.rs"), Decision::Skipped(_)));
    }

    #[test]
    fn no_include_patterns_reads_everything_not_excluded() {
        let f = filter(&[], &["target/**"]);
        assert_eq!(decide(&f, "src/main.rs"), Decision::Read);
        assert_eq!(decide(&f, "anything/at/all.txt"), Decision::Read);
        assert!(matches!(decide(&f, "target/debug/x"), Decision::Skipped(_)));
    }

    #[test]
    fn many_includes_and_many_excludes_compose() {
        let f = filter(
            &["_docs/**/*.md", "method/**/*.md", "README.md"],
            &["_docs/scratch/**", "method/draft.md"],
        );
        assert_eq!(decide(&f, "README.md"), Decision::Read);
        assert_eq!(decide(&f, "method/08-gates.md"), Decision::Read);
        assert_eq!(decide(&f, "_docs/specs/SPEC-a.md"), Decision::Read);
        assert!(matches!(
            decide(&f, "_docs/scratch/note.md"),
            Decision::Skipped(_)
        ));
        assert!(matches!(
            decide(&f, "method/draft.md"),
            Decision::Skipped(_)
        ));
        assert_eq!(decide(&f, "src/main.rs"), Decision::NotIncluded);
    }

    #[test]
    fn an_exclude_beats_an_include_matching_the_same_path() {
        let f = filter(&["**/*.md"], &["AGENTS.md"]);
        assert!(matches!(decide(&f, "AGENTS.md"), Decision::Skipped(_)));
    }

    #[test]
    fn the_last_matching_exclude_layer_decides() {
        let f = PathFilter::build(
            Vec::new(),
            vec![
                Pattern::new("**/*.md", Layer::Registry),
                Pattern::new("AGENTS.md", Layer::Reserved),
            ],
        )
        .expect("compiles");
        match decide(&f, "AGENTS.md") {
            Decision::Skipped(pattern) => assert_eq!(pattern.layer, Layer::Reserved),
            other => panic!("expected Skipped, got {other:?}"),
        }
    }

    #[test]
    fn a_malformed_glob_is_a_usage_error() {
        let error = PathFilter::build(vec![Pattern::new("a[", Layer::Flag)], Vec::new())
            .expect_err("an unclosed class is refused");
        assert!(matches!(error, PathFilterError::Malformed { .. }));
    }

    #[test]
    fn a_control_prefix_is_refused() {
        for (glob, expected) in [
            ("!vendor/**", "ControlPrefix"),
            ("#a comment", "ControlPrefix"),
            ("/etc/passwd", "Absolute"),
            ("../outside/**", "Escape"),
            (r"docs/file\*.md", "Escaped"),
            ("{{a,b},{c,d}}.md", "Nested"),
        ] {
            let error = PathFilter::build(Vec::new(), vec![Pattern::new(glob, Layer::Project)])
                .expect_err("the grammar refuses it");
            let kind = match error {
                PathFilterError::ControlPrefix { .. } => "ControlPrefix",
                PathFilterError::Absolute(_) => "Absolute",
                PathFilterError::Escape(_) => "Escape",
                PathFilterError::Escaped(_) => "Escaped",
                PathFilterError::Nested(_) => "Nested",
                PathFilterError::Malformed { .. } => "Malformed",
            };
            assert_eq!(kind, expected, "for {glob}");
        }
    }

    #[test]
    fn a_single_star_does_not_cross_a_separator() {
        let f = filter(&["_docs/*.md"], &[]);
        assert_eq!(decide(&f, "_docs/a.md"), Decision::Read);
        assert_eq!(decide(&f, "_docs/specs/a.md"), Decision::NotIncluded);
    }

    #[test]
    fn a_double_star_descends() {
        let f = filter(&["_docs/**/*.md"], &[]);
        assert_eq!(decide(&f, "_docs/specs/deep/a.md"), Decision::Read);
    }

    #[test]
    fn a_basename_pattern_matches_at_any_depth() {
        let f = filter(&["**/AGENTS.md"], &[]);
        assert_eq!(decide(&f, "AGENTS.md"), Decision::Read);
        assert_eq!(decide(&f, "method/AGENTS.md"), Decision::Read);
    }

    #[test]
    fn a_root_anchored_pattern_matches_only_at_the_root() {
        let f = filter(&["README.md"], &[]);
        assert_eq!(decide(&f, "README.md"), Decision::Read);
        assert_eq!(decide(&f, "method/README.md"), Decision::NotIncluded);
    }

    #[test]
    fn a_brace_alternation_matches_each_branch() {
        let f = filter(&["_docs/**/{SPEC,ADR}-*.md"], &[]);
        assert_eq!(decide(&f, "_docs/specs/SPEC-a.md"), Decision::Read);
        assert_eq!(decide(&f, "_docs/decisions/ADR-a.md"), Decision::Read);
        assert_eq!(decide(&f, "_docs/other/KI-a.md"), Decision::NotIncluded);
    }

    #[test]
    fn the_walk_path_form_is_normalized() {
        let f = filter(&["src/**"], &[]);
        assert_eq!(decide(&f, "./src/main.rs"), Decision::Read);
    }

    #[test]
    fn a_permissive_filter_judges_everything() {
        let f = PathFilter::permissive();
        assert!(f.judges(Utf8Path::new("./anything")));
        assert!(!f.has_includes());
    }
}
