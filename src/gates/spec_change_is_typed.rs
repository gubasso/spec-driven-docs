//! Gate: a spec change an entry document cites carries a well-formed clause.
//!
//! The gate reads the plan zone the instance declared, so it names no path of
//! its own.
//!
//! What it judges is a clause line: a line that names an owning spec by path,
//! in inline code, which is the shape `09-spec-to-code.md` fixes. On such a
//! line every `ADDED`, `MODIFIED`, or `REMOVED` token must be followed by a
//! space and the rule ID in inline code. Judging is per occurrence rather
//! than per line, so a line carrying one good clause and one malformed clause
//! reports the malformed one.
//!
//! The line test is what keeps the gate off ordinary prose. A plan zone holds
//! whatever the planning tool writes there, and a sentence like "three
//! sections REMOVED this week" is narrative rather than a citation. Judging
//! every occurrence in every file would fail that sentence with no way to
//! write it.
//!
//! A declared zone the gate cannot read is a violation rather than a pass,
//! because a check over an empty set is a green light over nothing. That
//! covers an absent directory and an unreadable one, for both declarations,
//! and a recorded path that is not repository-relative. The variable is held
//! to no such shape: reaching records outside the checkout is what it is for.
//! Where the project declared no zone a command may read, the gate reports
//! nothing and `08-gates.md` carries the case as unenforced.

use camino::{Utf8Path, Utf8PathBuf};

use crate::domain::finding::Finding;
use crate::domain::manifest::{PLAN_ZONE_VAR, validate_plan_zone_path};
use crate::domain::rule_id::RuleId;
use crate::gates::paths::{PlanZoneTarget, plan_zone};
use crate::gates::{GateCtx, GateError, GateResult, Violation};

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

/// The content of the code span opening at the start of `rest`, and the byte
/// length of the whole span, both fences included.
///
/// A span opens and closes on runs of equal length, so the doubled form that
/// quotes a token containing a backtick is read the same way as the single
/// one, and a longer run does not close a shorter opener. Both the content
/// and the length are returned because a caller scanning a line has to
/// advance past the closing fence, not into it.
fn code_span(rest: &str) -> Option<(&str, usize)> {
    let run = rest.bytes().take_while(|byte| *byte == b'`').count();
    if run == 0 {
        return None;
    }
    let body = &rest[run..];
    let mut at = 0;
    while at < body.len() {
        let start = at + body[at..].find('`')?;
        let length = body[start..].bytes().take_while(|b| *b == b'`').count();
        if length == run {
            return Some((&body[..start], run + start + run));
        }
        at = start + length;
    }
    None
}

/// Whether the text after a type token is a well-formed rule-ID citation.
fn clause_is_well_formed(rest: &str) -> bool {
    let Some(rest) = rest.strip_prefix(' ') else {
        return false;
    };
    code_span(rest).is_some_and(|(id, _)| {
        id.trim()
            .split_once(':')
            .is_some_and(|(domain, rule)| is_slug(domain) && is_slug(rule))
    })
}

/// Whether a character can be part of a word, for the token-boundary test.
///
/// Alphanumeric by Unicode rather than by ASCII byte: a type word abutting an
/// accented letter is inside a word, exactly as one abutting `x` is, and a
/// byte test would call the first a standalone token.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether a line is a citation clause line.
///
/// The shape the chapter fixes puts each clause on the line that names its
/// owning spec, so an inline-code token ending in `.md` is what separates a
/// citation from prose that happens to use one of the three words.
fn is_clause_line(line: &str) -> bool {
    let mut rest = line;
    while let Some(at) = rest.find('`') {
        rest = &rest[at..];
        let Some((span, length)) = code_span(rest) else {
            return false;
        };
        // The suffix is compared case-sensitively on purpose: the corpus
        // convention is lowercase, and a `.MD` path is not the shape the
        // chapter fixes.
        // sdd: permanent the lowercase suffix is the shape this rule fixes
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        let names_a_spec = span.trim_end().ends_with(".md");
        if names_a_spec {
            return true;
        }
        // Past the closing fence. Every fence is ASCII, so the index is a
        // character boundary, and the length is at least two, so the scan
        // always advances.
        rest = &rest[length..];
    }
    false
}

/// Report every malformed clause in one file's text.
fn judge(file: &str, text: &str, violations: &mut Vec<Violation>) {
    for (index, line) in text.lines().enumerate() {
        if !is_clause_line(line) {
            continue;
        }
        for word in TYPES {
            for (at, _) in line.match_indices(word) {
                if line[..at].chars().next_back().is_some_and(is_word_char) {
                    continue;
                }
                let after = at + word.len();
                let rest = &line[after..];
                if rest.chars().next().is_some_and(is_word_char) {
                    continue;
                }
                if clause_is_well_formed(rest) {
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

/// Every file under the resolved zone, sorted.
///
/// Only `.git` is pruned. The wider repository prune list names vendored and
/// generated trees, and a plan zone is neither: its subdirectories belong to
/// the planning tool, so a zone partitioned into `dist/` or `target/` would
/// lose those clauses to a list that was never about it.
///
/// A walk error is raised rather than dropped. A directory the process cannot
/// read holds clauses this returns none of, and reporting that as an empty
/// zone would read as a clean review.
///
/// This is the second route a subject path takes into a gate, after
/// [`crate::gates::walk_files`], so the result passes through
/// [`GateCtx::subjects`]. A zone inside the repository is filtered on its
/// repository-relative form, which is the form a project's declaration
/// speaks. A zone outside the repository is not filtered, because no
/// repository-relative pattern can name it and the project declared the zone
/// itself.
fn documents(ctx: &GateCtx, root: &Utf8Path) -> Result<Vec<Utf8PathBuf>, GateError> {
    // Every filter is off, unlike the repository walk. A plan zone is
    // whatever the planning tool writes, and it is commonly git-ignored, so
    // honouring `.gitignore` here would report a populated zone as empty.
    let mut pruner = ignore::overrides::OverrideBuilder::new(root.as_std_path());
    let _ = pruner.add("!.git/**");
    let _ = pruner.add("!.git");
    let pruner = pruner
        .build()
        .unwrap_or_else(|_| ignore::overrides::Override::empty());

    let mut files = Vec::new();
    for entry in ignore::WalkBuilder::new(root.as_std_path())
        .standard_filters(false)
        .hidden(false)
        .overrides(pruner)
        .build()
    {
        let entry = entry
            .map_err(|source| GateError::io(root, std::io::Error::other(source.to_string())))?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        if let Ok(path) = Utf8PathBuf::from_path_buf(entry.into_path()) {
            files.push(path);
        }
    }
    files.sort();
    let relative: Vec<Utf8PathBuf> = files
        .iter()
        .filter_map(|path| path.strip_prefix(&ctx.repo_root).ok())
        .map(Utf8Path::to_path_buf)
        .collect();
    if relative.len() != files.len() {
        return Ok(files);
    }
    let kept = ctx.subjects(relative);
    Ok(kept
        .into_iter()
        .map(|path| ctx.repo_root.join(path))
        .collect())
}

/// Judge the declared plan zone.
///
/// # Errors
///
/// [`GateError::Io`] when a declared zone cannot be walked or one of its
/// files cannot be read. A file whose bytes are not UTF-8 is skipped instead:
/// a plan zone holds whatever the planning tool writes there, attachments
/// included.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    run_for(ctx, plan_zone(ctx))
}

/// Judge one resolved target. The resolution is the caller's, so every case
/// is reachable without setting an environment variable.
///
/// # Errors
///
/// See [`run`].
pub fn run_for(ctx: &GateCtx, target: PlanZoneTarget) -> GateResult {
    // The two declarations are not held to the same shape, and that is the
    // point of the variable. A recorded zone is repository-relative, so a
    // value that leaves the root is a broken record rather than a zone: it
    // reached here through a permissive read that skipped the argument-time
    // check. The variable is how a project reaches records outside the
    // checkout, so an absolute path there is the documented case.
    let (root, declared_by) = match target {
        PlanZoneTarget::Unchecked => return Ok(Vec::new()),
        PlanZoneTarget::Broken(reason) => return Ok(vec![Violation::Layout(reason)]),
        PlanZoneTarget::Variable(path) => (path, format!("{PLAN_ZONE_VAR} names")),
        PlanZoneTarget::Tracked(path) => {
            if let Err(error) = validate_plan_zone_path(&path) {
                return Ok(vec![Violation::Layout(format!(
                    "the recorded plan zone is {path}, which is not a zone a gate can read: {error}"
                ))]);
            }
            (path, "the recorded plan zone is".to_string())
        }
    };
    let full = ctx.path(&root);
    if !full.is_dir() {
        return Ok(vec![Violation::Layout(format!(
            "{declared_by} {root}, which is not a directory a gate can read"
        ))]);
    }
    let mut violations = Vec::new();
    for path in documents(ctx, &full)? {
        let text = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(source) => return Err(GateError::io(&path, source)),
        };
        let Ok(text) = String::from_utf8(text) else {
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

    const SPEC: &str = "- `_docs/specs/SPEC-auth.md` — ";
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

    /// Whether an unreadable directory really is unreadable here.
    ///
    /// Probed rather than guessed: a privileged runner reads mode 000, and
    /// a proxy such as "can this process list /root" answers yes on any
    /// image whose /root is world-readable, which silently retires the two
    /// tests below on the machines that most need them.
    fn mode_zero_blocks_reads() -> bool {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let locked = dir.path().join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let blocked = std::fs::read_dir(&locked).is_err();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        blocked
    }

    /// A code span holding a multi-byte character is scanned by boundary,
    /// never by an index into the middle of one.
    #[test]
    fn a_multi_byte_code_span_is_scanned_rather_than_sliced() {
        let dir = repository(
            "plan",
            "See `café` and `—` here. Three sections REMOVED this week.\n",
        );
        assert!(judge_zone(&dir, tracked("plan")).is_empty());
    }

    /// The scan advances past the closing fence, so a span before the spec
    /// path neither hides a malformed clause nor invents one.
    #[test]
    fn a_span_before_the_spec_path_does_not_desynchronize_the_scan() {
        let missed = repository(
            "plan",
            "- Renamed ``foo`` in `_docs/specs/SPEC-x.md` — ADDED bad\n",
        );
        let out = judge_zone(&missed, tracked("plan"));
        assert_eq!(
            out.len(),
            1,
            "a doubled span hid a malformed clause: {out:?}"
        );

        let invented = repository(
            "plan",
            "`x` and see foo.md `y` — three sections REMOVED this week\n",
        );
        assert!(
            judge_zone(&invented, tracked("plan")).is_empty(),
            "prose between two spans was read as a clause line"
        );
    }

    /// A longer run does not close a shorter opener, so the content a
    /// citation carries is the whole span.
    #[test]
    fn a_longer_backtick_run_does_not_close_a_shorter_opener() {
        let dir = repository("plan", &format!("{SPEC}ADDED `a:b``garbage`\n"));
        assert_eq!(judge_zone(&dir, tracked("plan")).len(), 1);
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

    /// The spec's own scenario: a typed clause whose ID token is not a
    /// `<spec-slug>:<rule-slug>` pair in inline code.
    #[test]
    fn a_clause_whose_id_token_is_garbled_fails() {
        for garbled in [
            "ADDED auth-token-expiry",
            "ADDED `auth-token-expiry`",
            "ADDED `Auth:Token`",
            "ADDED",
        ] {
            let dir = repository("plan", &format!("{SPEC}{garbled}\n"));
            assert_eq!(judge_zone(&dir, tracked("plan")).len(), 1, "{garbled}");
        }
    }

    /// Per occurrence, not per line: one good clause never covers a bad one.
    #[test]
    fn a_line_carrying_both_shapes_reports_the_malformed_one() {
        let dir = repository(
            "plan",
            &format!("{SPEC}ADDED `auth:a` and REMOVED auth:b\n"),
        );
        let out = judge_zone(&dir, tracked("plan"));
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].contains("REMOVED is not followed"));
    }

    /// Narrative in a plan document is not a citation. The zone holds
    /// whatever the planning tool writes there.
    #[test]
    fn prose_using_the_three_words_is_not_judged() {
        let dir = repository(
            "plan",
            "# Week 3\n\nThree sections REMOVED, one ADDED, the rest MODIFIED.\n\nSee `auth:x`.\n",
        );
        assert!(judge_zone(&dir, tracked("plan")).is_empty());
    }

    /// A type word inside a longer word is not the token, whether the
    /// neighbouring letter is ASCII or not.
    #[test]
    fn a_type_word_inside_another_word_is_not_a_clause() {
        let dir = repository(
            "plan",
            &format!("{SPEC}READDED and UNMODIFIED and ADDEDLY and ADDEDé and éADDED\n"),
        );
        assert!(judge_zone(&dir, tracked("plan")).is_empty());
    }

    /// A doubled code span is inline code too.
    #[test]
    fn a_doubled_code_span_is_a_well_formed_id() {
        let dir = repository("plan", &format!("{SPEC}ADDED ``auth:token-expiry``\n"));
        assert!(judge_zone(&dir, tracked("plan")).is_empty());
    }

    /// A nested document is judged too, and reported at its full path. The
    /// walk prunes nothing but `.git`, because a zone's subdirectories
    /// belong to the planning tool.
    #[test]
    fn the_walk_reaches_every_subdirectory_but_git() {
        let dir = repository("plan", GOOD);
        for sub in ["2026/q1", "dist", "target", "vendor"] {
            let nested = dir.path().join("plan").join(sub);
            std::fs::create_dir_all(&nested).unwrap();
            std::fs::write(nested.join("work.md"), BAD).unwrap();
        }
        let git = dir.path().join("plan/.git");
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(git.join("COMMIT_EDITMSG"), BAD).unwrap();

        let out = judge_zone(&dir, tracked("plan"));
        assert_eq!(out.len(), 4, "{out:?}");
        assert!(
            out.iter()
                .any(|line| line.contains("plan/2026/q1/work.md:1"))
        );
        assert!(
            out.iter()
                .any(|line| line.contains("plan/vendor/work.md:1"))
        );
        assert!(
            !out.iter().any(|line| line.contains(".git")),
            "the walk judged the zone's own git directory"
        );
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

    /// A recorded zone that leaves the repository is refused rather than
    /// walked: the record reaches the gate through a permissive read that
    /// skipped the argument-time check.
    #[test]
    fn a_recorded_zone_outside_the_repository_is_refused() {
        let dir = repository("plan", BAD);
        // `.` is the case a first-character test misses: it survives an
        // is-absolute check and would walk the whole repository.
        for escape in ["/etc", "../elsewhere", ".", "  "] {
            let out = judge_zone(&dir, tracked(escape));
            assert_eq!(out.len(), 1, "{escape}: {out:?}");
            assert!(out[0].contains("not a zone a gate can read"), "{escape}");
        }
    }

    /// The variable is how a project reaches records outside the checkout,
    /// so an absolute path there is the documented case rather than a
    /// broken one.
    #[test]
    fn the_variable_may_name_a_zone_outside_the_repository() {
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("work.md"), BAD).unwrap();
        let dir = repository("", "");
        let named = Utf8PathBuf::from(outside.path().to_str().unwrap());
        let out = judge_zone(&dir, PlanZoneTarget::Variable(named));
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].contains("work.md:1"), "{out:?}");
    }

    /// A record declaring a gated zone it cannot resolve fails rather than
    /// skipping, which is the state `07-lifecycle.md` forbids.
    #[test]
    fn a_broken_declaration_is_a_layout_violation() {
        let dir = repository("plan", BAD);
        let out = judge_zone(
            &dir,
            PlanZoneTarget::Broken(
                "the recorded plan zone is tracked and its path is empty".into(),
            ),
        );
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].contains("its path is empty"));
    }

    /// Nothing a command may check means nothing reported, even over a
    /// directory that does hold a malformed clause.
    #[test]
    fn an_unchecked_zone_reports_nothing() {
        let dir = repository("plan", BAD);
        assert!(judge_zone(&dir, PlanZoneTarget::Unchecked).is_empty());
    }

    /// Bytes that are not UTF-8 are an attachment and are skipped; a file
    /// the process cannot read is raised, because a zone this gate could not
    /// read must not report as clean.
    #[test]
    fn an_attachment_is_skipped_and_an_unreadable_file_is_raised() {
        use std::os::unix::fs::PermissionsExt;
        let dir = repository("plan", GOOD);
        std::fs::write(dir.path().join("plan/attachment.bin"), [0xff_u8, 0xfe]).unwrap();
        assert!(judge_zone(&dir, tracked("plan")).is_empty());

        if !mode_zero_blocks_reads() {
            // A privileged runner reads mode 000, so the case does not exist.
            return;
        }
        let locked = dir.path().join("plan/locked.md");
        std::fs::write(&locked, BAD).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        let raised = run_for(&ctx, tracked("plan")).is_err();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(raised, "an unreadable file read as no clause");
    }

    /// An unreadable subdirectory is raised for the same reason.
    #[test]
    fn an_unreadable_subdirectory_is_raised() {
        use std::os::unix::fs::PermissionsExt;

        if !mode_zero_blocks_reads() {
            return;
        }
        let dir = repository("plan", GOOD);
        let sub = dir.path().join("plan/sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("entry.md"), BAD).unwrap();
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o000)).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        let raised = run_for(&ctx, tracked("plan")).is_err();
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(raised, "an unreadable subdirectory read as an empty one");
    }
}
