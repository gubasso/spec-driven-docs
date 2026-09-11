//! `gate` subcommand: runtime-shape.
//!
//! Runs one delivered gate against the invoking repository, printing every
//! violation to stdout, lists the registry, or explains which gates judge a
//! path. Gate semantics live in `gates`; this handler resolves the subject
//! filter, holds the containment boundary, and reports.

use camino::Utf8Path;

use crate::cli::gate::GateArgs;
use crate::context::AppContext;
use crate::domain::gate_id::GateId;
use crate::domain::path_filter::{Decision, Layer, PathFilter, Pattern};
use crate::error::AppError;
use crate::gates::{GATES, GateCtx, spec};
use crate::output;

/// The documentation root patterns are templated against.
///
/// The declaration phase resolves this from the instance manifest. Until
/// then the profile default stands, which is what every current caller
/// renders with.
const DOCS_ROOT: &str = "_docs";

// sdd: permanent the braces are the wiring template's placeholder, not a formatting argument
#[allow(clippy::literal_string_with_formatting_args)]
fn substitute_root(pattern: &str) -> String {
    pattern.replace("{docs_root}", DOCS_ROOT)
}

/// Build one gate's subject filter from the registry and the flags.
///
/// The layers are appended in precedence order, earliest first, because the
/// matcher reports the last matching exclude.
fn filter_for(id: GateId, include: &[String], exclude: &[String]) -> Result<PathFilter, AppError> {
    let row = spec(id);
    let mut includes: Vec<Pattern> = row
        .include
        .iter()
        .map(|glob| Pattern::new(substitute_root(glob), Layer::Registry))
        .collect();
    let mut excludes: Vec<Pattern> = row
        .exclude
        .iter()
        .map(|glob| Pattern::new(substitute_root(glob), Layer::Registry))
        .collect();
    includes.extend(
        include
            .iter()
            .map(|glob| Pattern::new(glob.clone(), Layer::Flag)),
    );
    excludes.extend(
        exclude
            .iter()
            .map(|glob| Pattern::new(glob.clone(), Layer::Flag)),
    );
    PathFilter::build(includes, excludes).map_err(|error| AppError::Usage(error.to_string()))
}

/// Refuse a path that names something outside the repository.
///
/// Pre-commit supplies safe paths. The public direct command must not read
/// an arbitrary host file, so the check sits where operator input arrives
/// rather than where a gate reads.
fn contained(path: &str) -> Result<(), AppError> {
    let candidate = Utf8Path::new(path);
    if candidate.is_absolute() {
        return Err(AppError::Usage(format!(
            "{path} is absolute: a gate judges repository-relative paths"
        )));
    }
    if candidate.components().any(|part| part.as_str() == "..") {
        return Err(AppError::Usage(format!(
            "{path} climbs out of the repository"
        )));
    }
    let resolved = std::fs::canonicalize(path);
    let root = std::fs::canonicalize(".");
    if let (Ok(resolved), Ok(root)) = (resolved, root) {
        if !resolved.starts_with(&root) {
            return Err(AppError::Usage(format!(
                "{path} is reached through a link that leaves the repository"
            )));
        }
    }
    Ok(())
}

/// Report which gates judge one path, and what decided each answer.
///
/// This answers path-declaration eligibility. It is not a prediction of what
/// pre-commit will run: pre-commit applies `types:` in addition to the
/// rendered patterns and this command does not, so a row's `types:` is
/// printed rather than folded into the answer.
fn explain(path: &str) -> Result<(), AppError> {
    contained(path)?;
    let subject = Utf8Path::new(path);
    for gate in GATES {
        let filter = filter_for(gate.id, &[], &[])?;
        let types = gate
            .types
            .map_or_else(String::new, |types| format!("  types: [{types}]"));
        let line = match filter.decide(subject) {
            Decision::Read => format!("judges       {}{types}", gate.id),
            Decision::Skipped(pattern) => format!(
                "skipped      {}  exclude {}  ({}){types}",
                gate.id, pattern.glob, pattern.layer
            ),
            Decision::NotIncluded => format!(
                "not included {}  include {}{types}",
                gate.id,
                filter
                    .includes()
                    .iter()
                    .map(|p| p.glob.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        output::line(line);
    }
    output::line("note: pre-commit also applies each row's types:, which this answer does not.");
    Ok(())
}

/// Run, list, or explain.
///
/// # Errors
///
/// [`AppError::Violations`] when the gate found any; [`AppError::Usage`] for
/// a refused path or a malformed pattern; I/O errors when it could not run.
pub fn run(_ctx: &AppContext, args: GateArgs) -> Result<(), AppError> {
    let GateArgs {
        id,
        files,
        list,
        explain: explain_path,
        include,
        exclude,
    } = args;
    if list {
        for gate in GATES {
            output::line(format!("{}: {}", gate.id, gate.name));
        }
        return Ok(());
    }
    if let Some(path) = explain_path {
        return explain(&path);
    }
    let Some(id) = id else {
        return Err(AppError::Usage(
            "a gate id, --list, or --explain is required".to_string(),
        ));
    };

    for path in &files {
        contained(path)?;
    }
    let filter = filter_for(id, &include, &exclude)?;
    // Filter the passed paths always. Ruff carries `--force-exclude`
    // because the opposite default surprised people under pre-commit, which
    // passes changed files explicitly, and pre-commit is this tool's only
    // caller.
    let gate_ctx = GateCtx::with_filter(".", filter);
    let judged: Vec<String> = gate_ctx
        .subjects(files.iter().map(Utf8Path::new))
        .into_iter()
        .map(ToString::to_string)
        .collect();

    let violations = (spec(id).run)(&gate_ctx, &judged)?;
    if violations.is_empty() {
        return Ok(());
    }
    for violation in &violations {
        output::line(violation);
    }
    Err(AppError::Violations {
        count: violations.len(),
    })
}
