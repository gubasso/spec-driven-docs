//! Read the known-issue zone as data.
//!
//! The index is derived, never stored: a committed listing is a second copy
//! of what each record already states, and one file every new case edits is
//! the branch collision the slug case id exists to avoid. Reading through
//! the same resolver the gates use keeps the listing and the gates agreed on
//! what counts as a record.

use camino::Utf8Path;
use serde::Serialize;

use crate::error::AppError;
use crate::gates::front_matter_values;
use crate::gates::paths::ki_records;
use crate::gates::{GateCtx, read_text};

/// One known-issue record, as `sdd ki list` reports it.
#[derive(Debug, Serialize)]
pub struct Case {
    /// The case id: the filename without its extension.
    pub id: String,
    /// How this project handles the defect, where the record states it.
    pub state: Option<String>,
    /// Where the case stands upstream, where the record states it.
    pub filing: Option<String>,
    /// The upstream issue or tracker, where the record names one.
    pub upstream: Option<String>,
    /// The record's path, relative to the target.
    pub path: String,
}

/// Every known-issue record under the target, in path order.
///
/// # Errors
///
/// [`AppError::Io`] when a record cannot be read.
pub fn cases(target: &Utf8Path) -> Result<Vec<Case>, AppError> {
    let ctx = GateCtx::new(target);
    let mut cases = Vec::new();
    for record in ki_records(&ctx, &[])? {
        let text = read_text(&ctx, &record)?;
        cases.push(Case {
            id: record.file_stem().unwrap_or_default().to_string(),
            state: first(&text, "state"),
            filing: first(&text, "filing"),
            upstream: first(&text, "upstream"),
            path: record.to_string(),
        });
    }
    Ok(cases)
}

/// The first value a front-matter key carries, where it carries one.
fn first(text: &str, key: &str) -> Option<String> {
    front_matter_values(text, key)
        .into_iter()
        .find(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_with(record: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let records = dir.path().join("_docs/reference/known-issues");
        std::fs::create_dir_all(&records).unwrap();
        std::fs::write(records.join("KI-vendor-replays.md"), record).unwrap();
        dir
    }

    #[test]
    fn reads_both_axes_and_the_upstream_reference() {
        let dir = target_with(
            "---\nupstream: https://example.invalid/issues/1\nstate: masked\nfiling: filed\n---\n# V\n",
        );
        let target = camino::Utf8Path::from_path(dir.path()).unwrap();
        let cases = cases(target).unwrap();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].id, "KI-vendor-replays");
        assert_eq!(cases[0].state.as_deref(), Some("masked"));
        assert_eq!(cases[0].filing.as_deref(), Some("filed"));
        assert_eq!(
            cases[0].path,
            "_docs/reference/known-issues/KI-vendor-replays.md"
        );
    }

    #[test]
    fn a_record_stating_no_axis_reports_none_rather_than_failing() {
        let dir = target_with("---\naffects: client\n---\n# V\n");
        let target = camino::Utf8Path::from_path(dir.path()).unwrap();
        let cases = cases(target).unwrap();
        assert_eq!(cases.len(), 1);
        assert!(cases[0].state.is_none());
        assert!(cases[0].filing.is_none());
        assert!(cases[0].upstream.is_none());
    }

    #[test]
    fn a_target_with_no_zone_lists_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let target = camino::Utf8Path::from_path(dir.path()).unwrap();
        assert!(cases(target).unwrap().is_empty());
    }
}
