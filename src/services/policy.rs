//! Which declarations an instance's own specifications do not yet
//! authorize.
//!
//! A project overruling a specification it owns is exercising ownership, so
//! nothing here fails. This reads the declarations, reads the sentinel each
//! one needs, and reports the ones whose sentinel the local specifications
//! lack. `sdd verify` prints each as a note, and `sdd policy reconcile`
//! offers the correction.
//!
//! The correction is conservative. It seeds the owning specification where
//! the instance lacks it, and otherwise appends the sentinel's rule block,
//! taken from the embedded seed, to the end of the local file's
//! Requirements section. It never rewrites a sentence the project may have
//! edited: a rule block is self-contained, and the unique-id gate catches a
//! collision. A file not in that shape gets a checklist and no write.

use std::collections::BTreeSet;

use camino::Utf8Path;

use camino::Utf8PathBuf;

use crate::adapters::fs::write_atomic;
use crate::domain::debt::Debt;
use crate::domain::ownership::Sha256;
use crate::domain::policy::{SENTINELS, Sentinel};
use crate::domain::profile::{DocsRoot, resolve_destination};
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
            "note: {} and no local specification defines `{}`; {} owns it; run 'sdd policy reconcile'",
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

/// What one reconciliation would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The owning specification is absent: write the seed.
    Seed {
        /// Where it lands.
        destination: Utf8PathBuf,
        /// The seed's bytes.
        bytes: Vec<u8>,
    },
    /// The owning specification is present and recognized: append the rule
    /// block to its Requirements section.
    Append {
        /// The file.
        destination: Utf8PathBuf,
        /// The rule block, as the seed states it.
        block: String,
        /// The whole file after the append.
        rewritten: String,
    },
    /// The owning specification is present and not in a recognized shape:
    /// print the block and write nothing.
    Checklist {
        /// The file.
        destination: Utf8PathBuf,
        /// The rule block to add by hand.
        block: String,
    },
}

/// One reconciliation and what it would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The declaration and its sentinel.
    pub reconciliation: Reconciliation,
    /// The action.
    pub action: Action,
}

/// The sentinel's rule block, from the heading to the line before the next
/// heading, as the embedded seed states it.
#[must_use]
pub fn rule_block(seed: &str, rule: RuleId) -> Option<String> {
    let heading = format!("### `{rule}`");
    let mut lines = seed.lines().skip_while(|line| !line.starts_with(&heading));
    let first = lines.next()?;
    let mut block = format!("{first}\n");
    for line in lines {
        if line.starts_with("### ") || line.starts_with("## ") {
            break;
        }
        block.push_str(line);
        block.push('\n');
    }
    Some(format!("{}\n", block.trim_end_matches('\n')))
}

/// The file with the block appended to the end of its Requirements section,
/// or `None` where the file has no such section to append to.
#[must_use]
pub fn append_to_requirements(text: &str, block: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|line| *line == "## Requirements")?;
    let end = lines[start + 1..]
        .iter()
        .position(|line| line.starts_with("## "))
        .map_or(lines.len(), |offset| start + 1 + offset);
    let mut out = String::new();
    for line in &lines[..end] {
        out.push_str(line);
        out.push('\n');
    }
    let trimmed = out.trim_end_matches('\n').to_string();
    out = format!("{trimmed}\n\n{block}");
    if end < lines.len() {
        out.push('\n');
        for line in &lines[end..] {
            out.push_str(line);
            out.push('\n');
        }
    }
    Some(out)
}

/// Every reconciliation the target needs, with what each would do.
///
/// # Errors
///
/// I/O errors when a specification cannot be read.
pub fn plan(target: &Utf8Path, docs_root: DocsRoot) -> Result<Vec<Plan>, AppError> {
    let mut plans = Vec::new();
    for reconciliation in needed(target, docs_root)? {
        let sentinel = reconciliation.sentinel;
        let seed = crate::embedded::asset(sentinel.source)
            .ok_or_else(|| anyhow::anyhow!("payload asset missing: {}", sentinel.source))?;
        let seed_text = std::str::from_utf8(seed).map_err(anyhow::Error::from)?;
        let block = rule_block(seed_text, sentinel.rule).ok_or_else(|| {
            anyhow::anyhow!("{} does not define {}", sentinel.source, sentinel.rule)
        })?;
        let destination = resolve_destination(sentinel.destination, docs_root);
        let full = target.join(&destination);
        let action = if full.is_file() {
            let text = std::fs::read_to_string(&full)?;
            match append_to_requirements(&text, &block) {
                Some(rewritten)
                    if crate::embedded::rule_ids_in(&rewritten)
                        .any(|id| id == sentinel.rule.as_str()) =>
                {
                    Action::Append {
                        destination,
                        block,
                        rewritten,
                    }
                }
                _ => Action::Checklist { destination, block },
            }
        } else {
            Action::Seed {
                destination,
                bytes: seed.to_vec(),
            }
        };
        plans.push(Plan {
            reconciliation,
            action,
        });
    }
    Ok(plans)
}

/// Update the manifest's record of one adopted file, or add the record
/// where the file was absent.
///
/// A write that left the record behind would report the instance as drifted
/// the moment it was made correct.
fn record_adopted(
    target: &Utf8Path,
    source: &str,
    destination: &Utf8Path,
    bytes: &[u8],
    baseline: &[u8],
) -> Result<(), AppError> {
    let manifest_path = target.join(crate::domain::manifest::MANIFEST_PATH);
    let text = std::fs::read_to_string(&manifest_path)?;
    let mut document: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;
    let digest = Sha256::of(bytes).to_string();
    let Some(entries) = document
        .get_mut("adopted_files")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return Err(AppError::ManifestInvalid(
            "adopted_files is not an array".to_string(),
        ));
    };
    let recorded = entries.iter_mut().find(|entry| {
        entry.get("destination").and_then(serde_json::Value::as_str) == Some(destination.as_str())
    });
    match recorded {
        Some(entry) => entry["sha256"] = serde_json::Value::String(digest),
        None => entries.push(serde_json::json!({
            "source": source,
            "destination": destination.as_str(),
            "sha256": digest,
            "baseline_sha256": Sha256::of(baseline).to_string(),
        })),
    }
    let rendered = serde_json::to_string_pretty(&document)
        .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;
    write_atomic(&manifest_path, format!("{rendered}\n").as_bytes())?;
    Ok(())
}

/// Carry out one plan, and report the file it wrote.
///
/// The write goes through a sibling temporary file and a rename, and the
/// result is re-read and re-parsed before the record is updated: a
/// specification that would not define the sentinel after the change is
/// not left in place.
///
/// # Errors
///
/// [`AppError::Refused`] for a checklist plan or a rewrite that does not
/// parse back, and I/O errors when the tree cannot be written.
pub fn apply(target: &Utf8Path, plan: &Plan) -> Result<Utf8PathBuf, AppError> {
    let sentinel = plan.reconciliation.sentinel;
    let seed = crate::embedded::asset(sentinel.source)
        .ok_or_else(|| anyhow::anyhow!("payload asset missing: {}", sentinel.source))?;
    let (destination, bytes) = match &plan.action {
        Action::Seed { destination, bytes } => (destination, bytes.clone()),
        Action::Append {
            destination,
            rewritten,
            ..
        } => (destination, rewritten.clone().into_bytes()),
        Action::Checklist { destination, .. } => {
            return Err(AppError::Refused(format!(
                "{destination} is not in a shape this command rewrites; add the rule by hand"
            )));
        }
    };
    let full = target.join(destination);
    if full.is_symlink() {
        return Err(AppError::Refused(format!(
            "{destination} is a symlink; refusing to write through it"
        )));
    }
    let previous = if full.is_file() {
        Some(std::fs::read(&full)?)
    } else {
        None
    };
    write_atomic(&full, &bytes)?;
    let written = std::fs::read_to_string(&full)?;
    if !crate::embedded::rule_ids_in(&written).any(|id| id == sentinel.rule.as_str()) {
        // Put the file back as it was; the record has not moved yet.
        match previous {
            Some(bytes) => write_atomic(&full, &bytes)?,
            None => std::fs::remove_file(&full)?,
        }
        return Err(AppError::Refused(format!(
            "{destination} did not define `{}` after the rewrite; the file is unchanged",
            sentinel.rule
        )));
    }
    record_adopted(target, sentinel.source, destination, &bytes, seed)?;
    Ok(destination.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = "# Sample\n\n## Purpose\n\nOurs.\n\n## Requirements\n\n### `sample:first` — First\n\nThe author MUST keep it.\n\n#### Scenario: One\n\n- GIVEN x\n- WHEN y\n- THEN z\n\nVerify: `true`\n\n## Unenforced\n\n| Rule | Reviewer confirms |\n| --- | --- |\n";

    const BLOCK: &str = "### `sample:second` — Second\n\nThe author MUST add it.\n\n#### Scenario: Two\n\n- GIVEN a\n- WHEN b\n- THEN c\n\nVerify: `true`\n";

    #[test]
    fn the_rule_block_runs_from_its_heading_to_the_next() {
        let seed = format!("{SPEC}\n{BLOCK}");
        let block = rule_block(&seed, RuleId::RecordedDimensionOnlyShrinks);
        assert!(block.is_none(), "a rule the seed lacks is not found");
        let debt =
            std::str::from_utf8(crate::embedded::asset("_docs/specs/SPEC-budget-debt.md").unwrap())
                .unwrap();
        let block = rule_block(debt, RuleId::RecordedDimensionOnlyShrinks).unwrap();
        assert!(block.starts_with("### `budget-debt:a-recorded-dimension-only-shrinks`"));
        assert!(block.contains("Verify:"));
        assert!(!block.contains("debt-is-created-by-an-explicit-act"));
        assert!(block.ends_with('\n') && !block.ends_with("\n\n"));
    }

    #[test]
    fn the_block_is_appended_before_the_next_section_and_everything_else_survives() {
        let out = append_to_requirements(SPEC, BLOCK).unwrap();
        let ids: Vec<String> = crate::embedded::rule_ids_in(&out).collect();
        assert_eq!(
            ids,
            vec!["sample:first".to_string(), "sample:second".to_string()]
        );
        assert!(out.contains("Verify: `true`\n\n### `sample:second`"));
        assert!(out.contains("Verify: `true`\n\n## Unenforced\n"));
        assert!(out.ends_with("| --- | --- |\n"));
        assert!(out.starts_with("# Sample\n\n## Purpose\n\nOurs.\n"));
    }

    #[test]
    fn a_file_whose_requirements_close_it_takes_the_block_at_the_end() {
        let spec = SPEC.split("## Unenforced").next().unwrap();
        let out = append_to_requirements(spec, BLOCK).unwrap();
        assert!(out.ends_with(&format!("\n\n{BLOCK}")));
    }

    #[test]
    fn a_file_without_a_requirements_section_is_not_rewritten() {
        assert!(append_to_requirements("# Ours\n\nProse only.\n", BLOCK).is_none());
    }
}
