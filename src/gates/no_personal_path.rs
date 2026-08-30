//! Gate: a document carries no path into a person's home directory.
//!
//! An absolute home path is the author's machine leaking into a file every
//! reader receives: it resolves for one person and misleads everyone else.
//! The remedy is always available — write `~/`, `$HOME/`, or a bracketed
//! placeholder — so this reports the shape rather than guessing at intent.
//!
//! Two exemptions, both by purpose rather than by path. A file whose whole
//! job is one person's environment — `.env`, `.envrc.local`, and the
//! `.example` copies that show where those values go — is where a home path
//! belongs, so it is skipped by name. And a file git ignores never reaches
//! this gate at all, because pre-commit passes only what the repository
//! tracks.
//!
//! Code is not stripped before matching. A fenced example command carrying
//! a real home directory is exactly the leak this reports, so the rule's own
//! documents spell the forbidden shape with a placeholder segment instead.

use crate::domain::finding::Finding;
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation, read_text};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::DocumentCarriesNoPersonalPath];

/// The home-directory prefixes a path can start with, POSIX and Windows.
const ROOTS: &[&str] = &["/home/", "/Users/", "\\Users\\", "/Users\\", "\\Users/"];

/// Segments that stand for a person rather than naming one.
///
/// A document teaching the shape needs to write it, and these are the words
/// it writes. Anything opening with a placeholder marker — `<`, `$`, `{`,
/// `%` — is a stand-in too, and is judged by shape rather than by list.
const PLACEHOLDERS: &[&str] = &[
    "user",
    "username",
    "you",
    "youruser",
    "your-user",
    "me",
    "...",
];

/// Suffixes an environment file wears when it ships as a fill-in copy.
const SAMPLE_SUFFIXES: &[&str] = &[".example", ".sample", ".template", ".dist"];

/// Whether this file's purpose is one person's environment.
fn is_environment_file(path: &str) -> bool {
    let mut name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    loop {
        let trimmed = SAMPLE_SUFFIXES
            .iter()
            .find_map(|suffix| name.strip_suffix(suffix));
        match trimmed {
            Some(rest) => name = rest,
            None => break,
        }
    }
    name == ".env"
        || name.starts_with(".env.")
        || name == ".envrc"
        || name.starts_with(".envrc.")
        || name
            .split('.')
            .skip(1)
            .any(|part| part.eq_ignore_ascii_case("local"))
}

/// Whether the segment following a home root stands in for a person.
fn is_placeholder(segment: &str) -> bool {
    if segment.is_empty() {
        return true;
    }
    if segment.starts_with(['<', '$', '{', '%']) {
        return true;
    }
    let bare = segment.trim_matches(['<', '>', '{', '}', '%', '$']);
    PLACEHOLDERS
        .iter()
        .any(|placeholder| bare.eq_ignore_ascii_case(placeholder))
}

/// The user segment that follows a home root at `start`.
fn segment_after(line: &str, start: usize) -> &str {
    let rest = &line[start..];
    let end = rest
        .find([
            '/', '\\', ' ', '\t', '"', '\'', '`', ')', ']', ',', ';', ':',
        ])
        .unwrap_or(rest.len());
    &rest[..end]
}

/// Whether the line names a home directory belonging to a particular person.
///
/// One line is one finding however many paths it carries: the remedy is the
/// same for all of them, and a repeated report reads as repeated damage.
fn carries_personal_path(line: &str) -> bool {
    for root in ROOTS {
        let mut from = 0;
        while let Some(offset) = line[from..].find(root) {
            let start = from + offset + root.len();
            if !is_placeholder(segment_after(line, start)) {
                return true;
            }
            from = start;
        }
    }
    false
}

fn judge(file: &str, text: &str, violations: &mut Vec<Violation>) {
    for (index, raw) in text.lines().enumerate() {
        if carries_personal_path(raw) {
            violations.push(Violation::Finding(Finding::on_line(
                RuleId::DocumentCarriesNoPersonalPath,
                file,
                index + 1,
                raw.to_string(),
            )));
        }
    }
}

/// Judge every file pre-commit passed.
///
/// # Errors
///
/// [`crate::gates::GateError::Io`] when a file cannot be read.
pub fn run(ctx: &GateCtx, files: &[String]) -> GateResult {
    let mut violations = Vec::new();
    for file in files {
        if is_environment_file(file) {
            continue;
        }
        let text = read_text(ctx, file)?;
        judge(file, &text, &mut violations);
    }
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_on_named(name: &str, text: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(name), text).unwrap();
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[name.to_string()])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn run_on(text: &str) -> Vec<String> {
        run_on_named("doc.md", text)
    }

    /// A POSIX home directory belonging to someone, assembled rather than
    /// written. This gate scans every text file including its own source,
    /// and it strips no code, so a literal here would be a real finding.
    fn home(user: &str) -> String {
        format!("/home/{user}")
    }

    /// The macOS spelling of the same.
    fn mac_home(user: &str) -> String {
        format!("/Users/{user}")
    }

    #[test]
    fn accepts_a_home_relative_path() {
        assert!(run_on("Install into `~/.local/bin` or `$HOME/bin`.\n").is_empty());
    }

    #[test]
    fn rejects_an_absolute_home_path_naming_its_owner() {
        let path = home("ada");
        let out = run_on(&format!("Run it from {path}/projects/widget.\n"));
        assert_eq!(
            out,
            vec![format!(
                "FAIL docs-foundations:a-document-carries-no-personal-path doc.md:1: Run it from {path}/projects/widget."
            )]
        );
    }

    #[test]
    fn rejects_a_macos_home_path() {
        let text = format!("See {}/Library/logs.\n", mac_home("ada"));
        assert_eq!(run_on(&text).len(), 1);
    }

    #[test]
    fn a_placeholder_segment_is_the_documented_shape() {
        assert!(run_on("Write it as /home/<user>/notes or /Users/you/notes.\n").is_empty());
    }

    #[test]
    fn a_shell_variable_segment_is_a_placeholder() {
        assert!(run_on("Expands to /home/$USER/.config.\n").is_empty());
    }

    #[test]
    fn a_fenced_example_is_judged_like_prose() {
        let text = format!("Run:\n\n```bash\ncd {}/src\n```\n", home("ada"));
        let out = run_on(&text);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("doc.md:4"));
    }

    #[test]
    fn one_line_reports_once_however_many_paths_it_carries() {
        let text = format!("{} and {} both.\n", home("ada"), home("grace"));
        assert_eq!(run_on(&text).len(), 1);
    }

    #[test]
    fn an_environment_file_may_carry_a_real_path() {
        let key = format!("KEY={}/key.pem\n", home("ada"));
        for name in [".envrc.local", ".env.example", ".env"] {
            assert!(
                run_on_named(name, &key).is_empty(),
                "{name} is an environment file and may carry a real path"
            );
        }
    }

    #[test]
    fn an_ordinary_document_is_not_exempted_by_a_sample_suffix() {
        let text = format!("{}/x\n", home("ada"));
        assert_eq!(run_on_named("guide.md.example", &text).len(), 1);
    }

    #[test]
    fn a_document_named_for_a_locale_is_not_an_environment_file() {
        let text = format!("{}/x\n", home("ada"));
        assert_eq!(run_on_named("local.md", &text).len(), 1);
    }
}
