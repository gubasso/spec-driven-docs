//! The shell environment a project's gates read, reported and written.
//!
//! Two items exist: the docs scratch variable, which overrides the location
//! the instance records, and the ignore entry that keeps a docs scratch
//! inside the repository out of version control. The report reads which
//! shell loader the repository records and whether each item is in place.
//! An apply writes one item through that loader, and nothing else.
//!
//! The loader is read from the repository's own files, never from the host:
//! `.envrc` means direnv, `.env` means a dotenv loader, and neither means
//! nothing loads a variable here. With direnv, the per-developer value goes
//! to `.envrc.local`, which the repository ignores and `.envrc` sources.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::domain::manifest::DOCS_SCRATCH_VAR;
use crate::error::AppError;

/// The machine schema `sdd env --json` declares.
pub const SCHEMA: &str = "sdd.env/1";

/// The direnv loader file.
pub const ENVRC: &str = ".envrc";

/// The per-developer file `.envrc` sources.
pub const ENVRC_LOCAL: &str = ".envrc.local";

/// The dotenv loader file.
pub const DOTENV: &str = ".env";

/// The repository's ignore file.
pub const GITIGNORE: &str = ".gitignore";

/// The line `.envrc` carries to load the per-developer file.
pub const SOURCE_LOCAL: &str = "source_env_if_exists .envrc.local";

/// Which shell loader the repository records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Loader {
    /// The repository carries `.envrc`.
    Direnv,
    /// The repository carries `.env` and no `.envrc`.
    Dotenv,
    /// The repository carries neither.
    None,
}

/// One environment item this verb reports and writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Item {
    /// The docs scratch variable, through the recorded loader.
    DocsScratchVariable,
    /// The ignore entry for a docs scratch inside the repository.
    DocsScratchIgnore,
}

impl Item {
    /// The word the command line and the JSON spell.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DocsScratchVariable => "docs-scratch-variable",
            Self::DocsScratchIgnore => "docs-scratch-ignore",
        }
    }
}

/// Where one item stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemState {
    /// The item is in place.
    Wired,
    /// The item can be written and is not there yet.
    Unwired,
    /// No loader is recorded, so nothing loads the variable.
    NoLoader,
    /// The docs scratch sits outside the repository, so no entry applies.
    Outside,
    /// No docs scratch is declared, so there is nothing to ignore.
    Undeclared,
}

/// One item's row.
#[derive(Debug, Clone, Serialize)]
pub struct ItemReport {
    /// Which item.
    pub item: Item,
    /// Where it stands.
    pub state: ItemState,
    /// The file the item lives in, where one applies.
    pub file: Option<Utf8PathBuf>,
    /// The value the item names: the variable's value as the file or the
    /// shell holds it, or the scratch path the entry covers.
    pub value: Option<String>,
}

/// The whole report.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The machine schema of this object.
    pub schema: &'static str,
    /// The repository read.
    pub target: Utf8PathBuf,
    /// The loader the repository records.
    pub loader: Loader,
    /// Whether this shell carries the variable now.
    pub variable_in_shell: Option<String>,
    /// One row per item.
    pub items: Vec<ItemReport>,
    /// Every line an apply wrote, as `file: line`.
    pub wrote: Vec<String>,
}

/// The loader the repository records.
fn loader(target: &Utf8Path) -> Loader {
    if target.join(ENVRC).is_file() {
        Loader::Direnv
    } else if target.join(DOTENV).is_file() {
        Loader::Dotenv
    } else {
        Loader::None
    }
}

/// The file the variable's line goes to under each loader.
const fn variable_file(loader: Loader) -> Option<&'static str> {
    match loader {
        Loader::Direnv => Some(ENVRC_LOCAL),
        Loader::Dotenv => Some(DOTENV),
        Loader::None => None,
    }
}

/// The assignment one loader file carries.
fn assignment(loader: Loader, value: &str) -> String {
    let quoted = format!("'{}'", value.replace('\'', r"'\''"));
    match loader {
        Loader::Direnv => format!("export {DOCS_SCRATCH_VAR}={quoted}"),
        Loader::Dotenv | Loader::None => format!("{DOCS_SCRATCH_VAR}={quoted}"),
    }
}

/// Whether a line assigns the variable, in either loader's spelling.
fn assigns(line: &str) -> bool {
    let line = line.trim_start();
    let line = line.strip_prefix("export ").unwrap_or(line);
    line.starts_with(&format!("{DOCS_SCRATCH_VAR}="))
}

/// The words of the first command on a shell line, with quotes removed.
///
/// A quoted word stays one word, whatever it holds. Backslashes follow
/// Bash: outside quotes one escapes any character, inside double quotes it
/// escapes only `$`, a backtick, `"`, `\`, or a newline and stays literal
/// before anything else, and inside single quotes it is literal. An
/// unquoted `;`, `&`, or `|` ends the command, and so does a `#` that opens
/// a word.
fn command_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                let held = word.get_or_insert_with(String::new);
                held.extend(chars.by_ref().take_while(|&quoted| quoted != '\''));
            }
            '"' => {
                let held = word.get_or_insert_with(String::new);
                while let Some(quoted) = chars.next() {
                    match quoted {
                        '"' => break,
                        '\\' => match chars.peek() {
                            Some('$' | '`' | '"' | '\\' | '\n') => held.extend(chars.next()),
                            _ => held.push('\\'),
                        },
                        _ => held.push(quoted),
                    }
                }
            }
            '\\' => word.get_or_insert_with(String::new).extend(chars.next()),
            ';' | '&' | '|' => break,
            '#' if word.is_none() => break,
            _ if c.is_whitespace() => words.extend(word.take()),
            _ => word.get_or_insert_with(String::new).push(c),
        }
    }
    words.extend(word);
    words
}

/// Whether an `.envrc` line loads the per-developer file.
///
/// Only a command that reads the file counts, and only when the file is the
/// operand it reads. A line that merely names it, such as
/// `watch_file .envrc.local` or a trailing comment, loads nothing.
fn sources_local(line: &str) -> bool {
    let words = command_words(line);
    matches!(
        words.first().map(String::as_str),
        Some("source_env" | "source_env_if_exists" | "source" | ".")
    ) && words
        .get(1)
        .is_some_and(|operand| operand.trim_start_matches("./") == ENVRC_LOCAL)
}

/// Whether the repository's `.envrc` loads the per-developer file.
fn envrc_sources_local(target: &Utf8Path) -> bool {
    std::fs::read_to_string(target.join(ENVRC)).is_ok_and(|text| text.lines().any(sources_local))
}

/// The value one loader file assigns, unquoted, where it assigns one.
fn assigned(text: &str) -> Option<String> {
    text.lines().rfind(|line| assigns(line)).map(|line| {
        let (_, value) = line.split_once('=').unwrap_or_default();
        value
            .trim()
            .trim_matches('\'')
            .trim_matches('"')
            .to_string()
    })
}

/// The docs scratch in effect: the variable, then the record.
fn scratch(target: &Utf8Path) -> Option<Utf8PathBuf> {
    crate::gates::paths::docs_scratch_variable().or_else(|| {
        crate::services::installer::recorded_field(target, "docs_scratch")
            .and_then(|value| value.as_str().map(Utf8PathBuf::from))
            .filter(|path| !path.as_str().is_empty())
    })
}

/// The scratch path relative to the target, where it sits inside it.
fn inside(target: &Utf8Path, scratch: &Utf8Path) -> Option<Utf8PathBuf> {
    let relative = if scratch.is_absolute() {
        scratch.strip_prefix(target).ok()?.to_owned()
    } else {
        scratch.to_owned()
    };
    let escapes = relative
        .components()
        .any(|part| matches!(part, camino::Utf8Component::ParentDir));
    (!escapes && !relative.as_str().is_empty()).then(|| {
        Utf8PathBuf::from(
            relative
                .as_str()
                .trim_start_matches("./")
                .trim_end_matches('/'),
        )
    })
}

/// The ignore-file lines that name exactly one path.
fn ignore_forms(path: &Utf8Path) -> [String; 4] {
    [
        path.to_string(),
        format!("/{path}"),
        format!("{path}/"),
        format!("/{path}/"),
    ]
}

/// Whether the repository ignores `path`.
fn covered(target: &Utf8Path, path: &Utf8Path) -> bool {
    // The scratch is a directory, and a directory-only pattern such as
    // `.docs-scratch/` matches a path Git has never seen only when the path
    // says it is one.
    let directory = Utf8PathBuf::from(format!("{path}/"));
    if let Some(answer) = crate::adapters::history::ignores(target, &directory) {
        return answer;
    }
    let forms = ignore_forms(path);
    std::fs::read_to_string(target.join(GITIGNORE)).is_ok_and(|text| {
        text.lines()
            .any(|line| forms.iter().any(|form| line.trim() == form))
    })
}

/// The ignore entry's row for a declared docs scratch.
fn ignore_row(target: &Utf8Path, path: &Utf8Path) -> ItemReport {
    inside(target, path).map_or_else(
        || ItemReport {
            item: Item::DocsScratchIgnore,
            state: ItemState::Outside,
            file: None,
            value: Some(path.to_string()),
        },
        |relative| ItemReport {
            item: Item::DocsScratchIgnore,
            state: if covered(target, &relative) {
                ItemState::Wired
            } else {
                ItemState::Unwired
            },
            file: Some(Utf8PathBuf::from(GITIGNORE)),
            value: Some(relative.to_string()),
        },
    )
}

/// Report the environment items of `target`.
///
/// SATISFIES setup:env-reports-the-loader-and-each-item
#[must_use]
pub fn report(target: &Utf8Path) -> Report {
    let loader = loader(target);
    let file = variable_file(loader);
    let held = file.and_then(|file| std::fs::read_to_string(target.join(file)).ok());
    let value = held.as_deref().and_then(assigned);
    let variable = ItemReport {
        item: Item::DocsScratchVariable,
        state: match (loader, &value) {
            (Loader::None, _) => ItemState::NoLoader,
            (Loader::Direnv, Some(_)) if !envrc_sources_local(target) => ItemState::Unwired,
            (_, Some(_)) => ItemState::Wired,
            (_, None) => ItemState::Unwired,
        },
        file: file.map(Utf8PathBuf::from),
        value,
    };
    let ignore = scratch(target).as_deref().map_or(
        ItemReport {
            item: Item::DocsScratchIgnore,
            state: ItemState::Undeclared,
            file: None,
            value: None,
        },
        |path| ignore_row(target, path),
    );
    Report {
        schema: SCHEMA,
        target: target.to_owned(),
        loader,
        variable_in_shell: crate::domain::paths::variable(DOCS_SCRATCH_VAR),
        items: vec![variable, ignore],
        wrote: Vec::new(),
    }
}

/// Append `line` to the file at `path` unless a line already satisfies
/// `held`, keeping every byte already there.
fn ensure_line(
    target: &Utf8Path,
    file: &str,
    line: &str,
    held: impl Fn(&str) -> bool,
    wrote: &mut Vec<String>,
) -> Result<(), AppError> {
    let path = target.join(file);
    if path.is_symlink() {
        return Err(AppError::Refused(format!(
            "{file} is a symlink; refusing to write through it"
        )));
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(source) => return Err(AppError::Io(source)),
    };
    if text.lines().any(&held) {
        return Ok(());
    }
    let mut next = text;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(line);
    next.push('\n');
    crate::adapters::fs::write_atomic(&path, next.as_bytes())?;
    wrote.push(format!("{file}: {line}"));
    Ok(())
}

/// Set the variable's line in one file, replacing an earlier assignment.
fn set_assignment(
    target: &Utf8Path,
    file: &str,
    line: &str,
    wrote: &mut Vec<String>,
) -> Result<(), AppError> {
    let path = target.join(file);
    if path.is_symlink() {
        return Err(AppError::Refused(format!(
            "{file} is a symlink; refusing to write through it"
        )));
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(source) => return Err(AppError::Io(source)),
    };
    if text.lines().any(|held| held == line) {
        return Ok(());
    }
    let mut next = String::new();
    let mut replaced = false;
    for held in text.lines() {
        if assigns(held) && !replaced {
            next.push_str(line);
            replaced = true;
        } else if assigns(held) {
            continue;
        } else {
            next.push_str(held);
        }
        next.push('\n');
    }
    if !replaced {
        next.push_str(line);
        next.push('\n');
    }
    crate::adapters::fs::write_atomic(&path, next.as_bytes())?;
    wrote.push(format!("{file}: {line}"));
    Ok(())
}

/// Write one item through the loader the repository records.
///
/// SATISFIES setup:env-writes-one-item-through-the-recorded-loader
///
/// # Errors
///
/// [`AppError::Usage`] where the variable item names no value, and
/// [`AppError::Refused`] where no loader is recorded, where the scratch sits
/// outside the repository or is not declared, or where a file to write is a
/// symlink.
pub fn apply(target: &Utf8Path, item: Item, value: Option<&str>) -> Result<Report, AppError> {
    let mut wrote = Vec::new();
    match item {
        Item::DocsScratchVariable => {
            let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
                return Err(AppError::Usage(format!(
                    "--apply {} needs --value <PATH>: the location this host keeps the docs scratch at",
                    item.as_str()
                )));
            };
            let loader = loader(target);
            let Some(file) = variable_file(loader) else {
                return Err(AppError::Refused(format!(
                    "{target} records no shell loader, neither {ENVRC} nor {DOTENV}, so nothing would load the variable; set `{}` in the shell that runs the gates",
                    assignment(Loader::None, value)
                )));
            };
            set_assignment(target, file, &assignment(loader, value), &mut wrote)?;
            if loader == Loader::Direnv {
                ensure_line(target, ENVRC, SOURCE_LOCAL, sources_local, &mut wrote)?;
            }
            let forms = ignore_forms(Utf8Path::new(file));
            ensure_line(
                target,
                GITIGNORE,
                file,
                |line| forms.iter().any(|form| line.trim() == form),
                &mut wrote,
            )?;
        }
        Item::DocsScratchIgnore => {
            let Some(scratch) = scratch(target) else {
                return Err(AppError::Refused(format!(
                    "{target} declares no docs scratch, so there is nothing to ignore"
                )));
            };
            let Some(relative) = inside(target, &scratch) else {
                return Err(AppError::Refused(format!(
                    "the docs scratch {scratch} sits outside {target}, so no ignore entry applies"
                )));
            };
            if !covered(target, &relative) {
                let forms = ignore_forms(&relative);
                ensure_line(
                    target,
                    GITIGNORE,
                    &format!("/{relative}/"),
                    |line| forms.iter().any(|form| line.trim() == form),
                    &mut wrote,
                )?;
            }
        }
    }
    let mut report = report(target);
    report.wrote = wrote;
    Ok(report)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    #[test]
    fn an_assignment_reads_back_in_either_spelling() {
        assert_eq!(
            assigned("export SDD_DOCS_SCRATCH='../notes'\n").as_deref(),
            Some("../notes")
        );
        assert_eq!(assigned("SDD_DOCS_SCRATCH=\"x\"\n").as_deref(), Some("x"));
        assert_eq!(assigned("OTHER=1\n"), None);
    }

    #[test]
    fn only_a_command_that_reads_the_local_file_loads_it() {
        assert!(sources_local("source_env_if_exists .envrc.local"));
        assert!(sources_local("  source_env ./.envrc.local"));
        assert!(sources_local("source \".envrc.local\""));
        assert!(!sources_local("watch_file .envrc.local"));
        assert!(!sources_local("# source_env_if_exists .envrc.local"));
        assert!(!sources_local("source_env_if_exists .envrc.other"));
        assert!(!sources_local(
            "source_env_if_exists .envrc.private # .envrc.local"
        ));
        assert!(!sources_local(
            "source_env_if_exists \".envrc.local backup\""
        ));
        assert!(sources_local("source_env_if_exists .envrc.local;"));
        assert!(sources_local(
            "source_env_if_exists .envrc.local && echo loaded"
        ));
        assert!(sources_local("source_env_if_exists '.envrc.local'||true"));
        assert!(!sources_local(r#"source_env_if_exists "\.envrc.local""#));
        assert!(sources_local(r"source_env_if_exists \.envrc.local"));
        assert!(!sources_local(r"source_env_if_exists '\.envrc.local'"));
        assert_eq!(command_words(r#"a "x\"y\\z\q""#), ["a", r#"x"y\z\q"#]);
    }

    #[test]
    fn a_value_with_a_quote_stays_one_shell_word() {
        assert_eq!(
            assignment(Loader::Direnv, "it's"),
            r"export SDD_DOCS_SCRATCH='it'\''s'"
        );
    }

    #[test]
    fn a_scratch_inside_the_target_is_relative_and_one_outside_is_none() {
        let target = Utf8Path::new("/work/project");
        assert_eq!(
            inside(target, Utf8Path::new(".docs-scratch/")),
            Some(Utf8PathBuf::from(".docs-scratch"))
        );
        assert_eq!(
            inside(target, Utf8Path::new("/work/project/notes")),
            Some(Utf8PathBuf::from("notes"))
        );
        assert_eq!(inside(target, Utf8Path::new("../beside")), None);
        assert_eq!(inside(target, Utf8Path::new("/elsewhere")), None);
    }
}
