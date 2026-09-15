//! The stage: a persistent workbench holding one rendered candidate.
//!
//! A stage is evidence an agent reads. It holds every destination the
//! candidate would land, the reference material of the exact installed
//! version, and a receipt describing both. It is written outside the target
//! and never into it, and no production verb reads a byte of it: a landing
//! renders the candidate again from this binary's own sources.
//!
//! The directory is renamed into place once it is complete, so a stage a
//! reader can see is a stage that finished. Only `sdd stage clean` removes
//! one, and only after the receipt inside it says this tool wrote it.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::domain::manifest::MANIFEST_PATH;
use crate::domain::profile::ProfileId;
use crate::error::AppError;

/// The machine schema `stage.json` declares.
pub const STAGE_SCHEMA: &str = "sdd.stage/1";

/// The base directory an automation points every stage at.
pub const STAGE_ROOT_VAR: &str = "SDD_STAGE_ROOT";

/// The receipt every stage carries, and the one file `clean` trusts.
pub const RECEIPT_FILE: &str = "stage.json";

/// Where the candidate's own destinations are rendered.
pub const ARTIFACTS_DIR: &str = "artifacts";

/// Where the installed version's reference material is copied.
pub const REFERENCE_DIR: &str = "reference";

/// The payload roots a stage carries as reference material.
///
/// Every root the binary carries that an adopting project reads while it
/// migrates. The repository's own decision records and tests are not here:
/// they are this project's history and its proof, and neither is knowledge
/// an adopting project reads.
const REFERENCE_ROOTS: [&str; 8] = [
    "method",
    "templates",
    "_docs/specs",
    "instance",
    "comparison-docs",
    "reference/prior-art",
    "reference/tracker-markup",
    ".markdownlint",
];

/// What a stage was asked to render.
///
/// Every choice a landing takes is here too. A stage rendered under
/// different choices than the landing that follows it would be evidence
/// about a candidate nobody is going to write.
#[derive(Debug, Clone)]
pub struct Request {
    /// The repository to render a candidate for.
    pub target: Utf8PathBuf,
    /// The profile to project.
    pub profile: ProfileId,
    /// The stage directory the operator named.
    pub output: Option<Utf8PathBuf>,
    /// The documentation scratch the candidate would record.
    pub docs_scratch: Option<Option<Utf8PathBuf>>,
    /// Paths the candidate would record under `reserved:`.
    pub reserve: Vec<String>,
    /// The writing-style selection the candidate would record.
    pub writing_style: Option<crate::domain::instance_config::WritingStyle>,
}

/// Which rule chose the stage root, for the report that names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RootSource {
    /// The `--output` flag.
    Flag,
    /// The `SDD_STAGE_ROOT` base an automation declared.
    Environment,
    /// This tool's own state root.
    Default,
}

/// One artifact the stage carries, and what a landing would do with it.
#[derive(Debug, Clone, Serialize)]
pub struct Artifact {
    /// The destination, relative to the target.
    pub path: String,
    /// Who owns the bytes after a landing.
    pub ownership: String,
    /// Whether the candidate carries the file or one region of it.
    pub placement: String,
}

/// The receipt a stage carries, and the report the command prints.
#[derive(Debug, Clone, Serialize)]
pub struct Receipt {
    /// The machine schema of this record.
    pub schema: &'static str,
    /// The version of the binary that rendered the candidate.
    pub version: String,
    /// The target this candidate was rendered for.
    pub target: Utf8PathBuf,
    /// The profile projected.
    pub profile: ProfileId,
    /// Where this stage sits.
    pub root: Utf8PathBuf,
    /// Which rule chose that root.
    pub root_source: RootSource,
    /// The version the target's own record claims, where one is readable.
    pub recorded_version: Option<String>,
    /// The documentation root the candidate records.
    pub docs_root: String,
    /// The documentation scratch the candidate records.
    pub docs_scratch: Option<Utf8PathBuf>,
    /// The paths the candidate's own declaration reserves.
    pub reserve: Vec<String>,
    /// The writing source the candidate's own declaration selects.
    pub writing_style: String,
    /// Every destination the candidate would land.
    pub artifacts: Vec<Artifact>,
    /// Every reference root copied, relative to the stage.
    pub reference: Vec<String>,
    /// What the projection chose to leave alone, and why.
    pub notes: Vec<String>,
}

/// Where a stage goes, and which rule decided.
///
/// The flag wins, then the automation base, then this tool's state root.
/// The base is a directory of stages rather than one stage, so an
/// automation that stages many targets names one variable.
///
/// # Errors
///
/// [`AppError::Usage`] when the resolved path is relative.
pub fn resolve_root(
    target: &Utf8Path,
    output: Option<&Utf8Path>,
    env: Option<&str>,
    state_root: &Utf8Path,
) -> Result<(Utf8PathBuf, RootSource), AppError> {
    let (root, source) = match (output, env) {
        (Some(named), _) => (named.to_owned(), RootSource::Flag),
        (None, Some(base)) if !base.is_empty() => (
            Utf8Path::new(base).join(stage_name(target)),
            RootSource::Environment,
        ),
        _ => (
            state_root.join("stages").join(stage_name(target)),
            RootSource::Default,
        ),
    };
    if !root.is_absolute() {
        return Err(AppError::Usage(format!(
            "the stage root must be absolute: {root}"
        )));
    }
    Ok((root, source))
}

/// The nearest existing ancestor of a path, resolved through every link.
///
/// A path that does not exist yet still has an ancestor that does, and
/// that is what says where it will really be created.
fn resolved_ancestor(path: &Utf8Path) -> Utf8PathBuf {
    let mut walked = path.to_owned();
    loop {
        if let Ok(canonical) = std::fs::canonicalize(walked.as_std_path())
            && let Ok(canonical) = Utf8PathBuf::from_path_buf(canonical)
        {
            let rest = path
                .strip_prefix(&walked)
                .unwrap_or_else(|_| Utf8Path::new(""));
            return canonical.join(rest);
        }
        let Some(parent) = walked.parent() else {
            return path.to_owned();
        };
        walked = parent.to_owned();
    }
}

/// The writing source a declaration selects, as one word a reader keeps.
fn writing_style_of(declaration: &crate::domain::instance_config::InstanceConfig) -> String {
    use crate::domain::instance_config::WritingSource;

    match declaration.writing_style.source {
        WritingSource::Builtin => "builtin".to_string(),
        WritingSource::None => "none".to_string(),
        WritingSource::Project => declaration
            .writing_style
            .path
            .as_ref()
            .map_or_else(|| "project".to_string(), |path| format!("project:{path}")),
    }
}

/// A scratch sibling this run alone owns.
///
/// The name carries the process and a timestamp, and the directory is
/// created exclusively, so the run never removes a path somebody else
/// left. A collision refuses rather than clearing what is there.
fn scratch_beside(root: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    let stamp = jiff::Timestamp::now().as_nanosecond();
    let partial = Utf8PathBuf::from(format!("{root}.partial-{}-{stamp}", std::process::id()));
    if let Some(parent) = partial.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(&partial).map_err(|source| {
        AppError::Refused(format!(
            "{partial} could not be created for this run: {source}"
        ))
    })?;
    Ok(partial)
}

/// One target's stage directory name: its own name and a digest of its path.
///
/// Two checkouts of one repository stage side by side, and neither name is
/// guessed from the other.
fn stage_name(target: &Utf8Path) -> String {
    let digest = crate::domain::ownership::Sha256::of(target.as_str().as_bytes()).to_string();
    let leaf = target.file_name().unwrap_or("target");
    format!("{leaf}-{}", &digest[..12])
}

/// Render one candidate into a stage, and report what it holds.
///
/// # Errors
///
/// [`AppError::Usage`] for a target or stage path the arguments cannot
/// mean, [`AppError::Refused`] where the stage directory already holds
/// something, and I/O errors writing the stage.
pub fn create(request: &Request, state_root: &Utf8Path) -> Result<Receipt, AppError> {
    let target = crate::services::installer::resolved_target(&request.target)?;
    let env = std::env::var(STAGE_ROOT_VAR).ok();
    let (root, root_source) = resolve_root(
        &target,
        request.output.as_deref(),
        env.as_deref(),
        state_root,
    )?;

    // A stage inside the target would be the one thing this command
    // promises not to do: put the candidate and its reference corpus into
    // the repository it is supposed to leave alone. The comparison is
    // between resolved paths, because a link in the output's ancestry
    // would otherwise carry it back inside.
    let resolved = resolved_ancestor(&root);
    if resolved == target || resolved.starts_with(&target) {
        return Err(AppError::Refused(format!(
            "{root} is inside {target}, and a stage is written outside the target it describes"
        )));
    }
    if root.exists() && root.read_dir_utf8().is_ok_and(|mut it| it.next().is_some()) {
        return Err(AppError::Refused(format!(
            "{root} already holds a stage; read it, or remove it with 'sdd stage clean {root}'"
        )));
    }

    let options = crate::services::installer::InitOptions {
        target: target.clone(),
        profile: request.profile,
        apply: false,
        dry_run: true,
        docs_scratch: request.docs_scratch.clone(),
        reserve: request.reserve.clone(),
        writing_style: request.writing_style.clone(),
    };
    let candidate = crate::services::installer::candidate_for(&target, &options)?;

    // Render beside the destination and rename the finished directory into
    // place, so a reader never meets a stage that is still being written.
    // The scratch name is this run's own: a predictable one would make the
    // command remove a sibling nothing proves it wrote.
    let partial = scratch_beside(&root)?;
    let render = render(&partial, &target, &root, root_source, &candidate, request);
    match render {
        Ok(receipt) => finish(&partial, &root, receipt),
        Err(error) => {
            // The directory is this run's own, so removing it takes back
            // only what this run made. A failure that left it behind would
            // accumulate debris no command removes.
            let _ = std::fs::remove_dir_all(&partial);
            Err(error)
        }
    }
}

/// Rename the finished directory into place.
fn finish(partial: &Utf8Path, root: &Utf8Path, receipt: Receipt) -> Result<Receipt, AppError> {
    if let Some(parent) = root.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // An empty directory at the destination is the one thing a rename
    // cannot land on. Anything else there already refused above.
    let _ = std::fs::remove_dir(root);
    if let Err(source) = std::fs::rename(partial, root) {
        let _ = std::fs::remove_dir_all(partial);
        return Err(AppError::Io(source));
    }
    Ok(receipt)
}

/// Write one candidate and its reference material under `partial`.
fn render(
    partial: &Utf8Path,
    target: &Utf8Path,
    root: &Utf8Path,
    root_source: RootSource,
    candidate: &crate::candidate::Candidate,
    request: &Request,
) -> Result<Receipt, AppError> {
    let mut artifacts = Vec::new();
    for destination in &candidate.destinations {
        let path = partial.join(ARTIFACTS_DIR).join(&destination.path);
        crate::adapters::fs::write_file(&path, &destination.bytes)?;
        artifacts.push(Artifact {
            path: destination.path.to_string(),
            ownership: match destination.ownership {
                crate::candidate::Ownership::Managed => "managed",
                crate::candidate::Ownership::Adopted => "adopted",
                crate::candidate::Ownership::Integration => "integration",
            }
            .to_string(),
            placement: match destination.placement {
                crate::candidate::Placement::WholeFile => "whole-file",
                crate::candidate::Placement::MarkedRegion => "marked-region",
            }
            .to_string(),
        });
    }
    crate::adapters::fs::write_file(
        &partial.join(ARTIFACTS_DIR).join(MANIFEST_PATH),
        candidate.manifest.to_json().as_bytes(),
    )?;
    artifacts.push(Artifact {
        path: MANIFEST_PATH.to_string(),
        ownership: "record".to_string(),
        placement: "whole-file".to_string(),
    });

    let mut reference = Vec::new();

    // The skills are copied as the packages an agent resolves, not as the
    // authored tree: a skill names its gates relative to its own root, and
    // a copy that split them would carry two broken references.
    for name in crate::embedded::skill_names() {
        let Some(package) = crate::embedded::skill_package(name) else {
            continue;
        };
        for (relative, bytes) in package {
            crate::adapters::fs::write_file(
                &partial
                    .join(REFERENCE_DIR)
                    .join("skills")
                    .join(name)
                    .join(&relative),
                bytes,
            )?;
        }
    }
    reference.push(format!("{REFERENCE_DIR}/skills"));

    // The release notes of the exact installed version, which is where a
    // migration reads what this release asked of an instance.
    crate::adapters::fs::write_file(
        &partial.join(REFERENCE_DIR).join("CHANGELOG.md"),
        crate::embedded::CHANGELOG.as_bytes(),
    )?;
    reference.push(format!("{REFERENCE_DIR}/CHANGELOG.md"));

    for root_name in REFERENCE_ROOTS {
        let mut carried = false;
        for (path, bytes) in crate::embedded::assets_under(root_name) {
            crate::adapters::fs::write_file(&partial.join(REFERENCE_DIR).join(&path), bytes)?;
            carried = true;
        }
        if carried {
            reference.push(format!("{REFERENCE_DIR}/{root_name}"));
        }
    }
    if let Some(bytes) = crate::embedded::asset("instance/docs-catalog.toml") {
        crate::adapters::fs::write_file(
            &partial
                .join(REFERENCE_DIR)
                .join("instance/docs-catalog.toml"),
            bytes,
        )?;
        reference.push(format!("{REFERENCE_DIR}/instance/docs-catalog.toml"));
    }

    let receipt = Receipt {
        schema: STAGE_SCHEMA,
        version: crate::domain::version::CanonVersion::current().to_string(),
        target: target.to_owned(),
        profile: request.profile,
        root: root.to_owned(),
        root_source,
        recorded_version: crate::services::installer::recorded_field(target, "canon_version")
            .and_then(|value| value.as_str().map(String::from)),
        docs_root: candidate.manifest.docs_root.to_string(),
        docs_scratch: candidate.manifest.docs_scratch.clone(),
        reserve: candidate.declaration.reserved.clone(),
        writing_style: writing_style_of(&candidate.declaration),
        artifacts,
        reference,
        notes: candidate.notes.clone(),
    };
    let json = serde_json::to_string_pretty(&receipt)
        .map_err(|error| anyhow::anyhow!("the stage receipt does not serialize: {error}"))?;
    crate::adapters::fs::write_file(&partial.join(RECEIPT_FILE), json.as_bytes())?;

    Ok(receipt)
}

/// Remove one stage this tool wrote, and refuse anything else.
///
/// # Errors
///
/// [`AppError::Usage`] for a relative path, and [`AppError::Refused`] for a
/// path that is not a stage, a stage whose receipt names another root, a
/// link, or a root this tool will not recurse into.
pub fn clean(path: &Utf8Path) -> Result<Vec<String>, AppError> {
    if !path.is_absolute() {
        return Err(AppError::Usage(format!(
            "the stage path must be absolute: {path}"
        )));
    }
    // The final component is inspected without following it, so a link
    // standing where a stage stood cannot redirect the removal.
    let held = std::fs::symlink_metadata(path)
        .map_err(|source| AppError::Refused(format!("{path}: {source}")))?;
    if held.file_type().is_symlink() {
        return Err(AppError::Refused(format!(
            "{path} is a symlink; a stage is a directory this tool wrote"
        )));
    }
    if !held.is_dir() {
        return Err(AppError::Refused(format!("{path} is not a directory")));
    }
    if path.parent().is_none() || path.as_str().matches('/').count() < 2 {
        return Err(AppError::Refused(format!(
            "{path} is too close to the filesystem root to remove"
        )));
    }
    if path.join(MANIFEST_PATH).exists() || path.join(".git").exists() {
        return Err(AppError::Refused(format!(
            "{path} looks like a project rather than a stage"
        )));
    }
    // A receipt is ordinary user-writable JSON, so it proves nothing on
    // its own. These refusals are what stand between a forged one and a
    // recursive removal of somewhere that matters.
    if let Some(home) = crate::domain::paths::UserEnv::from_process().home
        && path == home
    {
        return Err(AppError::Refused(format!(
            "{path} is the home directory, which is never a stage"
        )));
    }
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(cwd) = Utf8PathBuf::from_path_buf(cwd)
        && cwd.starts_with(path)
    {
        return Err(AppError::Refused(format!(
            "{path} holds the working directory, so it is not a stage this tool wrote"
        )));
    }
    // A stage this tool wrote holds the tree it renders. A directory that
    // carries a receipt and none of that structure is something else
    // wearing the name.
    if !path.join(ARTIFACTS_DIR).is_dir() {
        return Err(AppError::Refused(format!(
            "{path} carries no {ARTIFACTS_DIR}/ directory, so it is not a stage this tool wrote"
        )));
    }

    let receipt_path = path.join(RECEIPT_FILE);
    let text = std::fs::read_to_string(&receipt_path).map_err(|source| {
        AppError::Refused(format!(
            "{receipt_path} is not readable, so this is not a stage this tool wrote: {source}"
        ))
    })?;
    let receipt: serde_json::Value = serde_json::from_str(&text)
        .map_err(|source| AppError::Refused(format!("{receipt_path} does not parse: {source}")))?;
    if receipt.get("schema").and_then(serde_json::Value::as_str) != Some(STAGE_SCHEMA) {
        return Err(AppError::Refused(format!(
            "{receipt_path} does not declare {STAGE_SCHEMA}, so this is not a stage"
        )));
    }
    let declared = receipt
        .get("root")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::Refused(format!("{receipt_path} declares no root")))?;
    if declared != path.as_str() {
        return Err(AppError::Refused(format!(
            "{receipt_path} declares the root {declared}, and this path is {path}"
        )));
    }

    // Move the validated directory aside under a name this run owns, then
    // remove that. A directory swapped in after the checks is left where
    // it is, because what this removes is the thing it just moved.
    let aside = scratch_beside(path)?;
    let _ = std::fs::remove_dir(&aside);
    std::fs::rename(path, &aside)?;
    std::fs::remove_dir_all(&aside)?;
    Ok(vec![format!("removed {path}")])
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn root(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap())
    }

    #[test]
    fn the_flag_wins_then_the_base_then_the_state_root() {
        let target = Utf8Path::new("/work/project");
        let state = Utf8Path::new("/state/spec-driven-docs");

        let (path, source) = resolve_root(
            target,
            Some(Utf8Path::new("/tmp/here")),
            Some("/base"),
            state,
        )
        .unwrap();
        assert_eq!(path, "/tmp/here");
        assert_eq!(source, RootSource::Flag);

        let (path, source) = resolve_root(target, None, Some("/base"), state).unwrap();
        assert!(path.as_str().starts_with("/base/project-"), "{path}");
        assert_eq!(source, RootSource::Environment);

        let (path, source) = resolve_root(target, None, None, state).unwrap();
        assert!(
            path.as_str().starts_with(state.join("stages").as_str()),
            "{path}"
        );
        assert_eq!(source, RootSource::Default);
    }

    #[test]
    fn a_relative_stage_root_is_a_usage_error() {
        let error = resolve_root(
            Utf8Path::new("/work/project"),
            Some(Utf8Path::new("stage")),
            None,
            Utf8Path::new("/state"),
        )
        .unwrap_err();
        assert_eq!(error.kind(), "Usage");
    }

    #[test]
    fn two_targets_stage_under_different_names() {
        assert_ne!(
            stage_name(Utf8Path::new("/one/project")),
            stage_name(Utf8Path::new("/two/project"))
        );
    }

    #[test]
    fn clean_refuses_a_directory_that_is_not_a_stage() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("not-a-stage");
        std::fs::create_dir_all(&path).unwrap();
        let error = clean(&path).unwrap_err();
        assert_eq!(error.kind(), "Refused");
        assert!(path.exists(), "the directory was removed anyway");
    }

    #[test]
    fn clean_refuses_a_receipt_that_names_another_root() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("stage");
        std::fs::create_dir_all(path.join(ARTIFACTS_DIR)).unwrap();
        crate::adapters::fs::write_file(
            &path.join(RECEIPT_FILE),
            format!(r#"{{"schema":"{STAGE_SCHEMA}","root":"/elsewhere"}}"#).as_bytes(),
        )
        .unwrap();
        let error = clean(&path).unwrap_err();
        assert!(error.to_string().contains("/elsewhere"), "{error}");
        assert!(path.exists());
    }

    #[test]
    fn clean_refuses_a_link_standing_where_a_stage_stood() {
        let dir = tempfile::tempdir().unwrap();
        let real = root(&dir).join("stage");
        std::fs::create_dir_all(real.join(ARTIFACTS_DIR)).unwrap();
        crate::adapters::fs::write_file(
            &real.join(RECEIPT_FILE),
            format!(r#"{{"schema":"{STAGE_SCHEMA}","root":"{real}"}}"#).as_bytes(),
        )
        .unwrap();
        let link = root(&dir).join("link");
        std::os::unix::fs::symlink(real.as_std_path(), link.as_std_path()).unwrap();

        let error = clean(&link).unwrap_err();
        assert!(error.to_string().contains("symlink"), "{error}");
        assert!(real.exists(), "the link's target was removed");
    }

    #[test]
    fn clean_refuses_a_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("project");
        crate::adapters::fs::write_file(&path.join(MANIFEST_PATH), b"{}").unwrap();
        crate::adapters::fs::write_file(
            &path.join(RECEIPT_FILE),
            format!(r#"{{"schema":"{STAGE_SCHEMA}","root":"{path}"}}"#).as_bytes(),
        )
        .unwrap();
        let error = clean(&path).unwrap_err();
        assert!(error.to_string().contains("project"), "{error}");
        assert!(path.exists());
    }

    #[test]
    fn clean_refuses_a_forged_receipt_over_a_directory_that_is_not_a_stage() {
        let dir = tempfile::tempdir().unwrap();
        // Somebody's directory of work, with a receipt dropped in it that
        // names the directory as its own root.
        let path = root(&dir).join("someones-files");
        crate::adapters::fs::write_file(&path.join("notes.md"), b"mine").unwrap();
        crate::adapters::fs::write_file(
            &path.join(RECEIPT_FILE),
            format!(r#"{{"schema":"{STAGE_SCHEMA}","root":"{path}"}}"#).as_bytes(),
        )
        .unwrap();

        let error = clean(&path).unwrap_err();
        assert!(error.to_string().contains(ARTIFACTS_DIR), "{error}");
        assert!(path.join("notes.md").exists(), "the directory was removed");
    }

    #[test]
    fn clean_refuses_a_directory_holding_the_working_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(std::env::current_dir().unwrap())
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();
        let _ = dir;
        let error = clean(&path).unwrap_err();
        assert_eq!(error.kind(), "Refused");
        assert!(path.exists());
    }

    #[test]
    fn clean_removes_one_valid_stage() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("stage");
        crate::adapters::fs::write_file(&path.join(ARTIFACTS_DIR).join("AGENTS.md"), b"x").unwrap();
        crate::adapters::fs::write_file(
            &path.join(RECEIPT_FILE),
            format!(r#"{{"schema":"{STAGE_SCHEMA}","root":"{path}"}}"#).as_bytes(),
        )
        .unwrap();
        assert_eq!(clean(&path).unwrap().len(), 1);
        assert!(!path.exists());
    }
}
