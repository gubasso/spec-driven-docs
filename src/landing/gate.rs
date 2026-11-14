//! The version gate every command passes before its handler runs.
//!
//! Each command, and each action of a command family, carries one class in
//! [`TABLE`]. A gated class compares the version the target's instance
//! record names with this binary's own, and runs, warns, or refuses by the
//! rows the setup spec states, naming the command that fixes a mismatch.
//! The gate reads the record of any schema, never writes, never moves a
//! pin, and never runs `init` or `upgrade` by itself.
//!
//! SATISFIES setup:every-command-has-a-gate-class
//! SATISFIES setup:a-gated-command-names-its-fix

use std::fmt::Write as _;

use camino::{Utf8Path, Utf8PathBuf};

use crate::cli::Commands;
use crate::domain::manifest::{MANIFEST_PATH, Manifest};
use crate::domain::version::CanonVersion;
use crate::error::AppError;
use crate::self_depend::guidance::MOVED;

/// What the gate holds an action to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Needs no target, or must never fail.
    Exempt,
    /// Runs on a target with no instance by design, and before or during an
    /// upgrade. On an instance, an action that writes follows the writing
    /// rows.
    PreLanding,
    /// `upgrade` alone, which keeps its own checks.
    Upgrade,
    /// Reads an instance.
    ReadOnly,
    /// Writes an instance when applied; a preview is read-only.
    Writing,
}

/// When an action writes the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Writes {
    /// Never.
    Never,
    /// Only with `--apply`.
    WithApply,
    /// Always, with no flag.
    Always,
}

/// Where an action finds its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSource {
    /// The `--target` argument, `.` meaning the current folder.
    Flag,
    /// The current folder.
    CurrentFolder,
    /// No target.
    None,
}

/// One row of the class table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// The action, as its command path spells it.
    pub path: &'static str,
    /// Its class.
    pub class: Class,
    /// When it writes the target.
    pub writes: Writes,
    /// Where it finds its target.
    pub target: TargetSource,
}

const fn row(path: &'static str, class: Class, writes: Writes, target: TargetSource) -> Row {
    Row {
        path,
        class,
        writes,
        target,
    }
}

/// Every action of the command tree and its class, in one place.
pub const TABLE: &[Row] = &[
    row(
        "init",
        Class::PreLanding,
        Writes::WithApply,
        TargetSource::Flag,
    ),
    row("verify", Class::ReadOnly, Writes::Never, TargetSource::Flag),
    row(
        "upgrade",
        Class::Upgrade,
        Writes::WithApply,
        TargetSource::Flag,
    ),
    row(
        "gate",
        Class::ReadOnly,
        Writes::Never,
        TargetSource::CurrentFolder,
    ),
    row(
        "hooks",
        Class::Writing,
        Writes::WithApply,
        TargetSource::Flag,
    ),
    row(
        "debt baseline",
        Class::Writing,
        Writes::WithApply,
        TargetSource::Flag,
    ),
    row(
        "debt migrate",
        Class::Writing,
        Writes::WithApply,
        TargetSource::Flag,
    ),
    row(
        "debt tighten",
        Class::Writing,
        Writes::WithApply,
        TargetSource::Flag,
    ),
    row(
        "policy reconcile",
        Class::Writing,
        Writes::WithApply,
        TargetSource::Flag,
    ),
    row(
        "ki list",
        Class::ReadOnly,
        Writes::Never,
        TargetSource::Flag,
    ),
    row("docs", Class::Exempt, Writes::Never, TargetSource::None),
    row("method", Class::Exempt, Writes::Never, TargetSource::None),
    row("spec", Class::Exempt, Writes::Never, TargetSource::None),
    row("template", Class::Exempt, Writes::Never, TargetSource::None),
    row(
        "skill list",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "skill show",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "skill install",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "skill uninstall",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "stage",
        Class::PreLanding,
        Writes::Never,
        TargetSource::Flag,
    ),
    row(
        "stage clean",
        Class::PreLanding,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "stage compare",
        Class::PreLanding,
        Writes::Never,
        TargetSource::Flag,
    ),
    row("status", Class::Exempt, Writes::Never, TargetSource::None),
    // The setup skill reads the environment report while it plans a
    // landing, so a preview runs before an instance exists; an apply on an
    // instance follows the writing rows.
    row(
        "env",
        Class::PreLanding,
        Writes::WithApply,
        TargetSource::Flag,
    ),
    row("lint", Class::PreLanding, Writes::Never, TargetSource::Flag),
    row(
        "lint prove",
        Class::PreLanding,
        Writes::Never,
        TargetSource::Flag,
    ),
    row(
        "migrate inventory",
        Class::PreLanding,
        Writes::Never,
        TargetSource::Flag,
    ),
    row(
        "migrate archive",
        Class::PreLanding,
        Writes::WithApply,
        TargetSource::Flag,
    ),
    row(
        "migrate mark",
        Class::PreLanding,
        Writes::Always,
        TargetSource::Flag,
    ),
    row(
        "migrate status",
        Class::PreLanding,
        Writes::Never,
        TargetSource::Flag,
    ),
    row(
        "migrate close",
        Class::PreLanding,
        Writes::WithApply,
        TargetSource::Flag,
    ),
    row(
        "track status",
        Class::ReadOnly,
        Writes::Never,
        TargetSource::Flag,
    ),
    row(
        "track check",
        Class::ReadOnly,
        Writes::Never,
        TargetSource::Flag,
    ),
    row("doctor", Class::Exempt, Writes::Never, TargetSource::None),
    row(
        "assess",
        Class::PreLanding,
        Writes::Never,
        TargetSource::Flag,
    ),
    row("license", Class::Exempt, Writes::Never, TargetSource::None),
    row(
        "self-manifest",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "self-depend status",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "self-depend add",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "self-depend check",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "self-depend sync",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "self-depend clean",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row(
        "completions",
        Class::Exempt,
        Writes::Never,
        TargetSource::None,
    ),
    row("man", Class::Exempt, Writes::Never, TargetSource::None),
];

/// The row of one action, by its command path.
#[must_use]
pub fn row_of(path: &str) -> Option<&'static Row> {
    TABLE.iter().find(|row| row.path == path)
}

/// One parsed invocation, reduced to what the gate reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// The action's command path.
    pub path: &'static str,
    /// Whether the invocation asked to write.
    pub applied: bool,
    /// The target as the invocation spelled it, where it names one.
    pub target: Option<Utf8PathBuf>,
}

fn invocation(path: &'static str, applied: bool, target: Option<&Utf8Path>) -> Invocation {
    Invocation {
        path,
        applied,
        target: target.map(Utf8Path::to_owned),
    }
}

/// Reduce a parsed command to its action, its write request, and its target.
#[must_use]
pub fn invocation_of(command: &Commands) -> Invocation {
    use crate::cli::debt::DebtVerb;
    use crate::cli::ki::KiCommand;
    use crate::cli::lint::LintCommand;
    use crate::cli::migrate::MigrateCommand;
    use crate::cli::policy::PolicyVerb;
    use crate::cli::self_depend::SelfDependCommand;
    use crate::cli::skill::SkillCommand;
    use crate::cli::stage::StageCommand;
    use crate::cli::track::TrackCommand;

    match command {
        Commands::Init(args) => {
            // A landing from a stage lands in the folder it runs from,
            // unless it names its target.
            let target = args
                .target
                .clone()
                .or_else(|| args.stage.as_ref().map(|_| Utf8PathBuf::from(".")));
            invocation("init", args.apply, target.as_deref())
        }
        Commands::Verify(args) => invocation("verify", false, Some(&args.target)),
        Commands::Upgrade(args) => invocation("upgrade", false, Some(&args.target)),
        Commands::Gate(args) => {
            if args.id.is_some() {
                invocation("gate", false, Some(Utf8Path::new(".")))
            } else {
                // Listing and explaining read the registry alone.
                invocation("gate", false, None)
            }
        }
        Commands::Hooks(args) => invocation("hooks", args.apply, Some(Utf8Path::new(&args.target))),
        Commands::Debt(args) => match &args.verb {
            DebtVerb::Baseline(verb) => invocation("debt baseline", verb.apply, Some(&verb.target)),
            DebtVerb::Migrate(verb) => invocation("debt migrate", verb.apply, Some(&verb.target)),
            DebtVerb::Tighten(verb) => invocation("debt tighten", verb.apply, Some(&verb.target)),
        },
        Commands::Policy(args) => match &args.verb {
            PolicyVerb::Reconcile(verb) => {
                invocation("policy reconcile", verb.apply, Some(&verb.target))
            }
        },
        Commands::Ki(args) => match &args.command {
            KiCommand::List(list) => invocation("ki list", false, Some(&list.target)),
        },
        Commands::Docs(_) => invocation("docs", false, None),
        Commands::Method(_) => invocation("method", false, None),
        Commands::Spec(_) => invocation("spec", false, None),
        Commands::Template(_) => invocation("template", false, None),
        Commands::Skill(args) => match &args.command {
            SkillCommand::List => invocation("skill list", false, None),
            SkillCommand::Show(_) => invocation("skill show", false, None),
            SkillCommand::Install(_) => invocation("skill install", false, None),
            SkillCommand::Uninstall(_) => invocation("skill uninstall", false, None),
        },
        Commands::Stage(args) => match &args.command {
            None => invocation("stage", false, Some(&args.target)),
            Some(StageCommand::Clean(_)) => invocation("stage clean", false, None),
            Some(StageCommand::Compare(compare)) => {
                invocation("stage compare", false, Some(&compare.target))
            }
        },
        Commands::Status(_) => invocation("status", false, None),
        Commands::Env(args) => invocation("env", args.apply.is_some(), Some(&args.target)),
        Commands::Lint(args) => match &args.command {
            None => invocation("lint", false, Some(&args.target)),
            Some(LintCommand::Prove(prove)) => invocation("lint prove", false, Some(&prove.target)),
        },
        Commands::Migrate(args) => match &args.command {
            MigrateCommand::Inventory(verb) => {
                invocation("migrate inventory", false, Some(&verb.target))
            }
            MigrateCommand::Archive(verb) => {
                invocation("migrate archive", verb.apply, Some(&verb.target))
            }
            MigrateCommand::Mark(verb) => invocation("migrate mark", true, Some(&verb.target)),
            MigrateCommand::Status(verb) => invocation("migrate status", false, Some(&verb.target)),
            MigrateCommand::Close(verb) => {
                invocation("migrate close", verb.apply, Some(&verb.target))
            }
        },
        Commands::Track(args) => match &args.command {
            TrackCommand::Status(verb) => invocation("track status", false, Some(&verb.target)),
            TrackCommand::Check(verb) => invocation("track check", false, Some(&verb.target)),
        },
        Commands::Doctor(_) => invocation("doctor", false, None),
        Commands::Assess(args) => invocation("assess", false, Some(&args.target)),
        Commands::License(_) => invocation("license", false, None),
        Commands::SelfManifest => invocation("self-manifest", false, None),
        Commands::SelfDepend(args) => match &args.command {
            SelfDependCommand::Status(_) => invocation("self-depend status", false, None),
            SelfDependCommand::Add(_) => invocation("self-depend add", false, None),
            SelfDependCommand::Check(_) => invocation("self-depend check", false, None),
            SelfDependCommand::Sync(_) => invocation("self-depend sync", false, None),
            SelfDependCommand::Clean(_) => invocation("self-depend clean", false, None),
        },
        Commands::Completions(_) => invocation("completions", false, None),
        Commands::Man => invocation("man", false, None),
    }
}

/// What the gate decided for a check that lets the handler run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Passed {
    /// Nothing to say.
    Quiet,
    /// One line for stderr; the command keeps its own exit code.
    Warn(String),
}

/// What one instance record says about its version, read from any schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    /// The release that wrote the record.
    pub version: CanonVersion,
    /// The record's schema, where it is older than this binary's.
    pub older_schema: Option<u32>,
}

/// Read the version an instance record names, whatever its schema.
///
/// `None` where the target holds no record. A record that cannot be read
/// at all is left to the handler, which reports it.
#[must_use]
pub fn recorded(target: &Utf8Path) -> Option<Result<Recorded, String>> {
    let path = target.join(MANIFEST_PATH);
    if !path.is_file() {
        return None;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => return Some(Err(error.to_string())),
    };
    Some(
        Manifest::parse_any(&text)
            .map(|(manifest, older_schema)| Recorded {
                version: manifest.canon_version,
                older_schema,
            })
            .map_err(|error| error.to_string()),
    )
}

/// The text that names the setup skill reader beside `sdd init`.
const NO_INSTANCE: &str = "This repository has no spec-driven-docs instance. Run `sdd init` to land it. Read the setup skill with `sdd skill show sdd-setup`, and each file it names with `sdd skill show sdd-setup --file <path>`.";

/// The version a pin at `target` names, where a manager file pins this tool.
fn pinned(target: &Utf8Path) -> Option<String> {
    crate::self_depend::manager::detect(target)
        .into_iter()
        .find_map(|detected| detected.pin.map(|pin| pin.version.to_string()))
}

/// Whether `left` names an older release than `right`.
fn older(left: &str, right: &str) -> bool {
    let parse = |version: &str| -> Option<Vec<u64>> {
        version
            .split('.')
            .map(|part| part.parse().ok())
            .collect::<Option<Vec<u64>>>()
    };
    matches!((parse(left), parse(right)), (Some(a), Some(b)) if a < b)
}

/// Hold one action to the version of its target's instance.
///
/// # Errors
///
/// [`AppError::Refused`] naming the command that fixes the mismatch, by
/// the rows of `setup:a-gated-command-names-its-fix`.
pub fn require_aligned(row: &Row, applied: bool, target: &Utf8Path) -> Result<Passed, AppError> {
    let writes = match row.writes {
        Writes::Never => false,
        Writes::WithApply => applied,
        Writes::Always => true,
    };
    let Some(record) = recorded(target) else {
        return match row.class {
            Class::ReadOnly | Class::Writing => Err(AppError::Refused(format!(
                "{NO_INSTANCE} (setup:a-gated-command-names-its-fix)"
            ))),
            Class::Exempt | Class::PreLanding | Class::Upgrade => Ok(Passed::Quiet),
        };
    };
    let effective = match (row.class, writes) {
        (Class::Exempt | Class::Upgrade, _) | (Class::PreLanding, false) => {
            return Ok(Passed::Quiet);
        }
        (Class::PreLanding | Class::Writing | Class::ReadOnly, true) => Class::Writing,
        (Class::ReadOnly | Class::Writing, false) => Class::ReadOnly,
    };
    // A record nobody can read is the handler's to report.
    let Ok(record) = record else {
        return Ok(Passed::Quiet);
    };
    let binary = CanonVersion::current().to_string();
    let instance = record.version.to_string();
    if older(&binary, &instance) {
        let mut text = format!(
            "The instance is at sdd {instance} and this sdd is {binary}, which is older. Run the sdd of this project's environment, for example `direnv exec . sdd ...`."
        );
        if let Some(pin) = pinned(target).filter(|pin| older(pin, &instance)) {
            let _ = write!(text, " The pin names {pin}. Move it to {instance}.");
        }
        return Err(AppError::Refused(format!(
            "{text} (setup:a-gated-command-names-its-fix)"
        )));
    }
    if older(&instance, &binary) || record.older_schema.is_some() {
        let schema = record
            .older_schema
            .map(|schema| format!(", recorded in schema {schema},"))
            .unwrap_or_default();
        // A pin moved only where the recorded release is older. A record of
        // an older schema at this release needs the upgrade alone.
        let fix = if older(&instance, &binary) {
            MOVED
        } else {
            "Run `sdd upgrade` to move the record."
        };
        if effective == Class::Writing {
            return Err(AppError::Refused(format!(
                "The instance is at sdd {instance}{schema} and this sdd is {binary}. {fix} Then run this command again. (setup:a-gated-command-names-its-fix)"
            )));
        }
        return Ok(Passed::Warn(format!(
            "warning: the instance is at sdd {instance}{schema} and this sdd is {binary}. {fix} (setup:a-gated-command-names-its-fix)"
        )));
    }
    Ok(Passed::Quiet)
}

/// Gate one parsed command before its handler runs.
///
/// The warning of a passing read goes to stderr, so it never enters a JSON
/// body on stdout.
///
/// # Errors
///
/// Whatever [`require_aligned`] refuses.
pub fn gate(command: &Commands, cwd: &Utf8Path) -> Result<(), AppError> {
    let invocation = invocation_of(command);
    let Some(row) = row_of(invocation.path) else {
        return Ok(());
    };
    let Some(target) = invocation.target else {
        return Ok(());
    };
    if row.class == Class::Exempt || row.target == TargetSource::None {
        return Ok(());
    }
    let target = if target.is_absolute() {
        target
    } else if target == "." {
        cwd.to_owned()
    } else {
        cwd.join(target)
    };
    // A target that is no folder is the handler's to refuse.
    if !target.is_dir() {
        return Ok(());
    }
    if let Passed::Warn(line) = require_aligned(row, invocation.applied, &target)? {
        eprintln!("{line}");
    }
    Ok(())
}
