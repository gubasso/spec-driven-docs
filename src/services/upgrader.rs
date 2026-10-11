//! Binary-driven instance upgrade.
//!
//! The newer binary carries the newer payload, so an upgrade is mechanical:
//! refuse atomically while any managed file is locally edited, reinstall
//! from the embedded payload, prune what the new version stopped managing —
//! inside the vendored directory only, and never through a symlink — and
//! report the rule-ID diff the operator reconciles by hand. Copier's model;
//! what changed between versions is the changelog's business.

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::adapters::fs::sha256_file;
use crate::domain::manifest::{LegacyManifest, MANIFEST_PATH, Manifest, ManifestParseError};
use crate::domain::ownership::Sha256;
use crate::domain::profile::ProfileId;
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::services::installer::{InitOptions, init, init_holding};

/// What an upgrade was asked to do.
#[derive(Debug, Clone)]
pub struct UpgradeOptions {
    /// The absolute target instance.
    pub target: Utf8PathBuf,
    /// Report what would change and change nothing.
    pub dry_run: bool,
    /// The choices a stage recorded, where the landing takes them.
    pub staged: Option<crate::stage::StagedChoices>,
    /// The conflicts the operator named for replacement, as the record
    /// names each destination.
    pub replace: Vec<String>,
}

/// What kind of conflict one destination is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictKind {
    /// A managed file whose bytes are not the recorded ones.
    EditedManagedFile,
    /// A managed file the record names and the target lacks.
    MissingManagedFile,
    /// A managed region whose bytes are not the recorded ones.
    EditedManagedBlock,
    /// The file a recorded managed region lives in is gone.
    MissingIntegrationHost,
    /// A destination the candidate lands that holds bytes no record
    /// vouches for.
    Unattributed,
}

/// One destination the upgrade cannot overwrite unasked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Conflict {
    /// The destination, relative to the target.
    pub path: String,
    /// What kind of conflict it is.
    pub kind: ConflictKind,
}

impl Conflict {
    /// The line the text report prints.
    fn line(&self) -> String {
        let what = match self.kind {
            ConflictKind::EditedManagedFile => "locally edited managed file",
            ConflictKind::MissingManagedFile => "missing managed file",
            ConflictKind::EditedManagedBlock => "locally edited managed block",
            ConflictKind::MissingIntegrationHost => "missing integration host",
            ConflictKind::Unattributed => "destination holds bytes no record vouches for",
        };
        format!("CONFLICT {what}: {}", self.path)
    }
}

/// One named conflict, and where its bytes were saved before the landing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Replaced {
    /// The destination, relative to the target.
    pub path: String,
    /// Where its bytes are kept, or nothing where the target held none.
    pub saved_to: Option<Utf8PathBuf>,
}

/// One citation of a rule ID this binary no longer defines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Citation {
    /// The citing file, relative to the target.
    pub path: Utf8PathBuf,
    /// The one-based line the citation sits on.
    pub line: usize,
    /// The rule ID cited.
    pub id: String,
}

/// What an upgrade did.
#[derive(Debug, Default)]
pub struct UpgradeOutcome {
    /// Every line to print, in order.
    pub lines: Vec<String>,
    /// How many lines are conflicts or failures.
    pub failures: usize,
    /// The release the record names, where one was read.
    pub from: Option<CanonVersion>,
    /// Every conflict the target holds, named or not.
    pub conflicts: Vec<Conflict>,
    /// Every named conflict and where its bytes went.
    pub replaced: Vec<Replaced>,
    /// Every citation of a rule ID this binary no longer defines.
    pub retired_citations: Vec<Citation>,
}

struct Installed {
    version: CanonVersion,
    profile: ProfileId,
    docs_root: String,
    managed: Vec<(Utf8PathBuf, Sha256)>,
    integration: Vec<(Utf8PathBuf, Sha256)>,
    /// Whether the record is already at this binary's schema.
    ///
    /// The two versions move independently, so an instance can carry this
    /// binary's canon version and an older schema. Without this the "already
    /// at" shortcut would return before migrating, while every other verb
    /// refuses the record and sends the operator back here.
    schema_current: bool,
}

/// The marker pair a host file's managed region uses.
fn markers_for(path: &str) -> (&'static str, &'static str) {
    use crate::domain::marker::{AGENTS_BEGIN, AGENTS_END, BEGIN, END};
    if path == crate::domain::paths::HOOKS_CONFIG_PATH {
        (BEGIN, END)
    } else {
        (AGENTS_BEGIN, AGENTS_END)
    }
}

fn read_installed(target: &Utf8Path) -> Result<Installed, AppError> {
    let path = target.join(MANIFEST_PATH);
    if !path.is_file() {
        return Err(AppError::ManifestMissing(path));
    }
    let text = std::fs::read_to_string(&path)?;
    match Manifest::parse(&text) {
        Ok(manifest) => Ok(Installed {
            version: manifest.canon_version,
            profile: manifest.profile,
            docs_root: manifest.docs_root.as_str().to_string(),
            managed: manifest
                .managed_files
                .into_iter()
                .map(|entry| (entry.destination, entry.sha256))
                .collect(),
            integration: manifest
                .integration_blocks
                .into_iter()
                .map(|block| (block.path, block.marker_hash))
                .collect(),
            schema_current: true,
        }),
        Err(ManifestParseError::Older(_)) => {
            let legacy: LegacyManifest = serde_json::from_str(&text)
                .map_err(|e| AppError::ManifestInvalid(e.to_string()))?;
            Ok(Installed {
                version: legacy.canon_version,
                profile: legacy.profile,
                docs_root: legacy.docs_root.as_str().to_string(),
                managed: legacy
                    .managed_files
                    .into_iter()
                    .map(|entry| (entry.destination, entry.sha256))
                    .collect(),
                integration: legacy
                    .integration_blocks
                    .into_iter()
                    .map(|block| (block.path, block.marker_hash))
                    .collect(),
                schema_current: false,
            })
        }
        Err(error) => Err(AppError::ManifestInvalid(error.to_string())),
    }
}

/// Refuse an answer on a path where no plan offers a decision.
///
/// A target with nothing to do and one whose managed files were edited
/// both end before a plan exists. An answer given on either is an answer
/// to a question nobody asked, and letting it pass silently would make
/// typing look like consent on the one path where nothing checked it.
/// A downgrade refuses on its own terms before this can matter.
///
/// # Errors
///
/// [`AppError::Usage`] naming the decision nothing offered.
/// Every managed file and region the target no longer holds as recorded.
///
/// A reinstall replaces a managed file and re-splices a managed region, so
/// an edit to either would be lost. An edit outside the markers is the
/// project's own and survives, which is why the region is compared by its
/// own hash rather than the host file's.
///
/// # Errors
///
/// Any I/O error reading a destination.
fn conflicts_at(target: &Utf8Path, installed: &Installed) -> Result<Vec<Conflict>, AppError> {
    let mut conflicts = Vec::new();
    let mut push = |path: &Utf8Path, kind| {
        conflicts.push(Conflict {
            path: path.to_string(),
            kind,
        });
    };
    for (destination, recorded) in &installed.managed {
        let file = target.join(destination);
        if !file.is_file() {
            push(destination, ConflictKind::MissingManagedFile);
            continue;
        }
        if sha256_file(&file)? != *recorded {
            push(destination, ConflictKind::EditedManagedFile);
        }
    }
    for (path, recorded) in &installed.integration {
        let full = target.join(path);
        if !full.is_file() {
            push(path, ConflictKind::MissingIntegrationHost);
            continue;
        }
        let (begin, end) = markers_for(path.as_str());
        let host = std::fs::read_to_string(&full)?;
        match crate::domain::marker::block_hash_with(&host, begin, end) {
            Some(present) if present == *recorded => {}
            _ => push(path, ConflictKind::EditedManagedBlock),
        }
    }
    Ok(conflicts)
}

/// Every destination the candidate lands that the landing would refuse,
/// beyond what the record already names.
fn unattributed(
    target: &Utf8Path,
    candidate: &crate::candidate::Candidate,
    known: &[Conflict],
) -> Vec<Conflict> {
    let recorded = crate::services::installer::recorded(target);
    let mut found = Vec::new();
    for destination in &candidate.destinations {
        let Ok(held) = std::fs::read(target.join(&destination.path)) else {
            continue;
        };
        if held == destination.bytes
            || crate::landing::apply::vouches(
                &recorded,
                &destination.path,
                destination.ownership,
                &held,
            )
            || known
                .iter()
                .any(|conflict| conflict.path == destination.path.as_str())
        {
            continue;
        }
        found.push(Conflict {
            path: destination.path.to_string(),
            kind: ConflictKind::Unattributed,
        });
    }
    found
}

/// The options a reinstall carries.
///
/// No flag: the reinstall carries the recorded declarations forward, and
/// the project's own declaration file is adopted, so the reinstall reads
/// it rather than replacing it.
fn reinstall_options(
    target: &Utf8Path,
    profile: ProfileId,
    options: &UpgradeOptions,
) -> InitOptions {
    let staged = options.staged.as_ref();
    InitOptions {
        target: target.to_path_buf(),
        profile,
        apply: false,
        dry_run: true,
        docs_scratch: staged.map(|choices| choices.docs_scratch.clone()),
        reserve: staged
            .map(|choices| choices.reserve.clone())
            .unwrap_or_default(),
        writing_style: staged.map(|choices| choices.writing_style.clone()),
        acquisition: staged.and_then(|choices| choices.acquisition.clone()),
        replace: options.replace.clone(),
    }
}

/// Report what the landing took back.
///
/// The removals are the landing's own, taken only where the record
/// vouched for the bytes it removed. This only says what happened.
fn report_removals(removed: &[String], outcome: &mut UpgradeOutcome) {
    for raw in removed {
        outcome
            .lines
            .push(format!("removed managed file no longer owned: {raw}"));
    }
}

/// Upgrade an installed instance to this binary's version.
///
/// # Errors
///
/// [`AppError::Violations`] when conflicts block the upgrade or removals
/// remain unfinished, [`AppError::Refused`] when the binary is older than
/// the instance or the reinstall refuses, and manifest errors when the
/// record cannot be read.
pub fn upgrade(options: &UpgradeOptions) -> Result<UpgradeOutcome, AppError> {
    if !options.target.is_absolute() {
        return Err(AppError::Usage("target must be absolute".to_string()));
    }
    if !options.target.is_dir() {
        return Err(AppError::Usage(format!(
            "unresolved target: {}",
            options.target
        )));
    }
    let target = Utf8PathBuf::from_path_buf(std::fs::canonicalize(&options.target)?)
        .map_err(|p| AppError::Usage(format!("target is not UTF-8: {}", p.display())))?;

    // An apply holds the target for the whole run, the observation below
    // included. Without it, a second run could read the tree another
    // landing is halfway through and report that as drift rather than
    // naming the holder.
    let held = if options.dry_run {
        None
    } else {
        Some(crate::landing::lock::hold(&target)?)
    };

    let installed = read_installed(&target)?;
    // The version landed is the version running. The operator chose which
    // binary to install, and that binary projects only itself.
    let new = CanonVersion::current();
    let old = installed.version;
    let mut outcome = UpgradeOutcome::default();

    if old > new {
        return Err(AppError::Refused(format!(
            "sdd {new} is older than the installed canon {old}; upgrade sdd"
        )));
    }

    if let Some(staged) = options.staged.as_ref()
        && staged.profile != installed.profile
    {
        return Err(AppError::Refused(format!(
            "the stage projects the {} profile and {target} records {}; moving a profile is a migration to ask for deliberately",
            staged.profile, installed.profile
        )));
    }
    outcome.from = Some(old);
    let reinstall = reinstall_options(&target, installed.profile, options);
    outcome.retired_citations = retired_citations(&target, &installed);

    // The conflict scan comes before the version shortcut. A managed file
    // edited in a current instance is exactly the drift this verb serves,
    // and a shortcut that reported success first would leave the one route
    // to it unable to do its job.
    let conflicts = scan(&target, &installed, &reinstall, &options.replace)?;
    outcome.conflicts.clone_from(&conflicts);
    let unnamed: Vec<&Conflict> = conflicts
        .iter()
        .filter(|conflict| !options.replace.contains(&conflict.path))
        .collect();
    if !unnamed.is_empty() {
        outcome.failures += unnamed.len();
        outcome
            .lines
            .extend(unnamed.iter().map(|conflict| conflict.line()));
        report_citations(&mut outcome);
        return Ok(outcome);
    }

    if old == new
        && installed.schema_current
        && options.replace.is_empty()
        && options.staged.is_none()
    {
        outcome.lines.push(format!("OK already at {new}"));
        report_citations(&mut outcome);
        return Ok(outcome);
    }

    let save_root = save_root(&target, &options.replace, !options.dry_run)?;
    if options.dry_run {
        preview(
            &target,
            options,
            &reinstall,
            save_root.as_deref(),
            &mut outcome,
        )?;
        report_citations(&mut outcome);
        return Ok(outcome);
    }

    // Every named conflict's bytes are kept before the first write, so the
    // answer "discard" and the answer "keep" lose nothing either way.
    keep_named(
        &target,
        &options.replace,
        save_root.as_deref(),
        &mut outcome,
    )?;

    let reinstalled = init_holding(
        held,
        &InitOptions {
            apply: true,
            dry_run: false,
            ..reinstall
        },
        // The upgrade already classified the target; the reinstall is its
        // own act rather than a second landing decision.
        crate::landing::classify::Intent::Reconcile,
    )
    .map_err(|error| {
        AppError::Refused(format!(
            "upgrade aborted during reinstall from {old} to {new}: {error}"
        ))
    })?;
    // The reinstall's destination list is noise here, and its notes are
    // not: a seed that did not land because the project already holds the
    // destination is something the operator must hear about once.
    let removed = reinstalled.removed.clone();
    outcome.lines.extend(
        reinstalled
            .lines
            .into_iter()
            .filter(|line| line.starts_with("note:")),
    );

    finish(&target, &installed, &removed, old, new, &mut outcome);
    Ok(outcome)
}

/// Every conflict the target holds, and a refusal for a name that is none.
///
/// The record's own conflicts come first, then every destination the
/// candidate lands that holds bytes no record vouches for, because the
/// landing would refuse those too.
fn scan(
    target: &Utf8Path,
    installed: &Installed,
    reinstall: &InitOptions,
    named: &[String],
) -> Result<Vec<Conflict>, AppError> {
    let (conflicts, candidate) = conflicts_with_candidate(target, installed, reinstall)?;
    // A name is an answer to a question the scan asked. One that names no
    // conflict answers nothing, and taking it would make typing look like
    // consent to overwrite a file nobody looked at. One the landing would
    // refuse resolves nothing either, so it is refused here, before a copy
    // is saved, rather than by the landing after.
    for name in named {
        if !conflicts.iter().any(|conflict| &conflict.path == name) {
            return Err(AppError::Usage(format!(
                "--replace {name} names no conflict; the conflicts are the destinations this run lists"
            )));
        }
        if let Some((component, kind)) =
            crate::landing::apply::refuses_named(target, &candidate, name)
        {
            return Err(AppError::Refused(format!(
                "--replace {name} names a destination the landing cannot write: {component} {kind}; move it aside, then run this again"
            )));
        }
    }
    Ok(conflicts)
}

/// Every conflict the target holds, with the candidate that judged them.
fn conflicts_with_candidate(
    target: &Utf8Path,
    installed: &Installed,
    reinstall: &InitOptions,
) -> Result<(Vec<Conflict>, crate::candidate::Candidate), AppError> {
    let mut conflicts = conflicts_at(target, installed)?;
    let candidate =
        crate::services::installer::candidate_for(target, reinstall).map_err(|error| {
            AppError::Refused(format!(
                "upgrade could not render the candidate for {target}: {error}"
            ))
        })?;
    let unattributed = unattributed(target, &candidate, &conflicts);
    conflicts.extend(unattributed);
    Ok((conflicts, candidate))
}

/// The destinations `sdd upgrade --replace` accepts after the stage that
/// recorded `staged`, by the check that verb applies to each name.
///
/// Empty where no instance is recorded, because `sdd init` takes no name,
/// and wherever the upgrade refuses before it scans: the front gate that
/// classifies the target, an instance newer than this binary, or a stage
/// of another profile.
///
/// # Errors
///
/// Whatever rendering the candidate refuses.
pub(crate) fn replaceable(staged: &crate::stage::StagedChoices) -> Result<Vec<String>, AppError> {
    let target = &staged.target;
    if crate::commands::front::serves(crate::landing::classify::Intent::Upgrade, target).is_err() {
        return Ok(Vec::new());
    }
    let Ok(installed) = read_installed(target) else {
        return Ok(Vec::new());
    };
    if installed.version > CanonVersion::current() || staged.profile != installed.profile {
        return Ok(Vec::new());
    }
    let options = UpgradeOptions {
        target: target.clone(),
        dry_run: true,
        staged: Some(staged.clone()),
        replace: Vec::new(),
    };
    let reinstall = reinstall_options(target, installed.profile, &options);
    let (conflicts, candidate) = conflicts_with_candidate(target, &installed, &reinstall)?;
    Ok(conflicts
        .into_iter()
        .filter(|conflict| {
            crate::landing::apply::refuses_named(target, &candidate, &conflict.path).is_none()
        })
        .map(|conflict| conflict.path)
        .collect())
}

/// What a dry run reports: the move, each named conflict and where its
/// bytes would go, and the notes the landing would print.
fn preview(
    target: &Utf8Path,
    options: &UpgradeOptions,
    reinstall: &InitOptions,
    save_root: Option<&Utf8Path>,
    outcome: &mut UpgradeOutcome,
) -> Result<(), AppError> {
    let new = CanonVersion::current();
    let old = outcome.from.unwrap_or(new);
    // At the same canon version the work is the record's schema alone, or
    // the repair of what the operator named, so saying "upgrade X to X"
    // would describe nothing.
    if old != new {
        outcome
            .lines
            .push(format!("DRY RUN upgrade {old} to {new}"));
    } else if options.replace.is_empty() {
        outcome
            .lines
            .push(format!("DRY RUN migrate the record of {new}"));
    } else {
        outcome.lines.push(format!("DRY RUN repair {new}"));
    }
    for named in &options.replace {
        let saved_to = target
            .join(named)
            .is_file()
            .then(|| save_root.map(|root| root.join(named)))
            .flatten();
        outcome.lines.push(saved_to.as_ref().map_or_else(
            || format!("DRY RUN write {named}, which the target lacks"),
            |saved| format!("DRY RUN save {named} to {saved}, then replace it"),
        ));
        outcome.replaced.push(Replaced {
            path: named.clone(),
            saved_to,
        });
    }
    // A seed the landing will not write, and a file it cannot account for,
    // are what a dry run exists to show.
    let preview =
        init(reinstall, crate::landing::classify::Intent::Reconcile).map_err(|error| {
            AppError::Refused(format!(
                "upgrade could not be previewed from {old} to {new}: {error}"
            ))
        })?;
    outcome.lines.extend(
        preview
            .lines
            .into_iter()
            .filter(|line| line.starts_with("note:")),
    );
    Ok(())
}

/// Print every retired citation, one line each.
fn report_citations(outcome: &mut UpgradeOutcome) {
    for citation in &outcome.retired_citations {
        outcome.lines.push(format!(
            "retired rule ID cited: {}:{} `{}`; re-point it to the rule the changelog names",
            citation.path, citation.line, citation.id
        ));
    }
}

/// Where the bytes of a replaced destination are kept.
///
/// Under the docs scratch the instance declares, or that its variable
/// names, in `replaced/`. Where neither names one, under this tool's state
/// root, in a directory of this run's own. Never inside a stage, so
/// cleaning a stage never deletes a saved copy.
///
/// SATISFIES staging:a-replacement-keeps-the-bytes-it-replaces
///
/// A run that writes claims its state-root directory exclusively, so two
/// runs in one second never share one; a preview only names where it would.
fn save_root(
    target: &Utf8Path,
    named: &[String],
    claim: bool,
) -> Result<Option<Utf8PathBuf>, AppError> {
    if named.is_empty() {
        return Ok(None);
    }
    let env = crate::domain::paths::UserEnv::from_process();
    let recorded = crate::services::installer::recorded_field(target, "docs_scratch")
        .and_then(|value| value.as_str().map(Utf8PathBuf::from))
        .filter(|path| !path.as_str().is_empty());
    let scratch = env.docs_scratch.clone().map(Utf8PathBuf::from).or(recorded);
    if let Some(scratch) = scratch {
        return Ok(Some(target.join(scratch).join("replaced")));
    }
    let Some(state) = env.state_root() else {
        return Ok(None);
    };
    let runs = state
        .path
        .join("snapshots")
        .join(crate::stage::stage_name(target));
    let stamp = jiff::Timestamp::now()
        .strftime("%Y%m%dT%H%M%SZ")
        .to_string();
    let run = if claim {
        crate::adapters::fs::claim_run_dir(&runs, &stamp)?
    } else {
        runs.join(stamp)
    };
    Ok(Some(run.join("replaced")))
}

/// Keep every named destination's bytes, and report where each went.
fn keep_named(
    target: &Utf8Path,
    named: &[String],
    root: Option<&Utf8Path>,
    outcome: &mut UpgradeOutcome,
) -> Result<(), AppError> {
    for name in named {
        let saved_to = save(target, name, root)?;
        if let Some(saved) = &saved_to {
            outcome.lines.push(format!("saved {name} to {saved}"));
        }
        outcome.replaced.push(Replaced {
            path: name.clone(),
            saved_to,
        });
    }
    Ok(())
}

/// Keep one destination's bytes before it is replaced.
///
/// A copy already there with other bytes is somebody's earlier answer, and
/// writing over it would lose the bytes this step exists to keep.
fn save(
    target: &Utf8Path,
    named: &str,
    root: Option<&Utf8Path>,
) -> Result<Option<Utf8PathBuf>, AppError> {
    let held = match std::fs::read(target.join(named)) {
        Ok(held) => held,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(AppError::Io(source)),
    };
    let Some(root) = root else {
        return Err(AppError::Refused(format!(
            "no docs scratch and no state root resolve, so the bytes of {named} have nowhere to be kept; set XDG_STATE_HOME or declare a docs scratch"
        )));
    };
    let saved = root.join(named);
    match std::fs::read(&saved) {
        Ok(existing) if existing == held => return Ok(Some(saved)),
        Ok(_) => {
            return Err(AppError::Refused(format!(
                "{saved} already keeps other bytes of {named}; move it aside, then run this again"
            )));
        }
        Err(_) => {}
    }
    crate::adapters::fs::write_file(&saved, &held)?;
    Ok(Some(saved))
}

/// Every rule ID the instance's own specs define, read from the spec
/// directory under its documentation root.
fn instance_rule_ids(target: &Utf8Path, docs_root: &str) -> std::collections::BTreeSet<String> {
    let mut ids = std::collections::BTreeSet::new();
    let specs = target.join(docs_root).join("specs");
    if let Ok(entries) = specs.read_dir_utf8() {
        for entry in entries.filter_map(Result::ok) {
            if let Ok(text) = std::fs::read_to_string(entry.path()) {
                ids.extend(crate::embedded::rule_ids_in(&text));
            }
        }
    }
    ids
}

/// Every citation of a rule ID this binary no longer defines.
///
/// A citation is an inline-code token in a domain this binary defines
/// rules in, naming a rule it does not define. A rule heading is a
/// definition rather than a citation, a decision record is judged on its
/// `Enforced by` line alone, because its prose may describe a retirement,
/// and a file this tool manages is about to be replaced, so none of those
/// is listed. The changelog is history and is never re-pointed, a token
/// this binary's own specs carry is an example it ships, and a rule the
/// instance's own specs define is live wherever this binary once retired
/// a rule of the same name.
///
/// SATISFIES staging:an-upgrade-lists-every-retired-citation-first
fn retired_citations(target: &Utf8Path, installed: &Installed) -> Vec<Citation> {
    let defined = crate::embedded::spec_rule_ids();
    let domains: std::collections::BTreeSet<&str> = defined
        .iter()
        .filter_map(|id| id.split_once(':').map(|(domain, _)| domain))
        .collect();
    // A token this binary's own specs carry is an example it ships, such as
    // a scenario showing a dead citation, never a retirement it caused.
    let shipped: std::collections::BTreeSet<String> = crate::embedded::spec_texts()
        .flat_map(|text| {
            text.lines()
                .flat_map(crate::gates::adr_cites_a_live_rule::cited_ids)
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .collect();
    // A project that keeps a spec domain of the same name, such as its own
    // `staging`, defines its rules there, and a citation of one is live.
    let local = instance_rule_ids(target, &installed.docs_root);
    let managed: Vec<&Utf8PathBuf> = installed.managed.iter().map(|(path, _)| path).collect();
    let decisions = Utf8Path::new(&installed.docs_root).join("decisions");
    let mut found = Vec::new();
    let walker = walkdir::WalkDir::new(target)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !(entry.depth() > 0
                && entry.file_type().is_dir()
                && (crate::gates::PRUNED_DIRS.contains(&name.as_ref())
                    || name == crate::domain::paths::INSTANCE_DIR
                    || crate::domain::paths::DOCS_SCRATCH_LEAVES.contains(&name.as_ref())))
        });
    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(path) = entry.path().to_str() else {
            continue;
        };
        let Ok(relative) = Utf8Path::new(path).strip_prefix(target) else {
            continue;
        };
        if relative.extension() != Some("md")
            || relative == "CHANGELOG.md"
            || managed.iter().any(|held| *held == relative)
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let record = relative.starts_with(&decisions);
        for (index, line) in text.lines().enumerate() {
            if line.starts_with("### `") || (record && !line.starts_with("Enforced by ")) {
                continue;
            }
            for id in crate::gates::adr_cites_a_live_rule::cited_ids(line) {
                let domain = id.split_once(':').map_or(id, |(domain, _)| domain);
                if domains.contains(domain)
                    && !defined.contains(id)
                    && !shipped.contains(id)
                    && !local.contains(id)
                {
                    found.push(Citation {
                        path: relative.to_owned(),
                        line: index + 1,
                        id: id.to_string(),
                    });
                }
            }
        }
    }
    found
}

fn finish(
    target: &Utf8Path,
    installed: &Installed,
    removed: &[String],
    old: CanonVersion,
    new: CanonVersion,
    outcome: &mut UpgradeOutcome,
) {
    report_removals(removed, outcome);

    let local_ids = instance_rule_ids(target, &installed.docs_root);
    let upstream_only: Vec<String> = crate::embedded::spec_rule_ids()
        .difference(&local_ids)
        .cloned()
        .collect();
    if !upstream_only.is_empty() {
        outcome
            .lines
            .push("upstream rule IDs not present locally:".to_string());
        for id in upstream_only {
            outcome.lines.push(format!("  {id}"));
        }
    }

    if outcome.failures > 0 {
        outcome.lines.push(format!(
            "FAIL upgraded {old} to {new} with unfinished removals above"
        ));
    } else if old == new {
        outcome
            .lines
            .push(format!("OK migrated the record of {new}"));
    } else {
        outcome.lines.push(format!("OK upgraded {old} to {new}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SATISFIES staging:a-replacement-keeps-the-bytes-it-replaces
    #[test]
    fn two_saves_in_one_second_keep_both_copies() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let target = root.join("target");
        let runs = root.join("snapshots");
        let stamp = "20261009T213000Z";

        crate::adapters::fs::write_file(&target.join("AGENTS.md"), b"first").unwrap();
        let first = crate::adapters::fs::claim_run_dir(&runs, stamp).unwrap();
        let kept = save(&target, "AGENTS.md", Some(&first)).unwrap().unwrap();

        crate::adapters::fs::write_file(&target.join("AGENTS.md"), b"second").unwrap();
        let second = crate::adapters::fs::claim_run_dir(&runs, stamp).unwrap();
        let again = save(&target, "AGENTS.md", Some(&second)).unwrap().unwrap();

        assert_ne!(kept, again);
        assert_eq!(std::fs::read(&kept).unwrap(), b"first");
        assert_eq!(std::fs::read(&again).unwrap(), b"second");
    }
}
