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

/// The manifest's JSON with one adopted record updated, or added where the
/// file was absent.
///
/// A write that left the record behind would report the instance as drifted
/// the moment it was made correct.
fn with_adopted_record(
    document: &mut serde_json::Value,
    source: &str,
    destination: &Utf8Path,
    bytes: &[u8],
    baseline: &[u8],
) -> Result<(), AppError> {
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
    Ok(())
}

type Write = (Utf8PathBuf, Vec<u8>, &'static Sentinel);

/// The writes a set of plans amounts to, every destination checked to stay
/// inside the target, or the refusal that stops the whole apply.
fn preflight(target: &Utf8Path, plans: &[Plan]) -> Result<Vec<Write>, AppError> {
    let mut writes: Vec<Write> = Vec::new();
    for plan in plans {
        let sentinel = plan.reconciliation.sentinel;
        match &plan.action {
            Action::Seed { destination, bytes } => {
                writes.push((destination.clone(), bytes.clone(), sentinel));
            }
            Action::Append {
                destination,
                rewritten,
                ..
            } => writes.push((
                destination.clone(),
                rewritten.clone().into_bytes(),
                sentinel,
            )),
            Action::Checklist { destination, .. } => {
                return Err(AppError::Refused(format!(
                    "{destination} is not in a shape this command rewrites; add the rule by hand"
                )));
            }
        }
    }
    for (destination, _, _) in &writes {
        crate::adapters::fs::check_destination(target, destination)
            .map_err(|refusal| AppError::Refused(format!("{destination}: {refusal}")))?;
    }
    let manifest_relative = Utf8Path::new(crate::domain::manifest::MANIFEST_PATH);
    crate::adapters::fs::check_destination(target, manifest_relative)
        .map_err(|refusal| AppError::Refused(format!("{manifest_relative}: {refusal}")))?;
    Ok(writes)
}

/// Put every backed-up path back, and name the ones that could not be.
///
/// A path whose bytes already equal its backup is left alone, so a file
/// the failure never reached is not rewritten through the same failing
/// primitive.
fn restore(target: &Utf8Path, backups: &[(Utf8PathBuf, Option<Vec<u8>>)]) -> Vec<Utf8PathBuf> {
    let mut unrestored = Vec::new();
    for (destination, previous) in backups {
        let full = target.join(destination);
        // Only an absent file is evidence of absence. Any other read failure
        // says nothing about the file, so the restoration is attempted and
        // its own result decides.
        let current = match std::fs::read(&full) {
            Ok(bytes) => Some(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(None),
            Err(_) => None,
        };
        if current.as_ref() == Some(previous) {
            continue;
        }
        let put_back = previous.as_ref().map_or_else(
            || std::fs::remove_file(&full).is_ok() || !full.exists(),
            |bytes| write_atomic(&full, bytes).is_ok(),
        );
        if !put_back {
            unrestored.push(destination.clone());
        }
    }
    unrestored
}

/// Carry out every plan as one transaction, and report the files written.
///
/// Every destination and the manifest are checked to stay inside the
/// target before a byte lands, every existing file is backed up, each
/// specification is written atomically and re-read to confirm it defines
/// its sentinel, and the manifest is written last with every record moved
/// at once. Any failure restores every path this call touched, so the
/// operator never holds an adopted file the record does not describe, or
/// one plan applied and another not.
///
/// # Errors
///
/// [`AppError::Refused`] for a checklist plan, a destination that leaves
/// the target, or a rewrite that does not define its sentinel, and
/// manifest and I/O errors when the tree cannot be read or written. The
/// target is restored before any of these returns.
pub fn apply_all(target: &Utf8Path, plans: &[Plan]) -> Result<Vec<Utf8PathBuf>, AppError> {
    let manifest_relative = Utf8Path::new(crate::domain::manifest::MANIFEST_PATH);
    let writes = preflight(target, plans)?;
    let manifest_text = std::fs::read_to_string(target.join(manifest_relative))?;
    let mut document: serde_json::Value = serde_json::from_str(&manifest_text)
        .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;

    let mut backups: Vec<(Utf8PathBuf, Option<Vec<u8>>)> = Vec::new();
    let mut attempt = |backups: &mut Vec<(Utf8PathBuf, Option<Vec<u8>>)>| -> Result<(), AppError> {
        for (destination, bytes, sentinel) in &writes {
            let full = target.join(destination);
            let previous = if full.is_file() {
                Some(std::fs::read(&full)?)
            } else {
                None
            };
            backups.push((destination.clone(), previous));
            write_atomic(&full, bytes)?;
            let written = std::fs::read_to_string(&full)?;
            if !crate::embedded::rule_ids_in(&written).any(|id| id == sentinel.rule.as_str()) {
                return Err(AppError::Refused(format!(
                    "{destination} did not define `{}` after the rewrite",
                    sentinel.rule
                )));
            }
            let seed = crate::embedded::asset(sentinel.source)
                .ok_or_else(|| anyhow::anyhow!("payload asset missing: {}", sentinel.source))?;
            with_adopted_record(&mut document, sentinel.source, destination, bytes, seed)?;
        }
        backups.push((
            manifest_relative.to_path_buf(),
            Some(manifest_text.clone().into_bytes()),
        ));
        let rendered = serde_json::to_string_pretty(&document)
            .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;
        write_atomic(
            &target.join(manifest_relative),
            format!("{rendered}\n").as_bytes(),
        )?;
        Ok(())
    };
    if let Err(error) = attempt(&mut backups) {
        let unrestored = restore(target, &backups);
        let cause = match error {
            AppError::Refused(reason) => reason,
            other => format!("reconciliation aborted: {other}"),
        };
        if unrestored.is_empty() {
            return Err(AppError::Refused(format!(
                "{cause}; every file is restored"
            )));
        }
        let paths: Vec<&str> = unrestored.iter().map(|p| p.as_str()).collect();
        return Err(AppError::Refused(format!(
            "{cause}; restoration is incomplete, verify by hand: {}",
            paths.join(" ")
        )));
    }
    Ok(writes
        .into_iter()
        .map(|(destination, _, _)| destination)
        .collect())
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
