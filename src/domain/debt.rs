//! Budget debt: the inherited violations a project carries, and only downward.
//!
//! A budget gate measures one document on one or more dimensions and fails
//! the measurement that exceeds its budget. A project adopting the convention
//! with a corpus written before it would fail its first commit on files
//! nobody touched, so it records each inherited violation here, per gate,
//! per path, per dimension. A numeric dimension carries a ceiling the gate
//! judges against instead of the budget. A boolean dimension is an exception
//! that clears once the condition is corrected.
//!
//! The file can only shrink. Nothing delivers it, nothing raises a ceiling,
//! and a recorded dimension that stops matching what the gate measures is a
//! failure naming the command that lowers it. The tightening is pure: it
//! takes the parsed debt and a set of measurements and returns the new debt,
//! so the gates and the command share one implementation and a test drives
//! it with no filesystem. What a gate measures is each gate's business.
//!
//! SATISFIES budget-debt:a-recorded-dimension-only-shrinks

use std::collections::BTreeMap;
use std::fmt::Write as _;

use camino::Utf8Path;

use crate::domain::gate_id::GateId;

/// Where an instance keeps its debt.
pub const DEBT_PATH: &str = ".spec-driven-docs/debt.yaml";
/// The flat list of exempt chapters an older instance carries.
pub const LEGACY_DEBT_PATH: &str = ".spec-driven-docs/chapter-size-debt.txt";
/// The debt file schema this binary reads and writes.
pub const SCHEMA_VERSION: u64 = 1;

/// The gates that measure a budget, in id order.
pub const BUDGET_GATES: &[GateId] = &[
    GateId::AdrWordCap,
    GateId::AgentsDigestSize,
    GateId::ChapterSizeCap,
    GateId::SpecSizeCap,
];

/// What a dimension measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A count judged against a budget, and against a ceiling once recorded.
    Count,
    /// A condition that either holds or does not.
    Flag,
}

/// One dimension a budget gate measures: its key in the file, its kind, and
/// the noun a finding prints after the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DimensionSpec {
    /// The key under a path entry.
    pub name: &'static str,
    /// Count or flag.
    pub kind: Kind,
    /// The noun a finding prints after a count.
    pub label: &'static str,
}

const fn dim(name: &'static str, kind: Kind, label: &'static str) -> DimensionSpec {
    DimensionSpec { name, kind, label }
}

const WORDS: &[DimensionSpec] = &[dim("words", Kind::Count, "words")];
const LINES: &[DimensionSpec] = &[dim("lines", Kind::Count, "lines")];
const SPEC: &[DimensionSpec] = &[
    dim("authored_lines", Kind::Count, "authored lines"),
    dim("missing_toc", Kind::Flag, "missing table of contents"),
];

/// The dimensions one budget gate measures, or none for a gate that measures
/// no budget.
#[must_use]
pub const fn dimensions(gate: GateId) -> &'static [DimensionSpec] {
    match gate {
        GateId::AdrWordCap => WORDS,
        GateId::AgentsDigestSize | GateId::ChapterSizeCap => LINES,
        GateId::SpecSizeCap => SPEC,
        _ => &[],
    }
}

fn dimension_spec(gate: GateId, name: &str) -> Option<&'static DimensionSpec> {
    dimensions(gate).iter().find(|spec| spec.name == name)
}

/// One recorded dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recorded {
    /// The gate fails above this, and at or below the budget the entry is
    /// stale.
    Ceiling(usize),
    /// The condition is accepted until it is corrected.
    Exception,
}

/// What a gate measured on one dimension of one path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Measured {
    /// A count, with the budget the gate would judge it against.
    Count {
        /// What the gate counted.
        value: usize,
        /// The budget the specification states.
        budget: usize,
    },
    /// Whether the condition holds.
    Flag(bool),
}

/// One measurement: a gate, a path, a dimension, and what was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measurement {
    /// The gate that measured.
    pub gate: GateId,
    /// The path, in the form the gate reports it.
    pub path: String,
    /// The dimension's key.
    pub dimension: &'static str,
    /// What was found.
    pub value: Measured,
}

impl Measurement {
    /// A count measurement.
    #[must_use]
    pub fn count(
        gate: GateId,
        path: impl Into<String>,
        dimension: &'static str,
        value: usize,
        budget: usize,
    ) -> Self {
        Self {
            gate,
            path: path.into(),
            dimension,
            value: Measured::Count { value, budget },
        }
    }

    /// A flag measurement.
    #[must_use]
    pub fn flag(
        gate: GateId,
        path: impl Into<String>,
        dimension: &'static str,
        holds: bool,
    ) -> Self {
        Self {
            gate,
            path: path.into(),
            dimension,
            value: Measured::Flag(holds),
        }
    }

    /// Whether the measurement violates the budget on its own.
    #[must_use]
    pub const fn violates(&self) -> bool {
        match self.value {
            Measured::Count { value, budget } => value > budget,
            Measured::Flag(holds) => holds,
        }
    }
}

/// The entries of the legacy chapter list, normalized, in file order.
///
/// One parser for the gate and the migration, so the migration can never
/// convert a line the gate did not honour. The gate never trimmed a line,
/// so a line with surrounding whitespace named no file and exempted
/// nothing; it is kept as it was written and matches nothing here either.
#[must_use]
pub fn legacy_list(text: &str) -> Vec<String> {
    text.lines()
        .filter(|entry| !entry.is_empty() && !entry.starts_with('#'))
        .map(normalize)
        .collect()
}

/// The form a path takes in the file: repository-relative, no `./`.
#[must_use]
pub fn normalize(path: &str) -> String {
    path.trim_start_matches("./").to_string()
}

/// A debt file that cannot be trusted, or a verb the state refuses.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DebtError {
    /// The file is not YAML of this shape.
    #[error("{DEBT_PATH} does not parse: {0}")]
    Shape(String),
    /// One entry is not of this shape; the gate and path locate it.
    #[error("{DEBT_PATH}: {gate}: {path}: {detail}")]
    Malformed {
        /// The gate key the entry sits under.
        gate: String,
        /// The path key the entry sits under, or `-` for the gate itself.
        path: String,
        /// What is wrong.
        detail: String,
    },
    /// Both the legacy list and the dimensional file are present.
    #[error(
        "both {LEGACY_DEBT_PATH} and {DEBT_PATH} are present; run 'sdd debt migrate --apply' to finish the migration"
    )]
    TwoFormats,
    /// A baseline was requested over an existing debt file.
    #[error(
        "{DEBT_PATH} already exists and a baseline never widens it; fix the violation, or run 'sdd debt tighten --apply' where a recorded ceiling has slack"
    )]
    AlreadyBaselined,
    /// A baseline was requested while the legacy list is still in place.
    #[error("{LEGACY_DEBT_PATH} is present; run 'sdd debt migrate --apply' before a baseline")]
    LegacyBlocksBaseline,
    /// A migration was requested with no legacy list to migrate.
    #[error("{LEGACY_DEBT_PATH} is absent; there is no legacy list to migrate")]
    NothingToMigrate,
}

/// Which files are on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presence {
    /// `.spec-driven-docs/debt.yaml` exists.
    pub dimensional: bool,
    /// `.spec-driven-docs/chapter-size-debt.txt` exists.
    pub legacy: bool,
}

impl Presence {
    /// What an instance root carries.
    #[must_use]
    pub fn at(root: &Utf8Path) -> Self {
        Self {
            dimensional: root.join(DEBT_PATH).is_file(),
            legacy: root.join(LEGACY_DEBT_PATH).is_file(),
        }
    }
}

type Entries = BTreeMap<GateId, BTreeMap<String, BTreeMap<&'static str, Recorded>>>;

/// The recorded debt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Debt {
    entries: Entries,
}

/// One change a tightening makes, or declines to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A ceiling came down to the measurement.
    Lowered {
        /// The gate.
        gate: GateId,
        /// The path.
        path: String,
        /// The dimension.
        dimension: &'static str,
        /// The recorded ceiling.
        from: usize,
        /// The measurement it lowers to.
        to: usize,
    },
    /// A dimension left the file: within budget, corrected, or unmeasured.
    Removed {
        /// The gate.
        gate: GateId,
        /// The path.
        path: String,
        /// The dimension.
        dimension: &'static str,
        /// Why it left.
        reason: String,
    },
    /// A measurement rose above its ceiling; the entry stands and the gate
    /// fails on it.
    Grew {
        /// The gate.
        gate: GateId,
        /// The path.
        path: String,
        /// The dimension.
        dimension: &'static str,
        /// The recorded ceiling.
        ceiling: usize,
        /// The measurement above it.
        measured: usize,
    },
}

/// What a tightening produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tightened {
    /// The debt after tightening.
    pub debt: Debt,
    /// Every change, in gate, path, dimension order.
    pub changes: Vec<Change>,
}

impl Debt {
    /// Whether nothing is recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.values().all(BTreeMap::is_empty)
    }

    /// Record one dimension.
    pub fn record(&mut self, gate: GateId, path: &str, dimension: &'static str, value: Recorded) {
        self.entries
            .entry(gate)
            .or_default()
            .entry(normalize(path))
            .or_default()
            .insert(dimension, value);
    }

    /// What is recorded for one dimension of one path.
    #[must_use]
    pub fn recorded(&self, gate: GateId, path: &str, dimension: &str) -> Option<Recorded> {
        self.entries
            .get(&gate)?
            .get(&normalize(path))?
            .get(dimension)
            .copied()
    }

    /// Every recorded dimension of one gate, as `(path, dimension, value)`.
    #[must_use]
    pub fn recorded_for(&self, gate: GateId) -> Vec<(String, &'static str, Recorded)> {
        self.entries
            .get(&gate)
            .into_iter()
            .flat_map(|paths| {
                paths.iter().flat_map(|(path, dims)| {
                    dims.iter()
                        .map(|(dimension, value)| (path.clone(), *dimension, *value))
                })
            })
            .collect()
    }

    /// The debt a set of measurements becomes: every violating dimension,
    /// at the measurement.
    #[must_use]
    pub fn baseline(measurements: &[Measurement]) -> Self {
        let mut debt = Self::default();
        for measurement in measurements {
            match measurement.value {
                Measured::Count { value, budget } if value > budget => {
                    debt.record(
                        measurement.gate,
                        &measurement.path,
                        measurement.dimension,
                        Recorded::Ceiling(value),
                    );
                }
                Measured::Flag(true) => {
                    debt.record(
                        measurement.gate,
                        &measurement.path,
                        measurement.dimension,
                        Recorded::Exception,
                    );
                }
                Measured::Count { .. } | Measured::Flag(false) => {}
            }
        }
        debt
    }

    /// The debt after every ceiling comes down to its measurement.
    ///
    /// A ceiling never rises and a cleared exception is never reinstated.
    /// A dimension within its budget, a corrected condition, and a path the
    /// gate no longer measures all leave the file. A measurement above its
    /// ceiling is reported and left alone, because the gate already fails on
    /// it and lowering is not the fix.
    #[must_use]
    pub fn tighten(&self, measurements: &[Measurement]) -> Tightened {
        let found: BTreeMap<(GateId, String, &str), Measured> = measurements
            .iter()
            .map(|m| ((m.gate, normalize(&m.path), m.dimension), m.value))
            .collect();
        let mut debt = Self::default();
        let mut changes = Vec::new();
        for (gate, paths) in &self.entries {
            for (path, dims) in paths {
                for (dimension, recorded) in dims {
                    let key = (*gate, path.clone(), *dimension);
                    let removed = |reason: &str| Change::Removed {
                        gate: *gate,
                        path: path.clone(),
                        dimension,
                        reason: reason.to_string(),
                    };
                    match (recorded, found.get(&key)) {
                        (_, None) => {
                            changes.push(removed("the gate measures no such path"));
                        }
                        (Recorded::Ceiling(_), Some(Measured::Count { value, budget }))
                            if value <= budget =>
                        {
                            changes.push(removed("within the budget"));
                        }
                        (Recorded::Ceiling(ceiling), Some(Measured::Count { value, .. })) => {
                            if value < ceiling {
                                changes.push(Change::Lowered {
                                    gate: *gate,
                                    path: path.clone(),
                                    dimension,
                                    from: *ceiling,
                                    to: *value,
                                });
                                debt.record(*gate, path, dimension, Recorded::Ceiling(*value));
                            } else {
                                if value > ceiling {
                                    changes.push(Change::Grew {
                                        gate: *gate,
                                        path: path.clone(),
                                        dimension,
                                        ceiling: *ceiling,
                                        measured: *value,
                                    });
                                }
                                debt.record(*gate, path, dimension, *recorded);
                            }
                        }
                        (Recorded::Exception, Some(Measured::Flag(false))) => {
                            changes.push(removed("corrected"));
                        }
                        (Recorded::Exception, Some(Measured::Flag(true))) => {
                            debt.record(*gate, path, dimension, *recorded);
                        }
                        (Recorded::Ceiling(_), Some(Measured::Flag(_)))
                        | (Recorded::Exception, Some(Measured::Count { .. })) => {
                            changes.push(removed("the dimension's kind changed"));
                        }
                    }
                }
            }
        }
        Tightened { debt, changes }
    }

    /// Parse a debt file.
    ///
    /// # Errors
    ///
    /// [`DebtError::Shape`] when the text is not a mapping of this schema,
    /// and [`DebtError::Malformed`] naming the gate and path of the first
    /// entry that is not of the expected shape. Neither falls back to the
    /// empty debt: a file that quietly stops applying turns a green commit
    /// into a false pass.
    pub fn parse(text: &str) -> Result<Self, DebtError> {
        let value: yaml_serde::Value =
            yaml_serde::from_str(text).map_err(|error| DebtError::Shape(error.to_string()))?;
        let Some(top) = value.as_mapping() else {
            return Err(DebtError::Shape(
                "the document is not a mapping".to_string(),
            ));
        };
        let mut debt = Self::default();
        let mut schema = None;
        for (key, value) in top {
            let Some(key) = key.as_str() else {
                return Err(DebtError::Shape(format!("a key is not a string: {key:?}")));
            };
            if key == "schema_version" {
                schema = value.as_u64();
                if schema != Some(SCHEMA_VERSION) {
                    return Err(DebtError::Shape(format!(
                        "schema_version must be {SCHEMA_VERSION}, found {value:?}"
                    )));
                }
                continue;
            }
            let Some(gate) = BUDGET_GATES.iter().copied().find(|g| g.to_string() == key) else {
                return Err(DebtError::Malformed {
                    gate: key.to_string(),
                    path: "-".to_string(),
                    detail: "not a budget gate; the budget gates are adr-word-cap, agents-digest-size, chapter-size-cap, and spec-size-cap".to_string(),
                });
            };
            parse_gate(&mut debt, gate, key, value)?;
        }
        if schema.is_none() {
            return Err(DebtError::Shape("schema_version is missing".to_string()));
        }
        Ok(debt)
    }

    /// Read the debt an instance carries.
    ///
    /// An absent file is the empty debt and never an error: nothing delivers
    /// the file, so absence is the ordinary state.
    ///
    /// # Errors
    ///
    /// [`DebtError::TwoFormats`] when the legacy list sits beside the file,
    /// and the parse errors of [`Self::parse`].
    pub fn read(root: &Utf8Path) -> Result<Self, DebtError> {
        let presence = Presence::at(root);
        if presence.dimensional && presence.legacy {
            return Err(DebtError::TwoFormats);
        }
        if !presence.dimensional {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(root.join(DEBT_PATH))
            .map_err(|error| DebtError::Shape(error.to_string()))?;
        Self::parse(&text)
    }

    /// The file's text, in the one shape this binary writes.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("# Inherited budget violations this project carries.\n");
        out.push_str("#\n");
        out.push_str("# A ceiling is judged instead of the budget and only comes down: run\n");
        out.push_str(
            "# `sdd debt tighten --apply` after a document shrinks. An exception clears\n",
        );
        out.push_str("# once the condition is corrected. Nothing here can be widened.\n");
        let _ = writeln!(out, "schema_version: {SCHEMA_VERSION}");
        for (gate, paths) in &self.entries {
            if paths.is_empty() {
                continue;
            }
            let _ = writeln!(out, "\n{gate}:");
            for (path, dims) in paths {
                let _ = writeln!(out, "  {}:", quoted(path));
                for (dimension, recorded) in dims {
                    match recorded {
                        Recorded::Ceiling(ceiling) => {
                            let _ = writeln!(out, "    {dimension}:\n      ceiling: {ceiling}");
                        }
                        Recorded::Exception => {
                            let _ = writeln!(out, "    {dimension}: true");
                        }
                    }
                }
            }
        }
        out
    }
}

/// A path as a YAML scalar that reads back as itself.
fn quoted(path: &str) -> String {
    format!("'{}'", path.replace('\'', "''"))
}

fn parse_gate(
    debt: &mut Debt,
    gate: GateId,
    key: &str,
    value: &yaml_serde::Value,
) -> Result<(), DebtError> {
    let malformed = |path: &str, detail: String| DebtError::Malformed {
        gate: key.to_string(),
        path: path.to_string(),
        detail,
    };
    if value.is_null() {
        return Ok(());
    }
    let Some(paths) = value.as_mapping() else {
        return Err(malformed("-", "not a mapping of paths".to_string()));
    };
    for (path, dims) in paths {
        let Some(path) = path.as_str() else {
            return Err(malformed(
                "-",
                format!("a path key is not a string: {path:?}"),
            ));
        };
        let Some(dims) = dims.as_mapping() else {
            return Err(malformed(path, "not a mapping of dimensions".to_string()));
        };
        for (name, recorded) in dims {
            let Some(name) = name.as_str() else {
                return Err(malformed(
                    path,
                    format!("a dimension key is not a string: {name:?}"),
                ));
            };
            let Some(spec) = dimension_spec(gate, name) else {
                let known: Vec<&str> = dimensions(gate).iter().map(|d| d.name).collect();
                return Err(malformed(
                    path,
                    format!(
                        "`{name}` is not a dimension of {gate}; it measures {}",
                        known.join(", ")
                    ),
                ));
            };
            let value = match spec.kind {
                Kind::Count => recorded
                    .as_mapping()
                    .and_then(|m| m.get("ceiling"))
                    .and_then(yaml_serde::Value::as_u64)
                    .and_then(|n| usize::try_from(n).ok())
                    .map(Recorded::Ceiling)
                    .ok_or_else(|| {
                        malformed(path, format!("`{name}` must carry `ceiling: <count>`"))
                    })?,
                Kind::Flag => match recorded.as_bool() {
                    Some(true) => Recorded::Exception,
                    _ => {
                        return Err(malformed(
                            path,
                            format!(
                                "`{name}` must be `true`; a corrected exception is removed rather than set false"
                            ),
                        ));
                    }
                },
            };
            debt.record(gate, path, spec.name, value);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "schema_version: 1\n\nspec-size-cap:\n  _docs/specs/SPEC-legacy.md:\n    authored_lines:\n      ceiling: 417\n    missing_toc: true\n\nchapter-size-cap:\n  method/legacy.md:\n    lines:\n      ceiling: 417\n";

    fn sample() -> Debt {
        Debt::parse(SAMPLE).expect("the sample parses")
    }

    #[test]
    fn the_sample_parses_and_renders_back_to_itself() {
        let debt = sample();
        assert_eq!(
            debt.recorded(
                GateId::SpecSizeCap,
                "./_docs/specs/SPEC-legacy.md",
                "authored_lines"
            ),
            Some(Recorded::Ceiling(417))
        );
        assert_eq!(
            debt.recorded(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "missing_toc"
            ),
            Some(Recorded::Exception)
        );
        assert_eq!(Debt::parse(&debt.render()).unwrap(), debt);
    }

    #[test]
    fn an_absent_file_is_the_empty_debt() {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let debt = Debt::read(&root).expect("absence is not an error");
        assert!(debt.is_empty());
    }

    #[test]
    fn both_formats_present_is_an_error_naming_migrate() {
        let dir = tempfile::tempdir().unwrap();
        let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(root.join(".spec-driven-docs")).unwrap();
        std::fs::write(root.join(DEBT_PATH), SAMPLE).unwrap();
        std::fs::write(root.join(LEGACY_DEBT_PATH), "method/legacy.md\n").unwrap();
        let error = Debt::read(&root).unwrap_err();
        assert_eq!(error, DebtError::TwoFormats);
        assert!(error.to_string().contains("sdd debt migrate --apply"));
    }

    #[test]
    fn a_malformed_file_is_an_error_naming_the_gate_and_path() {
        let error = Debt::parse(
            "schema_version: 1\nchapter-size-cap:\n  method/a.md:\n    words:\n      ceiling: 3\n",
        )
        .unwrap_err();
        assert!(
            matches!(&error, DebtError::Malformed { gate, path, .. } if gate == "chapter-size-cap" && path == "method/a.md"),
            "{error}"
        );
        let error = Debt::parse("schema_version: 1\nno-personal-path:\n  a.md: {}\n").unwrap_err();
        assert!(matches!(&error, DebtError::Malformed { gate, .. } if gate == "no-personal-path"));
        let error =
            Debt::parse("schema_version: 1\nspec-size-cap:\n  a.md:\n    missing_toc: false\n")
                .unwrap_err();
        assert!(error.to_string().contains("must be `true`"), "{error}");
        assert!(matches!(
            Debt::parse("chapter-size-cap: {}\n").unwrap_err(),
            DebtError::Shape(_)
        ));
        assert!(matches!(
            Debt::parse("schema_version: 2\n").unwrap_err(),
            DebtError::Shape(_)
        ));
        assert!(matches!(
            Debt::parse("- a\n").unwrap_err(),
            DebtError::Shape(_)
        ));
    }

    #[test]
    fn baseline_records_every_violating_dimension_and_nothing_else() {
        let debt = Debt::baseline(&[
            Measurement::count(GateId::ChapterSizeCap, "./method/a.md", "lines", 250, 200),
            Measurement::count(GateId::ChapterSizeCap, "./method/b.md", "lines", 200, 200),
            Measurement::flag(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-a.md",
                "missing_toc",
                true,
            ),
            Measurement::flag(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-b.md",
                "missing_toc",
                false,
            ),
        ]);
        assert_eq!(
            debt.recorded(GateId::ChapterSizeCap, "method/a.md", "lines"),
            Some(Recorded::Ceiling(250))
        );
        assert_eq!(
            debt.recorded(GateId::ChapterSizeCap, "method/b.md", "lines"),
            None
        );
        assert_eq!(
            debt.recorded(GateId::SpecSizeCap, "_docs/specs/SPEC-a.md", "missing_toc"),
            Some(Recorded::Exception)
        );
        assert_eq!(
            debt.recorded(GateId::SpecSizeCap, "_docs/specs/SPEC-b.md", "missing_toc"),
            None
        );
    }

    #[test]
    fn tighten_lowers_a_ceiling_to_the_measurement() {
        let tightened = sample().tighten(&[
            Measurement::count(
                GateId::ChapterSizeCap,
                "./method/legacy.md",
                "lines",
                300,
                200,
            ),
            Measurement::count(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "authored_lines",
                417,
                300,
            ),
            Measurement::flag(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "missing_toc",
                true,
            ),
        ]);
        assert_eq!(
            tightened
                .debt
                .recorded(GateId::ChapterSizeCap, "method/legacy.md", "lines"),
            Some(Recorded::Ceiling(300))
        );
        assert_eq!(
            tightened.changes,
            vec![Change::Lowered {
                gate: GateId::ChapterSizeCap,
                path: "method/legacy.md".to_string(),
                dimension: "lines",
                from: 417,
                to: 300,
            }]
        );
    }

    #[test]
    fn tighten_never_raises_a_ceiling() {
        let tightened = sample().tighten(&[
            Measurement::count(
                GateId::ChapterSizeCap,
                "method/legacy.md",
                "lines",
                500,
                200,
            ),
            Measurement::count(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "authored_lines",
                417,
                300,
            ),
            Measurement::flag(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "missing_toc",
                true,
            ),
        ]);
        assert_eq!(
            tightened
                .debt
                .recorded(GateId::ChapterSizeCap, "method/legacy.md", "lines"),
            Some(Recorded::Ceiling(417)),
            "the ceiling moved on a document that grew"
        );
        assert!(matches!(
            tightened.changes.as_slice(),
            [Change::Grew {
                ceiling: 417,
                measured: 500,
                ..
            }]
        ));
    }

    #[test]
    fn tighten_clears_a_corrected_exception_and_never_reinstates_one() {
        let tightened = sample().tighten(&[
            Measurement::count(
                GateId::ChapterSizeCap,
                "method/legacy.md",
                "lines",
                417,
                200,
            ),
            Measurement::count(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "authored_lines",
                417,
                300,
            ),
            Measurement::flag(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "missing_toc",
                false,
            ),
        ]);
        assert_eq!(
            tightened.debt.recorded(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "missing_toc"
            ),
            None
        );
        // The condition returns. Tightening records nothing new: the gate
        // fails on it as a fresh violation, and only a baseline could have
        // accepted it.
        let again = tightened.debt.tighten(&[
            Measurement::count(
                GateId::ChapterSizeCap,
                "method/legacy.md",
                "lines",
                417,
                200,
            ),
            Measurement::count(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "authored_lines",
                417,
                300,
            ),
            Measurement::flag(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "missing_toc",
                true,
            ),
        ]);
        assert_eq!(
            again.debt.recorded(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "missing_toc"
            ),
            None
        );
        assert!(again.changes.is_empty());
    }

    #[test]
    fn a_ceiling_reached_by_the_budget_removes_the_entry() {
        let tightened = sample().tighten(&[
            Measurement::count(
                GateId::ChapterSizeCap,
                "method/legacy.md",
                "lines",
                200,
                200,
            ),
            Measurement::count(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "authored_lines",
                300,
                300,
            ),
            Measurement::flag(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "missing_toc",
                true,
            ),
        ]);
        assert_eq!(
            tightened
                .debt
                .recorded(GateId::ChapterSizeCap, "method/legacy.md", "lines"),
            None
        );
        assert_eq!(
            tightened.debt.recorded(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "authored_lines"
            ),
            None
        );
        assert_eq!(
            tightened.debt.recorded(
                GateId::SpecSizeCap,
                "_docs/specs/SPEC-legacy.md",
                "missing_toc"
            ),
            Some(Recorded::Exception)
        );
    }

    #[test]
    fn an_unmeasured_path_leaves_the_file() {
        let tightened = sample().tighten(&[]);
        assert!(tightened.debt.is_empty());
        assert_eq!(tightened.changes.len(), 3);
        assert!(tightened.changes.iter().all(|change| matches!(
            change,
            Change::Removed { reason, .. } if reason == "the gate measures no such path"
        )));
    }

    #[test]
    fn the_legacy_list_is_read_as_written_and_never_trimmed() {
        assert_eq!(
            legacy_list("# exempt\nmethod/a.md\n./method/b.md\n\n method/c.md \n"),
            vec![
                "method/a.md".to_string(),
                "method/b.md".to_string(),
                " method/c.md ".to_string()
            ]
        );
    }

    #[test]
    fn an_empty_debt_renders_no_gate() {
        let rendered = Debt::default().render();
        assert!(rendered.contains("schema_version: 1"));
        assert!(!rendered.contains("chapter-size-cap"));
        assert!(Debt::parse(&rendered).unwrap().is_empty());
    }

    #[test]
    fn a_path_needing_quotes_reads_back() {
        let mut debt = Debt::default();
        debt.record(
            GateId::ChapterSizeCap,
            "./it's/*.md",
            "lines",
            Recorded::Ceiling(3),
        );
        let parsed = Debt::parse(&debt.render()).unwrap();
        assert_eq!(
            parsed.recorded(GateId::ChapterSizeCap, "it's/*.md", "lines"),
            Some(Recorded::Ceiling(3))
        );
    }
}
