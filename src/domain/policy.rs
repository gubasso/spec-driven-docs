//! The sentinel rules: one per feature a project can declare against its
//! own adopted specifications.
//!
//! A feature is available the moment the binary carries it. Its normative
//! text arrives as a new adopted specification, seeded on the next upgrade,
//! and each such specification carries one rule that authorizes the
//! declaration. That rule ID is the sentinel. Whether an instance's
//! specifications authorize what its configuration declares is read by
//! presence of the sentinel and never by matching prose, because a rule ID
//! survives rewording and a sentence does not.
//!
//! A sentinel never joins a delivered gate's `cites` list. That gate check is
//! always-run, so a cited ID an instance's specifications lack would fail
//! every commit in the window between upgrading the binary and running
//! `sdd upgrade`. The canon suite holds that boundary.

use crate::domain::rule_id::RuleId;

/// One sentinel: the rule, and the adopted specification that owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sentinel {
    /// The rule an instance's specifications must define.
    pub rule: RuleId,
    /// The embedded specification that carries it.
    pub source: &'static str,
    /// Where an instance holds that specification, with `{docs_root}`
    /// templated.
    pub destination: &'static str,
    /// The declaration it authorizes, for the note that names it.
    pub declares: &'static str,
}

/// Every sentinel this binary knows.
pub const SENTINELS: &[Sentinel] = &[Sentinel {
    rule: RuleId::RecordedDimensionOnlyShrinks,
    source: "_docs/specs/SPEC-budget-debt.md",
    destination: "{docs_root}/specs/SPEC-budget-debt.md",
    declares: ".spec-driven-docs/debt.yaml records budget debt",
}];

/// The sentinel a rule is, if it is one.
#[must_use]
pub fn sentinel(rule: RuleId) -> Option<&'static Sentinel> {
    SENTINELS.iter().find(|s| s.rule == rule)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sentinel_is_owned_by_an_embedded_adopted_specification() {
        for s in SENTINELS {
            assert!(
                crate::embedded::asset(s.source).is_some(),
                "{} is not embedded",
                s.source
            );
            let adopted = crate::domain::profile::ProfileId::KnowledgeBase
                .profile()
                .adopted
                .iter()
                .any(|p| p.source == s.source && p.destination == s.destination);
            assert!(adopted, "{} is not an adopted projection", s.source);
            let text = crate::embedded::asset(s.source)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .unwrap_or_default();
            assert!(
                crate::embedded::rule_ids_in(text).any(|id| id == s.rule.as_str()),
                "{} does not define {}",
                s.source,
                s.rule
            );
        }
    }

    #[test]
    fn no_delivered_gate_cites_a_sentinel() {
        for row in crate::gates::GATES {
            for rule in row.cites {
                assert!(
                    sentinel(*rule).is_none(),
                    "{} cites the sentinel {rule}; an instance upgraded after the binary would fail every commit",
                    row.id
                );
            }
        }
    }
}
