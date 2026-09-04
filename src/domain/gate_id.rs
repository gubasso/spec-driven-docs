//! Gate identifiers: one variant per delivered gate.
//!
//! A gate ID names an executable check a consumer wires as a pre-commit hook
//! and invokes as `sdd gate <id>`. This enum is only the identity; what a
//! gate checks, how it is wired, and its display name live in the gate
//! registry, and canon-only checks are cargo tests rather than variants here.

use std::fmt;

use clap::ValueEnum;

/// A delivered gate, addressable as `sdd gate <id>` and as a pre-commit hook id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum GateId {
    /// Decision record citations resolve to a live rule.
    AdrCitesALiveRule,
    /// Decision record filenames are dated-free slugs.
    AdrFilenameShape,
    /// Decision record bodies stay within the word cap.
    AdrWordCap,
    /// Agent digests stay within their line budgets.
    AgentsDigestSize,
    /// Chapters and catalogs stay within their line caps.
    ChapterSizeCap,
    /// Comparison tables carry a verification date.
    ComparisonDatedTables,
    /// Comparison table pipes are escaped inside cells.
    ComparisonEscapedPipes,
    /// Comparison documents carry a legend.
    ComparisonLegend,
    /// Comparison cells carry at most one reference.
    ComparisonOneReferencePerCell,
    /// Comparison verdicts carry their word.
    ComparisonVerdictWord,
    /// Every rule ID a gate can print resolves to a local requirement.
    GateMessageCitesARule,
    /// The instance manifest is present and coherent.
    InstanceManifest,
    /// Bugzilla-bound report bodies fit the tracker's width.
    KiBugzillaReportWidth,
    /// Known-issue filenames are slugged case IDs.
    KiFilenameShape,
    /// Known-issue records carry one filing state.
    KiFiling,
    /// Known-issue records walk the mechanism.
    KiMechanismWalkthrough,
    /// Filed known-issue records carry their report body.
    KiReportBody,
    /// Known-issue records carry a retirement condition.
    KiRetireWhen,
    /// Known-issue records carry one state.
    KiState,
    /// Files carry no absolute path into a person's home directory.
    NoPersonalPath,
    /// Documents state the present rather than narrate their edits.
    NoSelfNarration,
    /// Paragraphs occupy one source line rather than hard-wrap.
    ProseStaysUnwrapped,
    /// Spec requirements carry all five parts.
    SpecRequirementParts,
    /// Rule IDs are unique across the local specs.
    SpecRuleIdUnique,
    /// Specs stay within their line cap and carry a TOC when long.
    SpecSizeCap,
    /// Spec verification lines name hooks that exist.
    SpecVerifyHooksExist,
    /// Suppression comments name a known-issue case.
    SuppressionNamesItsCase,
}

impl GateId {
    /// Every delivered gate, in id order.
    pub const ALL: &'static [Self] = &[
        Self::AdrCitesALiveRule,
        Self::AdrFilenameShape,
        Self::AdrWordCap,
        Self::AgentsDigestSize,
        Self::ChapterSizeCap,
        Self::ComparisonDatedTables,
        Self::ComparisonEscapedPipes,
        Self::ComparisonLegend,
        Self::ComparisonOneReferencePerCell,
        Self::ComparisonVerdictWord,
        Self::GateMessageCitesARule,
        Self::InstanceManifest,
        Self::KiBugzillaReportWidth,
        Self::KiFilenameShape,
        Self::KiFiling,
        Self::KiMechanismWalkthrough,
        Self::KiReportBody,
        Self::KiRetireWhen,
        Self::KiState,
        Self::NoPersonalPath,
        Self::NoSelfNarration,
        Self::ProseStaysUnwrapped,
        Self::SpecRequirementParts,
        Self::SpecRuleIdUnique,
        Self::SpecSizeCap,
        Self::SpecVerifyHooksExist,
        Self::SuppressionNamesItsCase,
    ];

    /// The kebab-case id used on the command line and as the hook id.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdrCitesALiveRule => "adr-cites-a-live-rule",
            Self::AdrFilenameShape => "adr-filename-shape",
            Self::AdrWordCap => "adr-word-cap",
            Self::AgentsDigestSize => "agents-digest-size",
            Self::ChapterSizeCap => "chapter-size-cap",
            Self::ComparisonDatedTables => "comparison-dated-tables",
            Self::ComparisonEscapedPipes => "comparison-escaped-pipes",
            Self::ComparisonLegend => "comparison-legend",
            Self::ComparisonOneReferencePerCell => "comparison-one-reference-per-cell",
            Self::ComparisonVerdictWord => "comparison-verdict-word",
            Self::GateMessageCitesARule => "gate-message-cites-a-rule",
            Self::InstanceManifest => "instance-manifest",
            Self::KiBugzillaReportWidth => "ki-bugzilla-report-width",
            Self::KiFilenameShape => "ki-filename-shape",
            Self::KiFiling => "ki-filing",
            Self::KiMechanismWalkthrough => "ki-mechanism-walkthrough",
            Self::KiReportBody => "ki-report-body",
            Self::KiRetireWhen => "ki-retire-when",
            Self::KiState => "ki-state",
            Self::NoPersonalPath => "no-personal-path",
            Self::NoSelfNarration => "no-self-narration",
            Self::ProseStaysUnwrapped => "prose-stays-unwrapped",
            Self::SpecRequirementParts => "spec-requirement-parts",
            Self::SpecRuleIdUnique => "spec-rule-id-unique",
            Self::SpecSizeCap => "spec-size-cap",
            Self::SpecVerifyHooksExist => "spec-verify-hooks-exist",
            Self::SuppressionNamesItsCase => "suppression-names-its-case",
        }
    }
}

impl std::str::FromStr for GateId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|gate| gate.as_str() == s)
            .ok_or_else(|| format!("unknown gate: {s}"))
    }
}

impl fmt::Display for GateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_every_variant_once() {
        let mut seen = std::collections::BTreeSet::new();
        for gate in GateId::ALL {
            assert!(seen.insert(gate.as_str()), "{gate} is duplicated");
        }
        assert_eq!(GateId::ALL.len(), 27);
    }

    #[test]
    fn clap_value_matches_as_str() {
        for gate in GateId::ALL {
            let value = gate.to_possible_value().unwrap();
            assert_eq!(value.get_name(), gate.as_str());
        }
    }
}
