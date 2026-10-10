//! What the Markdown linter will read here, reported, and one proof that its
//! link rule runs.
//!
//! The report reads the discovery locations the status report names for the
//! active profile, or for the candidate profile before a landing, and turns
//! each configuration that breaks the delivered shapes, each link rule that
//! is not loaded or not enabled, and each reserved Markdown path the linter
//! still reads into one finding with the rule it breaks and the kind of edit
//! that resolves it. A configuration whose grammar this binary does not read,
//! a JavaScript module, is reported as `unknown` rather than guessed at.
//!
//! The proof runs the delivered Markdown hook over a scratch file at the
//! repository root, first with a broken relative link and then a valid one,
//! and removes the file. A broken link that lints clean is a failed proof,
//! because it means the rule silently does not run.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::domain::markdownlint::{CLI2_NAMES, LIBRARY_NAMES, RELATIVE_LINKS_PACKAGE};
use crate::domain::profile::ProfileId;
use crate::domain::rule_id::RuleId;
use crate::error::AppError;

/// The machine schema `sdd lint --json` declares.
pub const SCHEMA: &str = "sdd.lint/1";

/// The machine schema `sdd lint prove --json` declares.
pub const PROVE_SCHEMA: &str = "sdd.lint-prove/1";

/// The rule the link findings and the proof cite.
const LINKS: RuleId = RuleId::RootLintConfigurationLoadsRelativeLinks;

/// The rule the placement findings cite.
const COMPOSES: RuleId = RuleId::LintConfigurationComposes;

/// The rule the reserved-path findings cite.
const DECLARES: RuleId = RuleId::ProjectDeclaresWhatItsGatesJudge;

/// The rule name markdownlint reports a broken relative link under.
const LINK_RULE: &str = "relative-links";

/// The edit one finding needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Edit {
    /// Fold the configuration into the repository root's CLI2 file.
    FoldIntoRoot,
    /// Replace a linked directory with the directory itself.
    ReplaceLink,
    /// Add the link rule's package to `customRules`.
    LoadRelativeLinks,
    /// Set `"relative-links": true` in the effective rules.
    EnableRelativeLinks,
    /// Add a CLI2 file at the root beside a library-family file, to load the
    /// rule.
    AddCompanionCli2,
    /// Add a reserved Markdown path to the root CLI2 file's `ignores`.
    AddIgnore,
    /// The binary cannot read this configuration; the agent inspects it.
    Unknown,
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// The path the edit lands on, relative to the target, or the reserved
    /// pattern.
    pub path: String,
    /// The rule the finding breaks.
    pub rule: &'static str,
    /// The edit that resolves it.
    pub edit: Edit,
}

/// Which profile the report read, and what decided it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProfileSource {
    /// The instance record.
    Recorded,
    /// The `--profile` flag, before a landing.
    Flag,
    /// The codebase default, before a landing.
    Default,
}

/// The whole report.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// The machine schema of this object.
    pub schema: &'static str,
    /// The repository read.
    pub target: Utf8PathBuf,
    /// The profile whose discovery locations were read.
    pub profile: ProfileId,
    /// What decided the profile.
    pub profile_source: ProfileSource,
    /// Every finding, in the order the report reads the locations.
    pub findings: Vec<Finding>,
}

fn finding(path: impl Into<String>, rule: RuleId, edit: Edit) -> Finding {
    Finding {
        path: path.into(),
        rule: rule.as_str(),
        edit,
    }
}

/// The profile a report reads: the recorded one wherever an instance is.
fn profile(
    target: &Utf8Path,
    flag: Option<ProfileId>,
) -> Result<(ProfileId, ProfileSource), AppError> {
    let recorded =
        crate::services::installer::recorded_field(target, "profile").and_then(|value| {
            ProfileId::every().find(|profile| Some(profile.as_str()) == value.as_str())
        });
    match (recorded, flag) {
        (Some(recorded), Some(asked)) if recorded != asked => Err(AppError::Refused(format!(
            "{target} records the {recorded} profile and this report asks for {asked}"
        ))),
        (Some(recorded), _) => Ok((recorded, ProfileSource::Recorded)),
        (None, Some(asked)) => Ok((asked, ProfileSource::Flag)),
        (None, None) => Ok((ProfileId::Codebase, ProfileSource::Default)),
    }
}

/// JSON with comments and trailing commas, as markdownlint-cli2 reads it.
fn jsonc(text: &str) -> Option<serde_json::Value> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for skipped in chars.by_ref() {
                    if last == '*' && skipped == '/' {
                        break;
                    }
                    last = skipped;
                }
            }
            _ => out.push(c),
        }
    }
    let trailing = regex::Regex::new(r",(\s*[}\]])").ok()?;
    serde_json::from_str(&trailing.replace_all(&out, "$1")).ok()
}

/// One configuration file's contents, where this binary reads its grammar.
fn read_configuration(path: &Utf8Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    match path.extension() {
        Some("jsonc" | "json") => jsonc(&text),
        Some("yaml" | "yml") => yaml_serde::from_str(&text).ok(),
        _ => None,
    }
}

/// Whether a rules object turns the link rule on.
fn enables(rules: Option<&serde_json::Value>) -> bool {
    match rules.and_then(|rules| rules.get(LINK_RULE)) {
        Some(serde_json::Value::Bool(on)) => *on,
        Some(serde_json::Value::Object(_)) => true,
        _ => false,
    }
}

/// The findings the repository root's configuration raises.
///
/// SATISFIES instance:the-root-lint-configuration-loads-relative-links
fn root_findings(target: &Utf8Path, landed: bool, findings: &mut Vec<Finding>) {
    let present = |name: &str| std::fs::symlink_metadata(target.join(name)).is_ok();
    let cli2: Vec<&str> = CLI2_NAMES
        .into_iter()
        .filter(|name| present(name))
        .collect();
    let library: Vec<&str> = LIBRARY_NAMES
        .into_iter()
        .filter(|name| present(name))
        .collect();
    if cli2.is_empty() && library.is_empty() {
        // Before a landing the root seed lands here and loads the rule.
        if landed {
            findings.push(finding(CLI2_NAMES[0], LINKS, Edit::LoadRelativeLinks));
        }
        return;
    }
    // The linter reads the first CLI2 file by precedence and ignores the
    // rest, so a setting in a later one loads nothing.
    let Some(effective) = cli2.first() else {
        findings.push(finding(CLI2_NAMES[0], LINKS, Edit::AddCompanionCli2));
        return library_findings(target, &library, None, findings);
    };
    let Some(value) = read_configuration(&target.join(effective)) else {
        findings.push(finding(*effective, LINKS, Edit::Unknown));
        return library_findings(target, &library, None, findings);
    };
    let loaded = value
        .get("customRules")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|rules| {
            rules.iter().any(|rule| {
                rule.as_str()
                    .is_some_and(|rule| rule.contains(RELATIVE_LINKS_PACKAGE))
            })
        });
    if !loaded {
        findings.push(finding(*effective, LINKS, Edit::LoadRelativeLinks));
    }
    library_findings(target, &library, Some((effective, &value)), findings);
}

/// The findings on which file's rules turn the link rule on.
fn library_findings(
    target: &Utf8Path,
    library: &[&str],
    cli2: Option<(&str, &serde_json::Value)>,
    findings: &mut Vec<Finding>,
) {
    // A library-family file at the root replaces the CLI2 file's `config`,
    // so it is the one whose rules decide.
    if let Some(name) = library.first() {
        match read_configuration(&target.join(name)) {
            Some(rules) if enables(Some(&rules)) => {}
            Some(_) => findings.push(finding(*name, LINKS, Edit::EnableRelativeLinks)),
            None => findings.push(finding(*name, LINKS, Edit::Unknown)),
        }
    } else if let Some((name, value)) = cli2
        && !enables(value.get("config"))
    {
        findings.push(finding(name, LINKS, Edit::EnableRelativeLinks));
    }
}

/// The `ignores` the root CLI2 file the linter reads carries, where it is
/// readable.
fn root_ignores(target: &Utf8Path) -> Vec<String> {
    CLI2_NAMES
        .iter()
        .find(|name| std::fs::symlink_metadata(target.join(name)).is_ok())
        .and_then(|name| read_configuration(&target.join(name)))
        .and_then(|value| {
            value
                .get("ignores")
                .and_then(serde_json::Value::as_array)
                .cloned()
        })
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.as_str().map(String::from))
        .collect()
}

/// Every reserved pattern that matches Markdown the linter still reads.
fn reserved_findings(target: &Utf8Path, findings: &mut Vec<Finding>) {
    use crate::domain::path_filter::{Layer, PathFilter, Pattern};

    let Ok(declaration) = crate::domain::instance_config::InstanceConfig::read(target) else {
        return;
    };
    if declaration.reserved.is_empty() {
        return;
    }
    let markdown: Vec<Utf8PathBuf> = walkdir::WalkDir::new(target)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !(entry.depth() > 0
                && entry.file_type().is_dir()
                && crate::gates::PRUNED_DIRS.contains(&name.as_ref()))
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = Utf8Path::from_path(entry.path())?;
            let relative = path.strip_prefix(target).ok()?;
            (relative.extension() == Some("md")).then(|| relative.to_owned())
        })
        .collect();
    let ignores = root_ignores(target);
    for glob in &declaration.reserved {
        let Ok(filter) = PathFilter::build(
            Vec::new(),
            vec![Pattern::new(glob.clone(), Layer::Reserved)],
        ) else {
            continue;
        };
        let holds_markdown = markdown.iter().any(|path| !filter.judges(path));
        if holds_markdown && !ignores.iter().any(|ignored| ignored == glob) {
            findings.push(finding(glob.clone(), DECLARES, Edit::AddIgnore));
        }
    }
}

/// Report what the Markdown linter will read at `target`.
///
/// SATISFIES setup:lint-reports-each-finding-with-its-rule
///
/// # Errors
///
/// [`AppError::Refused`] where the flag names another profile than the
/// record.
pub fn report(target: &Utf8Path, flag: Option<ProfileId>) -> Result<Report, AppError> {
    let (profile, profile_source) = profile(target, flag)?;
    let docs_root = crate::candidate::docs_root_of(profile)?;
    let landed = profile_source == ProfileSource::Recorded;
    let mut findings = Vec::new();
    for path in crate::services::verifier::linked_lint_scan_roots(target, docs_root) {
        findings.push(finding(path.to_string(), COMPOSES, Edit::ReplaceLink));
    }
    for path in
        crate::services::verifier::lint_configurations_that_break_composition(target, docs_root)
    {
        findings.push(finding(path.to_string(), COMPOSES, Edit::FoldIntoRoot));
    }
    root_findings(target, landed, &mut findings);
    if landed {
        reserved_findings(target, &mut findings);
    }
    Ok(Report {
        schema: SCHEMA,
        target: target.to_owned(),
        profile,
        profile_source,
        findings,
    })
}

/// One run of the hook over the scratch file.
#[derive(Debug, Clone, Serialize)]
pub struct Run {
    /// The process exit code, where it exited with one.
    pub exit: Option<i32>,
    /// Whether the output names the link rule.
    pub cites_rule: bool,
}

/// The whole proof.
#[derive(Debug, Clone, Serialize)]
pub struct Proof {
    /// The machine schema of this object.
    pub schema: &'static str,
    /// The command the proof ran, before the scratch file's name.
    pub command: Vec<String>,
    /// The run over a broken relative link, which must fail on the rule.
    pub broken: Run,
    /// The run over a valid relative link, which must pass.
    pub valid: Run,
    /// Whether the rule fired on the broken link and stayed quiet on the
    /// valid one.
    pub ok: bool,
    /// The rule the proof holds.
    pub rule: &'static str,
}

/// The command a proof runs: the delivered hook through pre-commit, unless
/// the operator's environment names another runner.
fn command() -> Vec<String> {
    crate::domain::paths::variable(crate::domain::paths::LINT_PROVE_RUNNER_VAR).map_or_else(
        || {
            ["pre-commit", "run", "markdownlint-cli2", "--files"]
                .map(String::from)
                .to_vec()
        },
        |runner| runner.split_whitespace().map(String::from).collect(),
    )
}

/// Run the command over one file and read what it said.
fn run(target: &Utf8Path, command: &[String], file: &str) -> Result<Run, AppError> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| AppError::Usage("the lint proof runner names no program".to_string()))?;
    let output = std::process::Command::new(program)
        .args(args)
        .arg(file)
        .current_dir(target)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|source| {
            AppError::Unavailable(format!(
                "{program} could not run, so the proof cannot hold the rule: {source}"
            ))
        })?;
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(Run {
        exit: output.status.code(),
        cites_rule: said.contains(LINK_RULE),
    })
}

/// Prove the link rule runs at `target`, and remove the scratch file.
///
/// SATISFIES instance:the-root-lint-configuration-loads-relative-links
///
/// # Errors
///
/// [`AppError::Unavailable`] where the runner cannot start, and I/O errors
/// writing or removing the scratch file.
pub fn prove(target: &Utf8Path) -> Result<Proof, AppError> {
    let name = format!("sdd-lint-proof-{}.md", std::process::id());
    let path = target.join(&name);
    if std::fs::symlink_metadata(&path).is_ok() {
        return Err(AppError::Refused(format!(
            "{path} already exists, and the proof writes only a file of its own"
        )));
    }
    let command = command();
    let broken = format!("# Proof\n\nA [broken link](./{name}.missing.md).\n");
    let valid = format!("# Proof\n\nA [valid link](./{name}).\n");
    let result = (|| {
        crate::adapters::fs::write_file(&path, broken.as_bytes())?;
        let broken = run(target, &command, &name)?;
        crate::adapters::fs::write_file(&path, valid.as_bytes())?;
        let valid = run(target, &command, &name)?;
        Ok::<_, AppError>((broken, valid))
    })();
    let removed = std::fs::remove_file(&path);
    let (broken, valid) = result?;
    removed?;
    let ok = broken.exit != Some(0) && broken.cites_rule && valid.exit == Some(0);
    Ok(Proof {
        schema: PROVE_SCHEMA,
        command,
        broken,
        valid,
        ok,
        rule: LINKS.as_str(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonc_reads_comments_and_trailing_commas() {
        let value = jsonc(
            "// head\n{\n  \"customRules\": [\"a//b\", /* x */ \"c\",],\n  \"config\": {\"relative-links\": true,},\n}\n",
        )
        .expect("the JSONC parses");
        assert_eq!(value["customRules"][0], "a//b");
        assert!(enables(value.get("config")));
    }

    #[test]
    fn the_rule_is_enabled_by_true_or_by_options() {
        assert!(enables(Some(
            &serde_json::json!({"relative-links": {"root_path": "."}})
        )));
        assert!(!enables(Some(
            &serde_json::json!({"relative-links": false})
        )));
        assert!(!enables(Some(&serde_json::json!({}))));
        assert!(!enables(None));
    }
}
