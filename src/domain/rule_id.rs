//! Rule identifiers: one variant per requirement the specs define.
//!
//! A rule ID is an address — `domain:rule` — that a failure message cites and
//! a reader follows to the sentence that binds. This enum is the compile-time
//! form of every `` ### `domain:rule` `` heading under the canon's specs; a
//! parity test in `embedded` holds the two sets equal. Rule prose lives in
//! the specs, never here.

use std::fmt;

macro_rules! rule_ids {
    ($($variant:ident => $id:literal,)+) => {
        /// A `domain:rule` requirement address defined by a spec.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum RuleId {
            $(
                #[doc = $id]
                $variant,
            )+
        }

        impl RuleId {
            /// Every rule the specs define, in slug order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The `domain:rule` slug pair this variant addresses.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $id),+
                }
            }
        }
    };
}

rule_ids! {
    CellCarriesOneReference => "comparison-docs:a-cell-carries-one-reference",
    ComparisonCarriesALegend => "comparison-docs:a-comparison-carries-a-legend",
    VerdictCarriesItsWord => "comparison-docs:a-verdict-carries-its-word",
    EveryTableIsDated => "comparison-docs:every-table-is-dated",
    TablePipesAreEscaped => "comparison-docs:table-pipes-are-escaped",
    CitationResolvesToARule => "decision-records:a-citation-resolves-to-a-rule",
    BodyStaysWithinWordCap => "decision-records:body-stays-within-350-words",
    FilenameCarriesNoDigit => "decision-records:filename-carries-no-digit",
    MergedRecordIsPermanent => "decision-records:merged-record-is-permanent",
    RecordIsNotRevised => "decision-records:record-is-not-revised",
    SeededRuleRunsNoCanonCommand => "distribution:a-seeded-rule-runs-no-canon-command",
    SkillHasOneOwner => "distribution:a-skill-has-one-owner",
    SkillInstallRestoresOnFailure => "distribution:a-skill-install-restores-on-failure",
    SkillObeysThePortableFormat => "distribution:a-skill-obeys-the-portable-format",
    SkillPlansBeforeItActs => "distribution:a-skill-plans-before-it-acts",
    StaleSkillIsNotAConflict => "distribution:a-stale-skill-is-not-a-conflict",
    InstallSweepsWhatThePayloadDropped => "distribution:an-install-sweeps-what-the-payload-dropped",
    InitializationPreservesProjectContent => "distribution:initialization-preserves-project-content",
    InstancesOperateOffline => "distribution:instances-operate-offline",
    ManifestIdentifiesEveryOwnedFile => "distribution:manifest-identifies-every-owned-file",
    SharedSkillArtifactsHaveOneHome => "distribution:shared-skill-artifacts-have-one-home",
    SkillInstallPreviewsBeforeWriting => "distribution:skill-install-previews-before-writing",
    SkillUninstallRemovesOnlyWhatItWrote => "distribution:skill-uninstall-removes-only-what-it-wrote",
    SkillsArePartOfThePayload => "distribution:skills-are-part-of-the-payload",
    ThePayloadNamesNoOtherProject => "distribution:the-payload-names-no-other-project",
    ThePayloadNamesNoPlanningTool => "distribution:the-payload-names-no-planning-tool",
    ThePayloadRootsAreDeclaredOnce => "distribution:the-payload-roots-are-declared-once",
    UpgradeConflictsAreAtomic => "distribution:upgrade-conflicts-are-atomic",
    UserScopeFilesStayUnrecorded => "distribution:user-scope-files-stay-unrecorded",
    AuthorInstructionsStayWithinBudget => "docs-format:author-instructions-stay-within-budget",
    ChapterStaysWithinLineCap => "docs-format:chapter-stays-within-200-lines",
    DocumentStatesThePresent => "docs-format:document-states-the-present",
    DocumentUsesStructuralMarkdownOnly => "docs-format:document-uses-structural-markdown-only",
    EveryBudgetCarriesAGate => "docs-format:every-budget-carries-a-gate",
    FenceDeclaresALanguage => "docs-format:fence-declares-a-language",
    ProseStaysUnwrapped => "docs-format:prose-stays-unwrapped",
    DocumentCarriesNoPersonalPath => "docs-foundations:a-document-carries-no-personal-path",
    DocumentOwnsWhatItGoverns => "docs-foundations:a-document-owns-what-it-governs",
    KindPrefixCarriesASlug => "docs-foundations:a-kind-prefix-carries-a-slug",
    ArtifactFilenamesCarryAKindPrefix => "docs-foundations:artifact-filenames-carry-a-kind-prefix",
    CompanionArtifactsShareTheSpecName => "docs-foundations:companion-artifacts-share-the-spec-name",
    SpecStatesThePresent => "docs-foundations:spec-states-the-present",
    SpecWinsOverRecord => "docs-foundations:spec-wins-over-record",
    SpecsAreCentralized => "docs-foundations:specs-are-centralized",
    ProhibitionsAreCapped => "docs-specs:prohibitions-are-capped",
    RequirementCarriesAVerification => "docs-specs:requirement-carries-a-verification",
    RequirementCarriesFiveParts => "docs-specs:requirement-carries-five-parts",
    RuleIdIsUniqueAndSlugged => "docs-specs:rule-id-is-unique-and-slugged",
    RuleIdOutlivesItsSentence => "docs-specs:rule-id-outlives-its-sentence",
    SpecStaysWithinLineCap => "docs-specs:spec-stays-within-300-lines",
    StatementUsesAnEarsPattern => "docs-specs:statement-uses-an-ears-pattern",
    UnenforcedRulesAreDeclared => "docs-specs:unenforced-rules-are-declared",
    VerificationNamesALiveHook => "docs-specs:verification-names-a-live-hook",
    DivergentResultNamesItsDestination => "guides:a-divergent-result-names-its-destination",
    ManualStepEnumeratesItsInteraction => "guides:a-manual-step-enumerates-its-interaction",
    StepFollowsItsProducers => "guides:a-step-follows-its-producers",
    StepIsOneImperativeAction => "guides:a-step-is-one-imperative-action",
    ExternalFactIsVerifiedUpstream => "guides:an-external-fact-is-verified-upstream",
    CitationsLiveInTheReferenceZone => "guides:citations-live-in-the-reference-zone",
    EveryStepCarriesItsCheck => "guides:every-step-carries-its-check",
    PreconditionsOpenAndVerificationCloses => "guides:preconditions-open-and-verification-closes",
    TheManifestStaysReadable => "instance:the-manifest-stays-readable",
    BugzillaReportBodyFitsReportWidth => "known-issues:a-bugzilla-report-body-fits-in-79-columns",
    FiledRecordCarriesItsReport => "known-issues:a-filed-record-carries-its-report",
    RecordCarriesItsRetirementCondition => "known-issues:a-record-carries-its-retirement-condition",
    RecordWalksTheMechanism => "known-issues:a-record-walks-the-mechanism",
    CaseIdIsASlug => "known-issues:case-id-is-a-slug",
    CanonGateIsNotDelivered => "release:a-canon-gate-is-not-delivered",
    ReleasedVersionIsNotReAuthored => "release:a-released-version-is-not-re-authored",
    TagDerivesFromTheVersionFile => "release:a-tag-derives-from-the-version-file",
    LicenseDeclaresBothHalves => "release:license-declares-both-halves",
    CanonRecordDescribesItsTree => "release:the-canon-record-describes-its-tree",
    DeliveredGateSetIsDeclaredOnce => "release:the-delivered-gate-set-is-declared-once",
    VersionsAreSemanticAndAligned => "release:versions-are-semantic-and-aligned",
    CommentCitesTheRule => "spec-to-code:a-comment-cites-the-rule",
    CommentNamesNoRecord => "spec-to-code:a-comment-names-no-record",
    GateMessageCitesTheRule => "spec-to-code:a-gate-message-cites-the-rule",
    SpecChangeIsTyped => "spec-to-code:a-spec-change-is-typed",
    SpecMayLeadItsCode => "spec-to-code:a-spec-may-lead-its-code",
    SuppressionNamesItsCase => "spec-to-code:a-suppression-names-its-case",
    EntryDocumentCitesRuleIds => "spec-to-code:an-entry-document-cites-rule-ids",
    UnenactedRulesAreTheBacklog => "spec-to-code:unenacted-rules-are-the-backlog",
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_well_formed_and_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for rule in RuleId::ALL {
            let id = rule.as_str();
            let (domain, name) = id.split_once(':').unwrap();
            let assert_slug = |part: &str| {
                assert!(!part.is_empty(), "{id} has an empty half");
                assert!(
                    part.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                    "{id} is not a slug pair"
                );
            };
            assert_slug(domain);
            assert_slug(name);
            assert!(seen.insert(id), "{id} is duplicated");
        }
    }

    #[test]
    fn display_renders_the_slug_pair() {
        assert_eq!(
            RuleId::ChapterStaysWithinLineCap.to_string(),
            "docs-format:chapter-stays-within-200-lines"
        );
    }
}
