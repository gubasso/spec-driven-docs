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
use crate::domain::instance_config::{self, InstanceConfig};
use crate::domain::path_filter::{Decision, PathFilter};
use crate::error::AppError;
use crate::gates::{GATES, GateCtx, spec};
use crate::output;

/// The documentation root patterns are templated against.
///
/// It comes from the instance's own record, because the two profiles use
/// different roots. Templating against a fixed `_docs` would build patterns
/// no path in a `codebase` instance matches, and every filename-selected
/// gate there would silently judge nothing.
fn docs_root() -> String {
    crate::services::verifier::read_manifest(Utf8Path::new("."))
        .map_or_else(|_| "_docs".to_string(), |m| m.docs_root.to_string())
}

// sdd: permanent the braces are the wiring template's placeholder, not a formatting argument
#[allow(clippy::literal_string_with_formatting_args)]
fn substitute_root(pattern: &str, docs_root: &str) -> String {
    pattern.replace("{docs_root}", docs_root)
}

/// Build one gate's subject filter from every layer.
///
/// The registry is the first layer, the project's declaration the second and
/// the fourth, and the command-line flags the third.
/// [`instance_config::resolve`] owns the algorithm, so the command line and
/// the renderer cannot disagree about it.
fn filter_for(
    id: GateId,
    declaration: &InstanceConfig,
    docs_root: &str,
    include: &[String],
    exclude: &[String],
) -> Result<PathFilter, AppError> {
    let row = spec(id);
    let registry_include: Vec<String> = row
        .include
        .iter()
        .map(|g| substitute_root(g, docs_root))
        .collect();
    let registry_exclude: Vec<String> = row
        .exclude
        .iter()
        .map(|g| substitute_root(g, docs_root))
        .collect();
    instance_config::resolve(
        &registry_include,
        &registry_exclude,
        declaration.for_gate(id),
        include,
        exclude,
        &declaration.reserved,
    )
    .map_err(|error| AppError::Usage(error.to_string()))
}

/// The declaration this invocation resolves against.
fn declaration() -> Result<InstanceConfig, AppError> {
    InstanceConfig::read(Utf8Path::new(".")).map_err(|error| AppError::Usage(error.to_string()))
}

/// Refuse a path this command must not follow.
///
/// A relative path that climbs out of the repository is refused, and so is
/// one reached through a link that leaves it. An absolute path is not:
/// pre-commit hands the message file at the `commit-msg` stage by absolute
/// path, and it sits outside the working tree by design, so refusing
/// absolutes would break a delivered wiring. What a gate reads stays bounded
/// by what pre-commit or the operator names, which was already true.
fn contained(path: &str) -> Result<(), AppError> {
    let candidate = Utf8Path::new(path);
    if candidate.components().any(|part| part.as_str() == "..") {
        return Err(AppError::Usage(format!(
            "{path} climbs out of the repository"
        )));
    }
    if candidate.is_absolute() {
        return Ok(());
    }
    // A relative path that resolves outside the repository got there through
    // a link, which is the case the `..` check cannot see.
    if let (Ok(resolved), Ok(root)) = (std::fs::canonicalize(path), std::fs::canonicalize(".")) {
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
    let declaration = declaration()?;
    let docs_root = docs_root();
    // Decide on the form a pattern speaks. `GateCtx` does this for the run
    // path; this one answers without a context.
    let relative = crate::domain::path_filter::project(Utf8Path::new(path), Utf8Path::new("."));
    let subject = relative.as_path();
    for gate in GATES {
        let filter = filter_for(gate.id, &declaration, &docs_root, &[], &[])?;
        let types = gate
            .types
            .map_or_else(String::new, |types| format!("  types: [{types}]"));
        // An `always_run` row discovers its own subjects, so the registry
        // include does not narrow them and `--explain` must say so. Answering
        // with `decide` here reported a path the gate does judge as
        // `not included`.
        let decision = if gate.always_run && filter.retains(subject) {
            Decision::Read
        } else {
            filter.decide(subject)
        };
        let line = match decision {
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
    let filter = filter_for(id, &declaration()?, &docs_root(), &include, &exclude)?;
    // Filter the passed paths always. Ruff carries `--force-exclude`
    // because the opposite default surprised people under pre-commit, which
    // passes changed files explicitly, and pre-commit is this tool's only
    // caller.
    let gate_ctx = GateCtx::with_filter(".", filter);
    // A positional value is a file to judge or a record root to resolve,
    // and `src/cli/gate.rs` says so. Filter the files; pass a directory
    // through untouched, because it is a support root and the records
    // discovered beneath it are filtered where the gate resolves them.
    // Filtering the root itself would silently drop it and let the gate
    // fall back to the default location, reporting nothing.
    let (roots, subjects): (Vec<String>, Vec<String>) = files
        .into_iter()
        .partition(|path| gate_ctx.path(path).is_dir());
    let judged: Vec<String> = roots
        .into_iter()
        .chain(
            gate_ctx
                .subjects(subjects.iter().map(Utf8Path::new))
                .into_iter()
                .map(ToString::to_string),
        )
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
