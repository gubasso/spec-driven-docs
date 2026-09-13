//! What a release needs of the engine that lands it.
//!
//! A bundle can be authentic, intact, and still wrong for this engine: it
//! may need a newer one, or a version the interval must pass through.
//! Both are declared by the release rather than inferred, because only the
//! release knows what it asked of the tool that wrote it.
//!
//! The declaration is required. An absence that means "no requirement"
//! cannot be told from an absence that means somebody forgot, so a
//! schema-one bundle without the file is invalid and says which file.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::version::CanonVersion;
use crate::plan::readiness::{Evaluation, Precondition, Requirement};

/// Where the declaration sits inside a bundle.
pub const DECLARATION_PATH: &str = "instance/compatibility.toml";

/// The schema this engine reads.
pub const SCHEMA: &str = "sdd.compatibility/1";

/// A declaration this engine cannot read.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CompatibilityError {
    /// The bytes are not the declaration's shape.
    #[error("{DECLARATION_PATH} does not parse: {0}")]
    Malformed(String),

    /// The declaration is written in a schema this engine does not read.
    #[error("{DECLARATION_PATH} declares schema {0}, and this engine reads {SCHEMA}")]
    UnknownSchema(String),
}

/// What one release needs of its engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    /// Always [`SCHEMA`] once parsed.
    pub schema: String,
    /// The lowest engine that can land this release.
    pub minimum_engine: CanonVersion,
    /// Versions an upgrade may not skip.
    #[serde(default)]
    pub must_pass_through: Vec<CanonVersion>,
}

impl Compatibility {
    /// Read one declaration.
    ///
    /// # Errors
    ///
    /// [`CompatibilityError`] when the bytes do not parse or the schema is
    /// one this engine does not read.
    pub fn parse(bytes: &[u8]) -> Result<Self, CompatibilityError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|source| CompatibilityError::Malformed(source.to_string()))?;
        let held: Self = toml::from_str(text)
            .map_err(|source| CompatibilityError::Malformed(source.to_string()))?;
        if held.schema != SCHEMA {
            return Err(CompatibilityError::UnknownSchema(held.schema));
        }
        Ok(held)
    }
}

/// What the plan checks the compatibility against.
#[derive(Debug, Clone)]
pub struct Interval {
    /// The engine running this plan.
    pub engine: CanonVersion,
    /// The release the target records, where it records one.
    pub recorded: Option<CanonVersion>,
    /// The release the plan converges toward.
    pub destination: CanonVersion,
}

/// Every compatibility precondition one interval carries.
///
/// Three axes, each blocked where it fails, because none of them is a
/// judgement call: an engine too old cannot decode the bundle, a skipped
/// version was declared unskippable by the release that needs it, and a
/// target is never downgraded.
#[must_use]
pub fn preconditions(held: &Compatibility, interval: &Interval) -> Vec<Precondition> {
    let mut preconditions = Vec::new();
    if interval.engine < held.minimum_engine {
        preconditions.push(Precondition {
            id: "engine-is-new-enough".to_string(),
            statement: format!("this release needs sdd {} or newer", held.minimum_engine),
            requirement: Requirement::Required,
            evaluation: Evaluation::Unsatisfied {
                reason: format!(
                    "this engine is {} and {} needs {}; install a newer sdd",
                    interval.engine, interval.destination, held.minimum_engine
                ),
            },
            resolved_by: None,
            evidence_refs: vec!["release".to_string()],
        });
    }

    if let Some(recorded) = interval.recorded {
        if recorded > interval.destination {
            preconditions.push(Precondition {
                id: "the-target-is-not-downgraded".to_string(),
                statement: "the destination is not older than what the target records".to_string(),
                requirement: Requirement::Required,
                evaluation: Evaluation::Unsatisfied {
                    reason: format!(
                        "the target records {recorded} and the destination is {}; a target is never downgraded",
                        interval.destination
                    ),
                },
                resolved_by: None,
                evidence_refs: vec!["record".to_string()],
            });
        }
        // A version the release says must be passed through is a small
        // step the operator takes deliberately. The engine names it and
        // never takes it on the operator's behalf.
        let skipped: Vec<String> = held
            .must_pass_through
            .iter()
            .filter(|version| **version > recorded && **version < interval.destination)
            .map(std::string::ToString::to_string)
            .collect();
        if !skipped.is_empty() {
            preconditions.push(Precondition {
                id: "no-version-is-skipped".to_string(),
                statement: "the interval passes through every version that requires it".to_string(),
                requirement: Requirement::Required,
                evaluation: Evaluation::Unsatisfied {
                    reason: format!(
                        "{} must be passed through; plan --to {} first",
                        skipped.join(", "),
                        skipped.first().cloned().unwrap_or_default()
                    ),
                },
                resolved_by: None,
                evidence_refs: vec!["release".to_string()],
            });
        }
    }
    preconditions
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn version(value: &str) -> CanonVersion {
        value.parse().unwrap()
    }

    fn declaration(minimum: &str, through: &[&str]) -> Compatibility {
        Compatibility {
            schema: SCHEMA.to_string(),
            minimum_engine: version(minimum),
            must_pass_through: through.iter().map(|held| version(held)).collect(),
        }
    }

    fn interval(engine: &str, recorded: Option<&str>, destination: &str) -> Interval {
        Interval {
            engine: version(engine),
            recorded: recorded.map(version),
            destination: version(destination),
        }
    }

    #[test]
    fn the_smallest_true_declaration_requires_only_its_engine() {
        let held =
            Compatibility::parse(b"schema = \"sdd.compatibility/1\"\nminimum_engine = \"0.9.0\"\n")
                .unwrap();
        assert_eq!(held.minimum_engine, version("0.9.0"));
        assert!(held.must_pass_through.is_empty());
        assert!(preconditions(&held, &interval("0.9.0", None, "0.9.0")).is_empty());
    }

    #[test]
    fn an_unknown_schema_or_shape_refuses() {
        assert!(matches!(
            Compatibility::parse(b"schema = \"sdd.compatibility/9\"\nminimum_engine = \"0.9.0\"\n")
                .unwrap_err(),
            CompatibilityError::UnknownSchema(_)
        ));
        assert!(matches!(
            Compatibility::parse(b"minimum_engine = \"0.9.0\"\n").unwrap_err(),
            CompatibilityError::Malformed(_)
        ));
        assert!(matches!(
            Compatibility::parse(
                b"schema = \"sdd.compatibility/1\"\nminimum_engine = \"0.9.0\"\nextra = 1\n"
            )
            .unwrap_err(),
            CompatibilityError::Malformed(_)
        ));
    }

    #[test]
    fn an_engine_below_requirement_is_blocked_naming_the_engine() {
        let held = declaration("0.9.0", &[]);
        let found = preconditions(&held, &interval("0.8.1", Some("0.8.0"), "0.9.0"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "engine-is-new-enough");
        assert_eq!(found[0].requirement, Requirement::Required);
        assert_eq!(
            found[0].verdict(),
            crate::plan::readiness::Readiness::Blocked
        );
    }

    #[test]
    fn a_skipped_intermediate_version_is_blocked_naming_it() {
        let held = declaration("0.1.0", &["0.8.0"]);
        let found = preconditions(&held, &interval("1.0.0", Some("0.7.0"), "0.9.0"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "no-version-is-skipped");
        let reason = format!("{:?}", found[0].evaluation);
        assert!(reason.contains("0.8.0"), "{reason}");
        assert!(reason.contains("--to 0.8.0"), "{reason}");
    }

    #[test]
    fn a_version_outside_the_interval_is_not_skipped() {
        let held = declaration("0.1.0", &["0.5.0", "0.9.5"]);
        assert!(preconditions(&held, &interval("1.0.0", Some("0.7.0"), "0.9.0")).is_empty());
    }

    #[test]
    fn a_downgrade_is_blocked() {
        let held = declaration("0.1.0", &[]);
        let found = preconditions(&held, &interval("1.0.0", Some("0.9.0"), "0.8.0"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "the-target-is-not-downgraded");
    }
}
