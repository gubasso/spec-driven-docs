//! Offline tracking-registry evaluation.
//!
//! Locate `<docs_root>/reference/tracking.yaml`, parse and bound it through
//! `domain::tracking`, resolve every declared path without a `..` or a
//! symlink escape, and classify each entry as current, due, or overdue
//! against a clock the caller supplies. Everything here is local: no network.
//! Comparing a pinned revision to its upstream is `commands::track`'s job,
//! over an explicit network boundary.

use camino::{Utf8Path, Utf8PathBuf};
use jiff::civil::Date;

use crate::adapters::fs::{DestinationRefusal, check_destination};
use crate::domain::rule_id::RuleId;
use crate::domain::tracking::{self, Entry, TrackingError};

/// Where the registry lives, relative to the repository root.
#[must_use]
pub fn registry_path(docs_root: &Utf8Path) -> Utf8PathBuf {
    docs_root.join("reference/tracking.yaml")
}

/// How an entry stands against the clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Freshness {
    /// The next check is in the future.
    Current,
    /// The next check is today or earlier by this many days (0 means today).
    Overdue(i64),
}

/// One problem found in an entry, addressed to the rule it breaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The rule the problem breaks.
    pub rule: RuleId,
    /// What is wrong, stated for the reader who will fix it.
    pub detail: String,
}

/// One evaluated entry.
#[derive(Debug, Clone)]
pub struct Assessed {
    /// The entry as parsed.
    pub entry: Entry,
    /// The date the next check falls due, when the date parsed.
    pub next_check: Option<Date>,
    /// How the entry stands against the clock, when the date parsed.
    pub freshness: Option<Freshness>,
    /// Every problem the entry carries.
    pub problems: Vec<Problem>,
}

/// A whole-registry evaluation.
#[derive(Debug)]
pub struct Report {
    /// Every entry, evaluated.
    pub entries: Vec<Assessed>,
    /// A registry-level failure that stops per-entry evaluation.
    pub fatal: Option<Problem>,
}

impl Report {
    /// Whether the registry has any problem or overdue entry.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.fatal.is_some()
            || self.entries.iter().any(|a| {
                !a.problems.is_empty() || matches!(a.freshness, Some(Freshness::Overdue(_)))
            })
    }
}

/// Today's date in UTC.
#[must_use]
pub fn today_utc() -> Date {
    jiff::Timestamp::now()
        .to_zoned(jiff::tz::TimeZone::UTC)
        .date()
}

const fn rule_for(error: &TrackingError) -> RuleId {
    match error {
        TrackingError::Bounds(_) | TrackingError::Shape(_) => RuleId::RegistryHasOneReadableShape,
        TrackingError::Semantic(_) => RuleId::PerishableSourceIsRegistered,
    }
}

/// Resolve a repository-relative path for reading, refusing an escape.
fn resolvable(root: &Utf8Path, relative: &str) -> Result<bool, String> {
    if relative.starts_with('/') {
        return Err("path is absolute".to_string());
    }
    if relative == ".."
        || relative.starts_with("../")
        || relative.contains("/../")
        || relative.ends_with("/..")
    {
        return Err("path escapes the tree with '..'".to_string());
    }
    match check_destination(root, Utf8Path::new(relative)) {
        Ok(()) => Ok(root.join(relative).is_file()),
        Err(DestinationRefusal::SymlinkEscape) => {
            Err("path is reached through a symlink".to_string())
        }
        Err(DestinationRefusal::FileBlocksDirectory(_) | DestinationRefusal::NotARegularFile) => {
            Ok(root.join(relative).is_file())
        }
    }
}

fn assess_entry(root: &Utf8Path, entry: &Entry, as_of: Date) -> Assessed {
    let mut problems = Vec::new();

    for (label, path) in std::iter::once(("path", &entry.path))
        .chain(entry.dependents.iter().map(|d| ("dependent", d)))
    {
        match resolvable(root, path) {
            Ok(true) => {}
            Ok(false) => problems.push(Problem {
                rule: RuleId::DeclaredDependentExists,
                detail: format!("entry '{}': {label} '{path}' names no file", entry.id),
            }),
            Err(reason) => problems.push(Problem {
                rule: RuleId::DeclaredDependentExists,
                detail: format!("entry '{}': {label} '{path}' {reason}", entry.id),
            }),
        }
    }

    if entry.revalidate.is_empty() {
        problems.push(Problem {
            rule: RuleId::EntryDeclaresHowToRevalidate,
            detail: format!("entry '{}': revalidate lists no step", entry.id),
        });
    }

    let (next_check, freshness) = if let Ok(last) = entry.last_checked.parse::<Date>() {
        let next = last
            .checked_add(jiff::Span::new().days(i64::from(entry.cadence_days)))
            .unwrap_or(last);
        let freshness = if next >= as_of {
            Freshness::Current
        } else {
            Freshness::Overdue((as_of - next).get_days().into())
        };
        if let Freshness::Overdue(_) = freshness {
            problems.push(Problem {
                rule: RuleId::OverdueEntryBlocks,
                detail: format!(
                    "entry '{}' was due {next}; revalidate it, then advance last_checked. Steps: {}",
                    entry.id,
                    entry.revalidate.join("; ")
                ),
            });
        }
        (Some(next), Some(freshness))
    } else {
        problems.push(Problem {
            rule: RuleId::PerishableSourceIsRegistered,
            detail: format!(
                "entry '{}': last_checked '{}' is not an ISO date",
                entry.id, entry.last_checked
            ),
        });
        (None, None)
    };

    Assessed {
        entry: entry.clone(),
        next_check,
        freshness,
        problems,
    }
}

/// Evaluate the registry at `<docs_root>/reference/tracking.yaml`, offline.
///
/// # Errors
///
/// I/O errors when the registry file cannot be read.
pub fn evaluate(root: &Utf8Path, docs_root: &Utf8Path, as_of: Date) -> std::io::Result<Report> {
    let path = root.join(registry_path(docs_root));
    let text = std::fs::read_to_string(&path)?;
    match tracking::parse(&text) {
        Ok(registry) => {
            let entries = registry
                .tracked
                .iter()
                .map(|entry| assess_entry(root, entry, as_of))
                .collect();
            Ok(Report {
                entries,
                fatal: None,
            })
        }
        Err(error) => Ok(Report {
            entries: Vec::new(),
            fatal: Some(Problem {
                rule: rule_for(&error),
                detail: error.to_string(),
            }),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &tempfile::TempDir, rel: &str, text: &str) {
        let p = dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn root(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap())
    }

    fn registry(dependent: &str, last_checked: &str) -> String {
        format!(
            "schema_version: 1\ntracked:\n  - id: sample\n    path: _docs/reference/x.md\n    last_checked: {last_checked}\n    cadence_days: 30\n    why: it moves\n    revalidate:\n      - re-fetch it\n    dependents:\n      - {dependent}\n"
        )
    }

    #[test]
    fn a_current_entry_has_no_problem() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            "_docs/reference/tracking.yaml",
            &registry("_docs/guide.md", "2026-08-20"),
        );
        write(&dir, "_docs/reference/x.md", "# x\n");
        write(&dir, "_docs/guide.md", "# g\n");
        let report = evaluate(
            &root(&dir),
            Utf8Path::new("_docs"),
            "2026-09-01".parse().unwrap(),
        )
        .unwrap();
        assert!(!report.has_failures());
        assert!(matches!(
            report.entries[0].freshness,
            Some(Freshness::Current)
        ));
    }

    #[test]
    fn an_overdue_entry_fails_with_the_due_date() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            "_docs/reference/tracking.yaml",
            &registry("_docs/guide.md", "2026-01-01"),
        );
        write(&dir, "_docs/reference/x.md", "# x\n");
        write(&dir, "_docs/guide.md", "# g\n");
        let report = evaluate(
            &root(&dir),
            Utf8Path::new("_docs"),
            "2026-09-01".parse().unwrap(),
        )
        .unwrap();
        assert!(report.has_failures());
        let overdue = &report.entries[0];
        assert!(matches!(overdue.freshness, Some(Freshness::Overdue(_))));
        assert!(
            overdue
                .problems
                .iter()
                .any(|p| p.rule == RuleId::OverdueEntryBlocks)
        );
        assert!(
            overdue
                .problems
                .iter()
                .any(|p| p.detail.contains("2026-01-31"))
        );
    }

    #[test]
    fn a_missing_dependent_fails() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            "_docs/reference/tracking.yaml",
            &registry("_docs/gone.md", "2026-08-25"),
        );
        write(&dir, "_docs/reference/x.md", "# x\n");
        let report = evaluate(
            &root(&dir),
            Utf8Path::new("_docs"),
            "2026-09-01".parse().unwrap(),
        )
        .unwrap();
        assert!(report.has_failures());
        assert!(
            report.entries[0]
                .problems
                .iter()
                .any(|p| p.rule == RuleId::DeclaredDependentExists && p.detail.contains("gone.md"))
        );
    }

    #[test]
    fn the_exact_due_boundary_is_current() {
        let dir = tempfile::tempdir().unwrap();
        // last_checked + 30 days = 2026-01-31; as_of 2026-01-31 is not past it.
        write(
            &dir,
            "_docs/reference/tracking.yaml",
            &registry("_docs/guide.md", "2026-01-01"),
        );
        write(&dir, "_docs/reference/x.md", "# x\n");
        write(&dir, "_docs/guide.md", "# g\n");
        let report = evaluate(
            &root(&dir),
            Utf8Path::new("_docs"),
            "2026-01-31".parse().unwrap(),
        )
        .unwrap();
        assert!(matches!(
            report.entries[0].freshness,
            Some(Freshness::Current)
        ));
        let report = evaluate(
            &root(&dir),
            Utf8Path::new("_docs"),
            "2026-02-01".parse().unwrap(),
        )
        .unwrap();
        assert!(matches!(
            report.entries[0].freshness,
            Some(Freshness::Overdue(1))
        ));
    }

    #[test]
    fn a_broken_registry_reports_one_fatal() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir,
            "_docs/reference/tracking.yaml",
            "schema_version: 2\ntracked: []\n",
        );
        let report = evaluate(&root(&dir), Utf8Path::new("_docs"), today_utc()).unwrap();
        assert!(report.fatal.is_some());
        assert!(report.has_failures());
    }
}
