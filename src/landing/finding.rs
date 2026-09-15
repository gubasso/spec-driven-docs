//! What the corpus says about itself, as far as a program can prove it.
//!
//! Brownfield was one word for three kinds of debt, and only one of them
//! is the reason the sweep exists. A structural finding is what makes two
//! conventions coexist, so it forces the sweep. A budget finding is a
//! measurement over a cap, so it becomes debt. Style is neither: no
//! delivered gate judges prose, and the engine does not get to claim a
//! document's prose is wrong because it was written first.

use serde::{Deserialize, Serialize};

use crate::landing::path::TargetPath;

/// Which kind of debt a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingKind {
    /// Two conventions would coexist. Only a sweep resolves it.
    Structural,
    /// A measurement is over a cap. Debt records it.
    Budget,
}

/// One thing the corpus shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Which kind of debt it is.
    pub kind: FindingKind,
    /// Where.
    pub path: TargetPath,
    /// The detector that found it, as a stable slug.
    pub rule: String,
    /// One sentence a person reads.
    pub statement: String,
    /// What was measured, where the finding is a measurement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurement: Option<Measurement>,
}

/// A number over a cap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Measurement {
    /// What was counted.
    pub dimension: String,
    /// How much of it there is.
    pub found: u64,
    /// How much the convention allows.
    pub cap: u64,
}

/// A document written before the instance, which nothing here judges.
///
/// Named rather than measured. `writing-style:no-delivered-gate-judges-prose`
/// forbids a delivered gate from judging prose, and an engine that reported
/// a style finding would be doing by another name what that rule refuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleCandidate {
    /// Where.
    pub path: TargetPath,
    /// Why it is a candidate, never why it is wrong.
    pub reason: String,
}

/// The structural detectors, by the slug each finding cites.
pub mod detector {
    /// A populated documentation root that is not the selected profile's.
    pub const FOREIGN_DOCS_ROOT: &str = "foreign-documentation-root";
    /// A document shaped like a specification that defines no rule ID.
    pub const SPEC_WITHOUT_RULE_ID: &str = "spec-without-a-rule-id";
    /// A document named by its position rather than by its subject.
    pub const ORDINAL_FILENAME: &str = "ordinal-filename";
    /// A decision record outside the decisions directory.
    pub const RECORD_OUTSIDE_DECISIONS: &str = "record-outside-the-decisions-directory";
    /// A settled corpus with no specifications directory.
    pub const NO_SPECS_DIRECTORY: &str = "no-specifications-directory";
}

/// Whether a filename is named by its position rather than its subject.
///
/// A number is an identity two branches can both claim, and a slug is not.
/// A subject that starts with a number keeps it: `roadmap-2026` is a
/// subject, and `01-intro` is a position.
#[must_use]
pub fn is_ordinal_name(name: &str) -> bool {
    let stem = name.strip_suffix(".md").unwrap_or(name);
    let Some((head, rest)) = stem.split_once('-') else {
        return false;
    };
    !head.is_empty() && head.bytes().all(|byte| byte.is_ascii_digit()) && !rest.is_empty()
}

/// Whether a document is shaped like a specification of this convention.
///
/// The shape is the filename, because a generic document under `specs/` is
/// the project's own and is not this convention's specification missing a
/// rule. Calling that structural would sweep files nobody adopted.
#[must_use]
pub fn is_spec_shaped(name: &str) -> bool {
    name.starts_with("SPEC-") && is_markdown(name)
}

/// Whether a filename is a markdown document.
///
/// Case-sensitive on purpose: the convention's own filenames are, and a
/// document named with a shouted extension is not one of them.
fn is_markdown(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .is_some_and(|extension| extension == "md")
}

/// Whether a document is shaped like a decision record.
#[must_use]
pub fn is_record_shaped(name: &str) -> bool {
    name.starts_with("ADR-") && is_markdown(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_position_is_an_ordinal_name_and_a_subject_is_not() {
        assert!(is_ordinal_name("01-intro.md"));
        assert!(is_ordinal_name("0007-thing.md"));
        assert!(!is_ordinal_name("roadmap-2026.md"));
        assert!(!is_ordinal_name("intro.md"));
        assert!(!is_ordinal_name("README.md"));
        assert!(!is_ordinal_name("2026.md"));
        assert!(!is_ordinal_name("01-.md"));
    }

    #[test]
    fn only_this_conventions_shapes_are_recognized() {
        assert!(is_spec_shaped("SPEC-distribution.md"));
        assert!(!is_spec_shaped("notes.md"));
        assert!(!is_spec_shaped("SPEC-distribution.rst"));
        assert!(is_record_shaped("ADR-a-choice.md"));
        assert!(!is_record_shaped("decision.md"));
    }
}
