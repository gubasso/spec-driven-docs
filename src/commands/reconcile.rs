//! `reconcile` subcommand: runtime-shape.
//!
//! One lifecycle over one plan. This phase lands the first verb: observe
//! the target, resolve the destination release, compute the plan, and
//! print it. It writes nothing into the target, so an operator can read
//! what would happen before anything can.

use camino::{Utf8Path, Utf8PathBuf};

use crate::cli::reconcile::{PlanArgs, ReconcileArgs, ReconcileCommand};
use crate::context::AppContext;
use crate::domain::ownership::Sha256;
use crate::domain::paths::UserEnv;
use crate::error::AppError;
use crate::output;
use crate::plan::observe::observe;
use crate::plan::planner::{Inputs, plan as compute};
use crate::plan::readiness::Readiness;
use crate::plan::{Plan, decision};
use crate::release::crates_io::CratesIoResolver;
use crate::release::embedded::EmbeddedReleaseBundle;
use crate::release::{Provenance, ReleaseBundle, ReleaseResolver, Role, Selector};

/// Run one reconcile verb.
///
/// # Errors
///
/// [`AppError::Usage`] for a target the arguments cannot mean and for a
/// decision answer the plan does not offer, and the resolver's refusals.
pub fn run(ctx: &AppContext, args: ReconcileArgs) -> Result<(), AppError> {
    match args.command {
        ReconcileCommand::Plan(plan) => run_plan(ctx, &plan),
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

fn run_plan(ctx: &AppContext, args: &PlanArgs) -> Result<(), AppError> {
    let target = resolve_target(ctx, &args.target)?;
    let selections =
        decision::parse(&args.set).map_err(|error| AppError::Usage(error.to_string()))?;
    let wanted = selector(&args.to)?;

    let (bundle, version, provenance, checksum, yanked): (
        Box<dyn ReleaseBundle>,
        String,
        String,
        Option<Sha256>,
        bool,
    ) = match &wanted {
        Selector::Embedded => (
            Box::new(EmbeddedReleaseBundle::new()),
            crate::domain::version::CanonVersion::current().to_string(),
            "native".to_string(),
            None,
            false,
        ),
        chosen => {
            let cache = UserEnv::from_process()
                .user_paths()
                .ok_or_else(|| AppError::Usage("no cache root resolves".to_string()))?
                .bundle_cache
                .path;
            let resolved = CratesIoResolver::new(&cache)
                .offline(args.offline)
                .resolve(chosen)?;
            let manifest = resolved.bundle.manifest()?;
            let provenance = match manifest.provenance {
                Provenance::Native => "native",
                Provenance::LegacyAdapted => "legacy-adapted",
            };
            (
                resolved.bundle,
                resolved.version.to_string(),
                provenance.to_string(),
                resolved.registry_checksum,
                resolved.yanked,
            )
        }
    };

    let observation = observe(&target)?;
    let declaration = bundle.declaration()?;
    let manifest = bundle.manifest()?;
    let candidate: std::collections::BTreeMap<String, Sha256> = manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.role == Role::Payload)
        .map(|artifact| (artifact.path.clone(), artifact.sha256.clone()))
        .collect();
    // The baseline comes from the recorded release. Where that is the
    // destination, the candidate is the baseline; where it is not, this
    // phase does not fetch it, and the plan says so rather than guessing.
    let recorded_is_destination = observation
        .installation
        .as_ref()
        .is_some_and(|installed| installed.canon_version.to_string() == version);
    let baseline = recorded_is_destination.then(|| candidate.clone());

    let inputs = Inputs {
        observation: &observation,
        declaration: &declaration,
        candidate: &candidate,
        baseline: baseline.as_ref(),
        selector: args.to.clone(),
        release: version,
        release_sha256: manifest.payload_sha256,
        provenance,
        registry_checksum: checksum,
        yanked,
        selections: &selections,
        now: jiff::Timestamp::now().to_string(),
    };
    let computed = compute(&inputs);
    // An answer is checked against the plan it was given for, so a
    // decision the plan stopped offering is a usage error rather than a
    // selection nothing reads.
    decision::validate(&computed.decisions, &selections)
        .map_err(|error| AppError::Usage(error.to_string()))?;

    if args.json {
        return output::json(&computed);
    }
    render(&computed);
    Ok(())
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
