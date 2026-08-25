//! A gate finding: one violation, addressed to the rule it breaks.
//!
//! Rendering is fixed here so every failure line a consumer sees has one
//! shape — `FAIL <domain>:<rule> [<path>[:<line>]]: <detail>` — and always
//! cites a rule a spec defines. What counts as a violation is each gate's
//! business, not this type's.

use std::fmt;

use camino::Utf8PathBuf;

use crate::domain::rule_id::RuleId;

/// One violation found by a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The requirement the violation breaks.
    pub rule: RuleId,
    /// The offending file, when the violation has one.
    pub path: Option<Utf8PathBuf>,
    /// The offending line within `path`, when the violation has one.
    pub line: Option<usize>,
    /// What is wrong, stated for the reader who will fix it.
    pub detail: String,
}

impl Finding {
    /// A finding anchored to a file.
    #[must_use]
    pub fn on_file(rule: RuleId, path: impl Into<Utf8PathBuf>, detail: impl Into<String>) -> Self {
        Self {
            rule,
            path: Some(path.into()),
            line: None,
            detail: detail.into(),
        }
    }

    /// A finding anchored to one line of a file.
    #[must_use]
    pub fn on_line(
        rule: RuleId,
        path: impl Into<Utf8PathBuf>,
        line: usize,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            rule,
            path: Some(path.into()),
            line: Some(line),
            detail: detail.into(),
        }
    }

    /// A finding about the repository as a whole.
    #[must_use]
    pub fn global(rule: RuleId, detail: impl Into<String>) -> Self {
        Self {
            rule,
            path: None,
            line: None,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FAIL {}", self.rule)?;
        if let Some(path) = &self.path {
            write!(f, " {path}")?;
            if let Some(line) = self.line {
                write!(f, ":{line}")?;
            }
        }
        if self.detail.is_empty() {
            return Ok(());
        }
        write!(f, ": {}", self.detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_with_path_and_line() {
        let finding = Finding::on_line(
            RuleId::BugzillaReportBodyFitsReportWidth,
            "_docs/reference/known-issues/KI-vendor-500.md",
            12,
            "83 columns",
        );
        assert_eq!(
            finding.to_string(),
            "FAIL known-issues:a-bugzilla-report-body-fits-in-79-columns \
             _docs/reference/known-issues/KI-vendor-500.md:12: 83 columns"
        );
    }

    #[test]
    fn renders_without_a_path() {
        let finding = Finding::global(
            RuleId::GateMessageCitesTheRule,
            "x:y resolves to no requirement",
        );
        assert_eq!(
            finding.to_string(),
            "FAIL spec-to-code:a-gate-message-cites-the-rule: x:y resolves to no requirement"
        );
    }
}
