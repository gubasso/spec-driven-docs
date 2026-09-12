//! Which declarations an instance's own specifications do not yet
//! authorize.
//!
//! A project overruling a specification it owns is exercising ownership, so
//! nothing here fails. This reads the declarations, reads the sentinel each
//! one needs, and reports the ones whose sentinel the local specifications
//! lack. `sdd verify` prints each as a note, and `sdd policy reconcile`
//! offers the correction.

use std::collections::BTreeSet;

use camino::Utf8Path;

use crate::domain::debt::Debt;
use crate::domain::policy::{SENTINELS, Sentinel};
use crate::domain::profile::DocsRoot;
use crate::domain::rule_id::RuleId;
use crate::error::AppError;

/// One declaration its specification does not authorize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconciliation {
    /// The sentinel the specifications lack.
    pub sentinel: &'static Sentinel,
}

impl Reconciliation {
    /// The note `sdd verify` prints for it.
    #[must_use]
    pub fn note(&self, docs_root: DocsRoot) -> String {
        format!(
            "note: {} and no local specification defines `{}`; {} owns it; add the rule to the project's copy",
            self.sentinel.declares,
            self.sentinel.rule,
            crate::domain::profile::resolve_destination(self.sentinel.destination, docs_root)
        )
    }
}

/// Every rule ID the instance's own specifications define.
///
/// # Errors
///
/// I/O errors when a specification cannot be read.
pub fn local_rule_ids(
    target: &Utf8Path,
    docs_root: DocsRoot,
) -> Result<BTreeSet<String>, AppError> {
    let specs = target.join(docs_root.as_str()).join("specs");
    let mut ids = BTreeSet::new();
    let Ok(entries) = specs.read_dir_utf8() else {
        return Ok(ids);
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        #[allow(
            clippy::case_sensitive_file_extension_comparisons,
            reason = "the corpus convention is lowercase"
        )]
        if !path.as_str().ends_with(".md") {
            continue;
        }
        let text = std::fs::read_to_string(path)?;
        ids.extend(crate::embedded::rule_ids_in(&text));
    }
    Ok(ids)
}

/// Whether the declaration a sentinel authorizes is active at the target.
///
/// A debt file that does not parse is not active: the verifier reports it
/// as a failure of its own, and a note beside that failure would name a
/// second problem where there is one.
fn active(target: &Utf8Path, sentinel: &Sentinel) -> bool {
    match sentinel.rule {
        RuleId::RecordedDimensionOnlyShrinks => {
            Debt::read(target).is_ok_and(|debt| !debt.is_empty())
        }
        RuleId::ProjectSelectsOneSource => {
            crate::domain::instance_config::InstanceConfig::read(target).is_ok_and(|declaration| {
                declaration.writing_style.source
                    != crate::domain::instance_config::WritingSource::Builtin
            })
        }
        _ => false,
    }
}

/// Every active declaration whose sentinel the local specifications lack.
///
/// # Errors
///
/// I/O errors when a specification cannot be read.
pub fn needed(target: &Utf8Path, docs_root: DocsRoot) -> Result<Vec<Reconciliation>, AppError> {
    let defined = local_rule_ids(target, docs_root)?;
    Ok(SENTINELS
        .iter()
        .filter(|sentinel| active(target, sentinel))
        .filter(|sentinel| !defined.contains(sentinel.rule.as_str()))
        .map(|sentinel| Reconciliation { sentinel })
        .collect())
}
