//! Gate: a spec change an entry document cites carries a well-formed clause.
//!
//! The gate reads the plan zone the instance declared, so it names no path of
//! its own. Every `ADDED`, `MODIFIED`, or `REMOVED` token there must be
//! followed by a space and the rule ID in inline code. Judging is per
//! occurrence rather than per line: a line carrying one good clause and one
//! malformed clause reports the malformed one.
//!
//! A declared zone whose directory is absent is a violation rather than a
//! pass, because a check over an empty set is a green light over nothing.
//! Where the project declared no zone a command may read, the gate reports
//! nothing and `08-gates.md` carries the case in its unenforced list.

use camino::Utf8PathBuf;

use crate::domain::finding::Finding;
use crate::domain::manifest::PLAN_ZONE_VAR;
use crate::domain::rule_id::RuleId;
use crate::gates::paths::{PlanZoneTarget, plan_zone};
use crate::gates::{GateCtx, GateResult, PRUNED_DIRS, Violation};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::SpecChangeIsTyped];

/// The three change types an entry document declares.
const TYPES: &[&str] = &["ADDED", "MODIFIED", "REMOVED"];

/// Whether a half of a rule ID is a slug.
fn is_slug(part: &str) -> bool {
    !part.is_empty()
        && part
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Whether the text after a type token is ` `<domain>:<rule>` `.
fn clause_is_well_formed(rest: &str) -> bool {
    let Some(rest) = rest.strip_prefix(' ') else {
        return false;
    };
    let Some(rest) = rest.strip_prefix('`') else {
        return false;
    };
    let Some(end) = rest.find('`') else {
        return false;
    };
    rest[..end]
        .split_once(':')
        .is_some_and(|(domain, rule)| is_slug(domain) && is_slug(rule))
}

/// Whether a byte can be part of a word, for the token-boundary test.
const fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Report every malformed clause in one file's text.
fn judge(file: &str, text: &str, violations: &mut Vec<Violation>) {
    for (index, line) in text.lines().enumerate() {
        for word in TYPES {
            for (at, _) in line.match_indices(word) {
                let bytes = line.as_bytes();
                if at > 0 && is_word_byte(bytes[at - 1]) {
                    continue;
                }
                let after = at + word.len();
                if bytes.get(after).copied().is_some_and(is_word_byte) {
                    continue;
                }
                if clause_is_well_formed(&line[after..]) {
                    continue;
                }
                violations.push(Violation::Finding(Finding::on_line(
                    RuleId::SpecChangeIsTyped,
                    file,
                    index + 1,
                    format!("{word} is not followed by an inline-code rule ID: {line}"),
                )));
            }
        }
    }
}

/// Every readable file under the resolved zone, pruning what no author owns.
fn documents(root: &camino::Utf8Path) -> Vec<Utf8PathBuf> {
    let mut files: Vec<Utf8PathBuf> = walkdir::WalkDir::new(root.as_std_path())
        .into_iter()
        .filter_entry(|entry| {
            !(entry.file_type().is_dir()
                && entry.depth() > 0
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| PRUNED_DIRS.contains(&name)))
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| Utf8PathBuf::from_path_buf(entry.into_path()).ok())
        .collect();
    files.sort();
    files
}

/// Judge the declared plan zone.
///
/// # Errors
///
/// This gate reads what it can and raises nothing: an unreadable file inside
/// the zone is skipped, because a plan zone can hold whatever the planning
/// tool writes there, binary attachments included.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    run_for(ctx, plan_zone(ctx))
}

/// Judge one resolved target. The resolution is the caller's, so every case
/// is reachable without setting an environment variable.
///
/// # Errors
///
/// None; the signature matches every other gate's.
pub fn run_for(ctx: &GateCtx, target: PlanZoneTarget) -> GateResult {
    let (root, declared_by) = match target {
        PlanZoneTarget::Unchecked => return Ok(Vec::new()),
        PlanZoneTarget::Variable(path) => (path, format!("{PLAN_ZONE_VAR} names")),
        PlanZoneTarget::Tracked(path) => (path, "the recorded plan zone is".to_string()),
    };
    let full = ctx.path(&root);
    if !full.is_dir() {
        return Ok(vec![Violation::Layout(format!(
            "{declared_by} {root}, which is not a directory a gate can read"
        ))]);
    }
    let mut violations = Vec::new();
    for path in documents(&full) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let shown = path.strip_prefix(&full).map_or_else(
            |_| path.clone(),
            |rest| {
                let mut shown = root.clone();
                shown.push(rest);
                shown
            },
        );
        judge(shown.as_str(), &text, &mut violations);
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "- `_docs/specs/SPEC-auth.md` — ADDED `auth:token-expiry-is-bounded`\n";
    const BAD: &str = "- `_docs/specs/SPEC-auth.md` — ADDED auth:token-expiry-is-bounded\n";

    /// A repository whose `zone` directory holds one entry document.
    fn repository(zone: &str, document: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        if !zone.is_empty() {
            let path = dir.path().join(zone);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join("entry.md"), document).unwrap();
        }
        dir
    }

    fn judge_zone(dir: &tempfile::TempDir, target: PlanZoneTarget) -> Vec<String> {
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run_for(&ctx, target)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn tracked(path: &str) -> PlanZoneTarget {
        PlanZoneTarget::Tracked(Utf8PathBuf::from(path))
    }

    #[test]
    fn a_tracked_zone_accepts_a_well_formed_clause() {
        let dir = repository("plan", GOOD);
        assert!(judge_zone(&dir, tracked("plan")).is_empty());
    }

    #[test]
    fn a_tracked_zone_reports_a_malformed_clause_with_its_line() {
        let dir = repository("plan", BAD);
        let out = judge_zone(&dir, tracked("plan"));
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].starts_with("FAIL spec-to-code:a-spec-change-is-typed plan/entry.md:1:"));
    }

    /// Per occurrence, not per line: one good clause never covers a bad one.
    #[test]
    fn a_line_carrying_both_shapes_reports_the_malformed_one() {
        let dir = repository("plan", "ADDED `auth:a` and REMOVED auth:b\n");
        let out = judge_zone(&dir, tracked("plan"));
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].contains("REMOVED is not followed"));
    }

    /// A type word inside a longer word is not the token.
    #[test]
    fn a_type_word_inside_another_word_is_not_a_clause() {
        let dir = repository("plan", "READDED and UNMODIFIED and ADDEDLY\n");
        assert!(judge_zone(&dir, tracked("plan")).is_empty());
    }

    /// A nested document is judged too, and reported at its full path.
    #[test]
    fn the_walk_reaches_a_nested_document() {
        let dir = repository("plan", GOOD);
        let nested = dir.path().join("plan/2026/q1");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("work.md"), BAD).unwrap();
        let out = judge_zone(&dir, tracked("plan"));
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].contains("plan/2026/q1/work.md:1"));
    }

    /// A declared zone whose directory is gone is drift, never a pass.
    #[test]
    fn an_absent_directory_is_a_layout_violation_naming_its_declaration() {
        let dir = repository("", "");
        let recorded = judge_zone(&dir, tracked("plan"));
        assert_eq!(recorded.len(), 1, "{recorded:?}");
        assert!(recorded[0].contains("the recorded plan zone is plan"));

        let named = judge_zone(&dir, PlanZoneTarget::Variable(Utf8PathBuf::from("gone")));
        assert_eq!(named.len(), 1, "{named:?}");
        assert!(named[0].contains("SDD_PLAN_ZONE names gone"));
    }

    /// Nothing a command may check means nothing reported, even over a
    /// directory that does hold a malformed clause.
    #[test]
    fn an_unchecked_zone_reports_nothing() {
        let dir = repository("plan", BAD);
        assert!(judge_zone(&dir, PlanZoneTarget::Unchecked).is_empty());
    }

    /// A file the walk cannot decode is skipped rather than raised: a plan
    /// zone holds whatever the planning tool writes there.
    #[test]
    fn an_undecodable_file_is_skipped() {
        let dir = repository("plan", GOOD);
        std::fs::write(dir.path().join("plan/attachment.bin"), [0xff_u8, 0xfe]).unwrap();
        assert!(judge_zone(&dir, tracked("plan")).is_empty());
    }
}
