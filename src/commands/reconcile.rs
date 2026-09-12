//! `reconcile` subcommand: runtime-shape.
//!
//! One lifecycle over one plan. `plan` computes and stores it, `show`
//! renders a stored one, and `apply` executes exactly that plan or refuses
//! because its inputs moved. Every verb here takes the target lock, so two
//! writers cannot interleave over one repository.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};

use crate::cli::reconcile::{ApplyArgs, PlanArgs, ReconcileArgs, ReconcileCommand, ShowArgs};
use crate::context::AppContext;
use crate::domain::ownership::Sha256;
use crate::domain::paths::UserEnv;
use crate::error::AppError;
use crate::output;
use crate::plan::apply::{Request, apply as execute};
use crate::plan::observe::observe;
use crate::plan::planner::{Inputs, plan as compute};
use crate::plan::readiness::Readiness;
use crate::plan::store::{Result as ApplyResult, Store};
use crate::plan::{Plan, decision};
use crate::release::crates_io::CratesIoResolver;
use crate::release::embedded::EmbeddedReleaseBundle;
use crate::release::{Provenance, ReleaseBundle, ReleaseResolver, Role, Selector};
use crate::transaction::lock::Lock;

/// Run one reconcile verb.
///
/// # Errors
///
/// [`AppError::Usage`] for a target the arguments cannot mean and for a
/// decision answer the plan does not offer, [`AppError::Busy`] when
/// another process holds the target, and the resolver's refusals.
pub fn run(ctx: &AppContext, args: ReconcileArgs) -> Result<(), AppError> {
    match args.command {
        ReconcileCommand::Plan(plan) => run_plan(ctx, &plan),
        ReconcileCommand::Show(show) => run_show(&show),
        ReconcileCommand::Apply(apply) => run_apply(ctx, &apply),
    }
}

fn resolve_target(ctx: &AppContext, target: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    if target.is_absolute() {
        return Ok(target.to_owned());
    }
    if target == "." {
        return Ok(ctx.cwd.clone());
    }
    Err(AppError::Usage("target must be absolute or .".to_string()))
}

/// What the caller asked for.
fn selector(value: &str) -> Result<Selector, AppError> {
    match value {
        "embedded" => Ok(Selector::Embedded),
        "latest" => Ok(Selector::Latest),
        version => version.parse().map(Selector::Exact).map_err(|_| {
            AppError::Usage(format!(
                "--to takes embedded, latest, or a semantic version; {version} is none of those"
            ))
        }),
    }
}

/// Where this tool keeps state that outlives a command.
fn state_root() -> Result<Utf8PathBuf, AppError> {
    Ok(UserEnv::from_process()
        .state_root()
        .ok_or_else(|| AppError::Usage("no state root resolves".to_string()))?
        .path)
}

/// The lock one target takes, keyed by where it is.
///
/// A digest rather than the path itself: a lock file named after a
/// repository would put a person's directory layout in the state root, and
/// two targets whose paths differ only in case would collide.
fn target_lock(target: &Utf8Path) -> Result<Utf8PathBuf, AppError> {
    let key = Sha256::of(target.as_str().as_bytes());
    Ok(state_root()?.join("locks").join(format!("{key}.lock")))
}

/// One release, read through the seam.
struct Release {
    bundle: Box<dyn ReleaseBundle>,
    version: String,
    provenance: String,
    checksum: Option<Sha256>,
    yanked: bool,
}

fn read_release(to: &str, offline: bool) -> Result<Release, AppError> {
    match selector(to)? {
        Selector::Embedded => Ok(Release {
            bundle: Box::new(EmbeddedReleaseBundle::new()),
            version: crate::domain::version::CanonVersion::current().to_string(),
            provenance: "native".to_string(),
            checksum: None,
            yanked: false,
        }),
        chosen => {
            let cache = UserEnv::from_process()
                .user_paths()
                .ok_or_else(|| AppError::Usage("no cache root resolves".to_string()))?
                .bundle_cache
                .path;
            let resolved = CratesIoResolver::new(&cache)
                .offline(offline)
                .resolve(&chosen)?;
            let manifest = resolved.bundle.manifest()?;
            let provenance = match manifest.provenance {
                Provenance::Native => "native",
                Provenance::LegacyAdapted => "legacy-adapted",
            };
            Ok(Release {
                bundle: resolved.bundle,
                version: resolved.version.to_string(),
                provenance: provenance.to_string(),
                checksum: resolved.registry_checksum,
                yanked: resolved.yanked,
            })
        }
    }
}

/// Compute one plan against one target.
fn compute_plan(
    target: &Utf8Path,
    to: &str,
    offline: bool,
    selections: &decision::Selections,
    release: &Release,
) -> Result<Plan, AppError> {
    let _ = offline;
    let observation = observe(target)?;
    let declaration = release.bundle.declaration()?;
    let manifest = release.bundle.manifest()?;
    let candidate: BTreeMap<String, Sha256> = manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.role == Role::Payload)
        .map(|artifact| (artifact.path.clone(), artifact.sha256.clone()))
        .collect();
    // The baseline comes from the recorded release. Where that is the
    // destination, the candidate is the baseline; where it is not, this
    // engine does not fetch it, and the plan says so rather than guessing.
    let recorded_is_destination = observation
        .installation
        .as_ref()
        .is_some_and(|installed| installed.canon_version.to_string() == release.version);
    let baseline = recorded_is_destination.then(|| candidate.clone());
    Ok(compute(&Inputs {
        observation: &observation,
        declaration: &declaration,
        candidate: &candidate,
        baseline: baseline.as_ref(),
        selector: to.to_string(),
        release: release.version.clone(),
        release_sha256: manifest.payload_sha256,
        provenance: release.provenance.clone(),
        registry_checksum: release.checksum.clone(),
        yanked: release.yanked,
        selections,
        now: jiff::Timestamp::now().to_string(),
    }))
}

/// Every byte one plan will write, by digest.
fn blobs_for(
    plan: &Plan,
    bundle: &dyn ReleaseBundle,
) -> Result<BTreeMap<Sha256, Vec<u8>>, AppError> {
    let mut blobs = BTreeMap::new();
    for operation in &plan.operations {
        let Some(after) = operation.after() else {
            continue;
        };
        if blobs.contains_key(after) {
            continue;
        }
        // A plan carries every byte it will write, so an apply never has
        // to go back to a bundle it may no longer be able to read.
        blobs.insert(after.clone(), bundle.blob(after)?);
    }
    Ok(blobs)
}

fn run_plan(ctx: &AppContext, args: &PlanArgs) -> Result<(), AppError> {
    let target = resolve_target(ctx, &args.target)?;
    let selections =
        decision::parse(&args.set).map_err(|error| AppError::Usage(error.to_string()))?;
    let release = read_release(&args.to, args.offline)?;

    // Planning holds the target shared: several readers may plan at once,
    // and none of them may overlap an apply.
    let _lock = Lock::shared(&target_lock(&target)?, "reconcile plan")?;
    let computed = compute_plan(&target, &args.to, args.offline, &selections, &release)?;
    decision::validate(&computed.decisions, &selections)
        .map_err(|error| AppError::Usage(error.to_string()))?;

    let store = Store::new(&state_root()?);
    let _ = store.prune(jiff::Timestamp::now(), Some(&computed.identity.plan_id));
    // Identical inputs while an executable plan exists reuse it, so an
    // operator who plans twice approves one thing.
    if !store.holds(&computed.identity.plan_id) {
        let blobs = blobs_for(&computed, release.bundle.as_ref())?;
        store.put(&computed, &blobs)?;
    }

    if args.json {
        return output::json(&computed);
    }
    render(&computed);
    output::line(format!(
        "apply it with: sdd reconcile apply {}",
        computed.identity.plan_id
    ));
    Ok(())
}

fn run_show(args: &ShowArgs) -> Result<(), AppError> {
    let store = Store::new(&state_root()?);
    if let Ok(plan) = store.get(&args.plan_id) {
        if args.json {
            return output::json(&plan);
        }
        output::line("executable plan");
        render(&plan);
        return Ok(());
    }
    let result = store.latest_result(&args.plan_id).ok_or_else(|| {
        AppError::Refused(format!(
            "no plan and no result carry the id {}; run 'sdd reconcile plan' again",
            args.plan_id
        ))
    })?;
    if args.json {
        return output::json(&result);
    }
    output::line("latest result");
    output::line(format!("disposition: {:?}", result.disposition));
    output::line(format!("reason: {}", result.reason));
    Ok(())
}

fn run_apply(ctx: &AppContext, args: &ApplyArgs) -> Result<(), AppError> {
    let target = resolve_target(ctx, &args.target)?;
    let store = Store::new(&state_root()?);
    let stored = store.get(&args.plan_id)?;

    // Apply holds the target alone, through observation, execution, and
    // the result write.
    let _lock = Lock::exclusive(&target_lock(&target)?, "reconcile apply")?;
    let selections: decision::Selections = stored
        .decisions
        .iter()
        .filter_map(|decision| {
            decision
                .selected
                .as_ref()
                .map(|answer| (decision.id.clone(), answer.clone()))
        })
        .collect();
    let release = read_release(&stored.desired_state.selector, true).or_else(|_| {
        // A destination the cache no longer holds is a moved input, which
        // the revalidation below reports rather than a fetch papering over.
        read_release(&stored.desired_state.selector, false)
    })?;
    let recomputed = compute_plan(
        &target,
        &stored.desired_state.selector,
        true,
        &selections,
        &release,
    )?;

    let result = execute(&Request {
        store: &store,
        target: &target,
        stored: &stored,
        recomputed: &recomputed,
        now: jiff::Timestamp::now().to_string(),
    })?;
    if args.json {
        return output::json(&result);
    }
    render_result(&result);
    Ok(())
}

fn render_result(result: &ApplyResult) {
    output::line(format!("{:?}: {}", result.disposition, result.reason));
    for operation in &result.operations {
        output::line(format!("  {} {}", operation.kind, operation.path));
    }
    for postcondition in &result.postconditions {
        let word = if postcondition.held { "held" } else { "FAILED" };
        output::line(format!("  {word}: {}", postcondition.id));
    }
}

fn render(plan: &Plan) {
    output::line(format!(
        "{} toward {} ({})",
        plan.classification, plan.desired_state.release, plan.desired_state.selector
    ));
    output::line(format!("plan {}", plan.identity.plan_id));
    let word = match plan.readiness {
        Readiness::Ready => "ready",
        Readiness::NeedsDecision => "needs a decision",
        Readiness::Blocked => "blocked",
    };
    output::line(format!("readiness: {word}"));
    if !plan.findings.is_empty() {
        output::line(format!("findings: {}", plan.findings.len()));
        for found in &plan.findings {
            output::line(format!(
                "  {} {}: {}",
                found.rule, found.path, found.statement
            ));
        }
    }
    if !plan.style_candidates.is_empty() {
        output::line(format!(
            "style candidates: {} (named, never judged)",
            plan.style_candidates.len()
        ));
    }
    output::line(format!("operations: {}", plan.operations.len()));
    for operation in &plan.operations {
        output::line(format!("  {} {}", operation.kind(), operation.path()));
    }
    for precondition in &plan.preconditions {
        if precondition.evaluation.is_satisfied() {
            continue;
        }
        output::line(format!(
            "  blocked by {}: {}",
            precondition.id, precondition.statement
        ));
    }
    for decision in &plan.decisions {
        if decision.selected.is_some() {
            continue;
        }
        output::line(format!("  decide --set {}=<answer>", decision.id));
    }
}
