//! Every control path this tool names, declared once.
//!
//! Two kinds of path meet here and stay apart. A control path is this
//! tool's own: the instance directory, the declaration, the debt files, the
//! hook configuration, the agent digest, the state and cache roots, and the
//! agent skill roots. A destination is what a release projects into a
//! target, and it is versioned data rather than a constant, so it stays
//! with the projection and reaches this module only as a resolved value.
//!
//! The module is otherwise pure. It reads no filesystem. It reads the
//! process environment in exactly one place, [`UserEnv::from_process`], so
//! every resolver below takes the environment as an argument and a test
//! constructs one instead of mutating the process.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::domain::manifest::PlanZone;
use crate::domain::profile::{DocsRoot, ProfileId};

/// The variable naming the invoking user's home directory.
pub const HOME_VAR: &str = "HOME";
/// The variable relocating Claude Code's whole configuration directory.
pub const CLAUDE_CONFIG_DIR_VAR: &str = "CLAUDE_CONFIG_DIR";
/// The variable naming the XDG state base directory.
///
/// Declared here and unread until the state root takes its XDG form. The
/// skills name the two shared gates by one absolute path under the state
/// root, so a state root that moved before the skills stopped spelling it
/// would separate the gates from their readers.
pub const XDG_STATE_HOME_VAR: &str = "XDG_STATE_HOME";
/// The variable naming the XDG cache base directory.
pub const XDG_CACHE_HOME_VAR: &str = "XDG_CACHE_HOME";
/// The variable that names the plan zone.
pub const PLAN_ZONE_VAR: &str = "SDD_PLAN_ZONE";
/// The variable that names the docs scratch.
pub const DOCS_SCRATCH_VAR: &str = "SDD_DOCS_SCRATCH";

/// This tool's directory name under a base directory.
pub const TOOL_DIR: &str = "spec-driven-docs";

/// The instance directory, relative to the instance root.
pub const INSTANCE_DIR: &str = ".spec-driven-docs";
/// The manifest path, relative to the instance root.
pub const MANIFEST_PATH: &str = ".spec-driven-docs/manifest.json";
/// Where an instance keeps its declaration.
pub const CONFIG_PATH: &str = ".spec-driven-docs/config.yaml";
/// Where an instance keeps its debt.
pub const DEBT_PATH: &str = ".spec-driven-docs/debt.yaml";
/// The flat list of exempt chapters an older instance carries.
pub const LEGACY_DEBT_PATH: &str = ".spec-driven-docs/chapter-size-debt.txt";
/// Where a pre-commit configuration lives in a target.
pub const HOOKS_CONFIG_PATH: &str = ".pre-commit-config.yaml";
/// Where the documentation block lives in a target.
pub const AGENTS_DIGEST_PATH: &str = "AGENTS.md";

/// The only roots an upgrade may remove dropped managed files from.
pub const PRUNABLE_ROOTS: &[&str] = &[".spec-driven-docs/", ".claude/skills/", ".agents/skills/"];

/// The state root, relative to the home directory.
pub const STATE_ROOT: &str = ".local/state/spec-driven-docs";
/// The cache root, relative to the home directory, where no variable moves it.
pub const CACHE_ROOT: &str = ".cache/spec-driven-docs";
/// The user-scope skill receipt, relative to the state root.
pub const SKILL_RECEIPT_FILE: &str = "skills.json";
/// What the skills share, relative to the state root.
pub const SHARED_DIR: &str = "skills/shared";
/// The plan store, relative to the state root.
pub const PLAN_STORE_DIR: &str = "plans";
/// The verified release bundles, relative to the cache root.
pub const BUNDLE_CACHE_DIR: &str = "bundles";

/// The user-scope skill receipt, relative to the home directory.
pub const SKILL_RECEIPT_PATH: &str = ".local/state/spec-driven-docs/skills.json";
/// The root holding what the skills share, relative to the home directory.
pub const SHARED_ROOT: &str = ".local/state/spec-driven-docs/skills/shared";
/// The skill root Claude Code reads, relative to the home directory.
pub const CLAUDE_ROOT: &str = ".claude/skills";
/// The skill root every other agent reads, relative to the home directory.
pub const AGENTS_ROOT: &str = ".agents/skills";

/// Which agent family a skill root serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentId {
    /// Claude Code, which reads its own configuration directory.
    Claude,
    /// The shared root Codex, `OpenCode`, and Pi each document that they read.
    Agents,
}

impl AgentId {
    /// The kebab-case name used on the command line and in the report.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Agents => "agents",
        }
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One agent skill root, and what relocates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentRoot {
    /// The family this root serves.
    pub id: AgentId,
    /// The root, relative to the home directory, where no variable moves it.
    pub default: &'static str,
    /// The variable relocating the agent's whole configuration directory.
    pub config_env: Option<&'static str>,
    /// The root, relative to that variable's value.
    pub relocated: &'static str,
}

/// The root Claude Code reads, and the variable that relocates it.
const CLAUDE: AgentRoot = AgentRoot {
    id: AgentId::Claude,
    default: CLAUDE_ROOT,
    config_env: Some(CLAUDE_CONFIG_DIR_VAR),
    relocated: "skills",
};

/// The root every other agent reads. No variable relocates it.
const AGENTS: AgentRoot = AgentRoot {
    id: AgentId::Agents,
    default: AGENTS_ROOT,
    config_env: None,
    relocated: "skills",
};

/// Every agent skill root this tool writes, in report order.
///
/// Two rows, and no third. Codex, `OpenCode`, and Pi each document that they
/// read the shared root, checked 2026-09-12, so a row per host would be
/// three names for one directory. A host with a root of its own is one more
/// row and one more test on the day it needs one.
pub const AGENT_ROOTS: &[AgentRoot] = &[CLAUDE, AGENTS];

/// The agent root one identifier names.
#[must_use]
pub const fn agent_root(id: AgentId) -> &'static AgentRoot {
    match id {
        AgentId::Claude => &CLAUDE,
        AgentId::Agents => &AGENTS,
    }
}

/// What decided a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathSource {
    /// The instance manifest records it.
    Recorded,
    /// Nothing moved it, so it is the declared default.
    Default,
    /// A variable carries it.
    Env,
    /// The profile's documentation root derives it.
    Profile,
    /// The target's own shape suggests it, and nothing recorded it.
    Proposal,
}

/// One resolved path, and what decided it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathEntry {
    /// The resolved path.
    pub path: Utf8PathBuf,
    /// What decided it.
    pub source: PathSource,
}

impl PathEntry {
    /// A path nothing moved.
    #[must_use]
    pub fn default_at(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            path: path.into(),
            source: PathSource::Default,
        }
    }

    /// A path a variable carried.
    #[must_use]
    pub fn from_env(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            path: path.into(),
            source: PathSource::Env,
        }
    }
}

/// One resolved agent skill root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentRootEntry {
    /// The family this root serves.
    pub id: AgentId,
    /// The absolute root.
    pub path: Utf8PathBuf,
    /// What decided it.
    pub source: PathSource,
    /// The variable that relocated it, where one did.
    pub variable: Option<&'static str>,
}

/// Where a location the project owns actually sits.
///
/// The four absent-looking cases are not one case. A repository-relative
/// directory is checkable from a fresh clone. A directory outside the
/// repository is not, and neither is one a variable resolves at run time,
/// and a project that keeps none promises nothing at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProjectLocation {
    /// A repository-relative directory under version control.
    Tracked {
        /// Where it sits, relative to the instance root.
        path: Utf8PathBuf,
        /// What decided it.
        source: PathSource,
    },
    /// A repository-relative directory version control does not carry.
    Untracked {
        /// Where it sits, relative to the instance root.
        path: Utf8PathBuf,
        /// What decided it.
        source: PathSource,
    },
    /// A directory outside the repository.
    External {
        /// Where it sits, as the record carries it.
        path: Utf8PathBuf,
        /// What decided it.
        source: PathSource,
    },
    /// Wherever a variable resolves at run time.
    Env {
        /// The variable that carries it.
        variable: &'static str,
        /// What that variable carries here, when it is set.
        value: Option<String>,
    },
    /// The project keeps none.
    None,
}

/// One answer the operator can give for a location the project owns.
///
/// A choice carries a path only where the target already holds that
/// directory. Inventing one would put a path this tool made up in front of
/// an operator as though the repository had suggested it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LocationChoice {
    /// A repository-relative directory the target already carries.
    Observed {
        /// Where it sits, relative to the target root.
        path: Utf8PathBuf,
    },
    /// Wherever a variable resolves at run time.
    Env {
        /// The variable that would carry it.
        variable: &'static str,
        /// What that variable carries here, when it is set.
        value: Option<String>,
    },
    /// A path the operator types.
    Operator,
    /// The project keeps none.
    None,
}

/// The choices this target offers for the two locations it owns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Proposals {
    /// What the plan zone can be here.
    pub plan_zone: Vec<LocationChoice>,
    /// What the docs scratch can be here.
    pub docs_scratch: Vec<LocationChoice>,
}

/// Every user-scope path, resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserPaths {
    /// Where this tool keeps state that outlives a command.
    pub state_root: PathEntry,
    /// Where this tool keeps what it can fetch again.
    pub cache_root: PathEntry,
    /// The receipt vouching for every user-scope file this tool wrote.
    pub skill_receipt: PathEntry,
    /// What every skill shares.
    pub shared_root: PathEntry,
    /// Where computed plans are stored.
    pub plan_store: PathEntry,
    /// Where verified release bundles are cached.
    pub bundle_cache: PathEntry,
    /// The agent skill roots, deduplicated by resolved path.
    pub agent_roots: Vec<AgentRootEntry>,
}

/// Every control and documentation destination inside one target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstancePaths {
    /// The instance directory.
    pub instance_dir: PathEntry,
    /// The instance manifest.
    pub manifest: PathEntry,
    /// What the project declares about the files its gates judge.
    pub declaration: PathEntry,
    /// The inherited violations the budget gates read.
    pub debt: PathEntry,
    /// The flat list an instance older than the debt file carries.
    pub legacy_debt: PathEntry,
    /// The pre-commit configuration carrying the managed block.
    pub hooks_config: PathEntry,
    /// The root agent digest carrying the documentation block.
    pub agents_digest: PathEntry,
    /// The documentation root.
    pub docs_root: PathEntry,
    /// Where the specifications sit.
    pub specs: PathEntry,
    /// Where the decision records sit.
    pub decisions: PathEntry,
    /// Where the reference material sits.
    pub reference: PathEntry,
    /// Where the step-by-step guides sit.
    pub guides: PathEntry,
}

/// One landed instance's paths, as the record and the environment leave them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivePaths {
    /// The recorded profile.
    pub profile: ProfileId,
    /// Every destination the record implies.
    pub destinations: InstancePaths,
    /// Where the planning tool writes its entry documents.
    pub plan_zone: ProjectLocation,
    /// Where material that is not a statement yet is staged.
    pub docs_scratch: ProjectLocation,
}

/// One profile's destinations, derived and recorded nowhere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidatePaths {
    /// The profile these destinations belong to.
    pub profile: ProfileId,
    /// Every destination that profile implies.
    pub destinations: InstancePaths,
}

/// Every path this binary can name for one target and one user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Paths {
    /// What resolves under the invoking user's home directory, or `null`
    /// where the home variable is unset and nothing under it resolves.
    pub user: Option<UserPaths>,
    /// What the target records, or `null` where it carries no instance.
    pub active: Option<ActivePaths>,
    /// What each profile would imply, whether or not one is recorded.
    pub candidates: BTreeMap<String, CandidatePaths>,
    /// What this target offers for the two locations the project owns.
    pub proposals: Proposals,
}

/// What the process environment carries, read once and passed down.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserEnv {
    /// The invoking user's home directory.
    pub home: Option<Utf8PathBuf>,
    /// Where Claude Code's configuration directory was moved to.
    pub claude_config_dir: Option<Utf8PathBuf>,
    /// The XDG state base directory.
    pub xdg_state_home: Option<Utf8PathBuf>,
    /// The XDG cache base directory.
    pub xdg_cache_home: Option<Utf8PathBuf>,
    /// What the plan-zone variable carries.
    pub plan_zone: Option<String>,
    /// What the docs-scratch variable carries.
    pub docs_scratch: Option<String>,
}

/// What one variable carries, or `None` where it is unset or blank.
///
/// A variable set to the empty string is a variable the shell exported and
/// nothing filled in. Treating it as a path would resolve every root to the
/// filesystem root.
#[must_use]
pub fn variable(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

impl UserEnv {
    /// Read every variable this module resolves from, once.
    #[must_use]
    pub fn from_process() -> Self {
        let path = |name: &str| variable(name).map(Utf8PathBuf::from);
        Self {
            home: path(HOME_VAR),
            claude_config_dir: path(CLAUDE_CONFIG_DIR_VAR),
            xdg_state_home: path(XDG_STATE_HOME_VAR),
            xdg_cache_home: path(XDG_CACHE_HOME_VAR),
            plan_zone: variable(PLAN_ZONE_VAR),
            docs_scratch: variable(DOCS_SCRATCH_VAR),
        }
    }

    /// The state root: home-relative until the skills stop spelling it.
    #[must_use]
    pub fn state_root(&self) -> Option<PathEntry> {
        self.home
            .as_ref()
            .map(|home| PathEntry::default_at(home.join(STATE_ROOT)))
    }

    /// The cache root, through the XDG variable or its default.
    #[must_use]
    pub fn cache_root(&self) -> Option<PathEntry> {
        if let Some(base) = self.xdg_cache_home.as_ref() {
            return Some(PathEntry::from_env(base.join(TOOL_DIR)));
        }
        self.home
            .as_ref()
            .map(|home| PathEntry::default_at(home.join(CACHE_ROOT)))
    }

    /// One agent skill root, resolved with what decided it.
    #[must_use]
    pub fn agent_root(&self, id: AgentId) -> Option<AgentRootEntry> {
        let row = agent_root(id);
        let home = self.home.as_ref()?;
        let relocated = match row.id {
            AgentId::Claude => self.claude_config_dir.as_ref(),
            AgentId::Agents => None,
        };
        Some(relocated.map_or_else(
            || AgentRootEntry {
                id: row.id,
                path: home.join(row.default),
                source: PathSource::Default,
                variable: None,
            },
            |base| AgentRootEntry {
                id: row.id,
                path: base.join(row.relocated),
                source: PathSource::Env,
                variable: row.config_env,
            },
        ))
    }

    /// Every selected agent root, in table order, deduplicated by path.
    ///
    /// Two selected roots that resolve to one directory are one destination.
    /// Planning it twice would list every file twice and make an install
    /// compare a write against itself.
    #[must_use]
    pub fn agent_roots(&self, selected: &[AgentId]) -> Vec<AgentRootEntry> {
        let mut resolved: Vec<AgentRootEntry> = Vec::new();
        for row in AGENT_ROOTS {
            if !selected.contains(&row.id) {
                continue;
            }
            let Some(entry) = self.agent_root(row.id) else {
                continue;
            };
            if resolved.iter().any(|held| held.path == entry.path) {
                continue;
            }
            resolved.push(entry);
        }
        resolved
    }

    /// Every user-scope path, with every agent root selected.
    #[must_use]
    pub fn user_paths(&self) -> Option<UserPaths> {
        let state = self.state_root()?;
        let cache = self.cache_root()?;
        Some(UserPaths {
            skill_receipt: PathEntry {
                path: state.path.join(SKILL_RECEIPT_FILE),
                source: state.source,
            },
            shared_root: PathEntry {
                path: state.path.join(SHARED_DIR),
                source: state.source,
            },
            plan_store: PathEntry {
                path: state.path.join(PLAN_STORE_DIR),
                source: state.source,
            },
            bundle_cache: PathEntry {
                path: cache.path.join(BUNDLE_CACHE_DIR),
                source: cache.source,
            },
            agent_roots: self.agent_roots(&[AgentId::Claude, AgentId::Agents]),
            state_root: state,
            cache_root: cache,
        })
    }
}

/// Every destination one documentation root implies.
#[must_use]
pub fn instance_paths(docs_root: DocsRoot) -> InstancePaths {
    let docs = Utf8PathBuf::from(docs_root.as_str());
    let under = |leaf: &str| PathEntry {
        path: docs.join(leaf),
        source: PathSource::Profile,
    };
    InstancePaths {
        instance_dir: PathEntry::default_at(INSTANCE_DIR),
        manifest: PathEntry::default_at(MANIFEST_PATH),
        declaration: PathEntry::default_at(CONFIG_PATH),
        debt: PathEntry::default_at(DEBT_PATH),
        legacy_debt: PathEntry::default_at(LEGACY_DEBT_PATH),
        hooks_config: PathEntry::default_at(HOOKS_CONFIG_PATH),
        agents_digest: PathEntry::default_at(AGENTS_DIGEST_PATH),
        docs_root: PathEntry {
            path: docs.clone(),
            source: PathSource::Profile,
        },
        specs: under("specs"),
        decisions: under("decisions"),
        reference: under("reference"),
        guides: under("guides"),
    }
}

/// The same destinations, marked as the record's rather than a profile's.
#[must_use]
pub fn recorded_paths(docs_root: DocsRoot) -> InstancePaths {
    let mut paths = instance_paths(docs_root);
    for entry in [
        &mut paths.docs_root,
        &mut paths.specs,
        &mut paths.decisions,
        &mut paths.reference,
        &mut paths.guides,
    ] {
        entry.source = PathSource::Recorded;
    }
    paths
}

/// One profile's derived destinations, per profile, keyed by its name.
#[must_use]
pub fn candidates() -> BTreeMap<String, CandidatePaths> {
    ProfileId::every()
        .map(|profile| {
            (
                profile.as_str().to_string(),
                CandidatePaths {
                    profile,
                    destinations: instance_paths(profile.profile().docs_root),
                },
            )
        })
        .collect()
}

/// Where the record says the plan zone sits.
#[must_use]
pub fn plan_zone_location(zone: &PlanZone, env: &UserEnv) -> ProjectLocation {
    match zone {
        PlanZone::Tracked { path } => ProjectLocation::Tracked {
            path: path.clone(),
            source: PathSource::Recorded,
        },
        PlanZone::Untracked { path } => ProjectLocation::Untracked {
            path: path.clone(),
            source: PathSource::Recorded,
        },
        PlanZone::Env => ProjectLocation::Env {
            variable: PLAN_ZONE_VAR,
            value: env.plan_zone.clone(),
        },
        PlanZone::None => ProjectLocation::None,
    }
}

/// Where the record says the docs scratch sits.
///
/// A recorded scratch may leave the repository, because staging beside the
/// checkout is one of the offered answers, so the leading component decides
/// which kind it is. The variable overrides the record, and the report says
/// so by naming the variable rather than the recorded path.
#[must_use]
pub fn docs_scratch_location(recorded: Option<&Utf8Path>, env: &UserEnv) -> ProjectLocation {
    if env.docs_scratch.is_some() {
        return ProjectLocation::Env {
            variable: DOCS_SCRATCH_VAR,
            value: env.docs_scratch.clone(),
        };
    }
    let Some(path) = recorded else {
        return ProjectLocation::None;
    };
    if path.is_absolute() || path.starts_with("..") {
        return ProjectLocation::External {
            path: path.to_owned(),
            source: PathSource::Recorded,
        };
    }
    ProjectLocation::Untracked {
        path: path.to_owned(),
        source: PathSource::Recorded,
    }
}

/// The directories a target already carries that a plan zone could be.
///
/// Read from the target rather than declared, because a repository that
/// already writes plans somewhere has answered the question and the
/// operator only has to confirm it.
pub const PLAN_ZONE_LEAVES: &[&str] = &["plan", "plans"];

/// The directories a target already carries that a docs scratch could be.
pub const DOCS_SCRATCH_LEAVES: &[&str] = &[".docs-scratch", ".scratch"];

/// What this target offers for the two locations the project owns.
///
/// `held` answers whether the target carries a repository-relative
/// directory, so the pure derivation stays testable and the caller owns the
/// one filesystem read.
pub fn proposals(env: &UserEnv, held: impl Fn(&Utf8Path) -> bool) -> Proposals {
    let mut plan_zone: Vec<LocationChoice> = Vec::new();
    for profile in ProfileId::every() {
        let docs = Utf8PathBuf::from(profile.profile().docs_root.as_str());
        for leaf in PLAN_ZONE_LEAVES {
            let candidate = docs.join(leaf);
            if held(&candidate) && !plan_zone.iter().any(|choice| names(choice, &candidate)) {
                plan_zone.push(LocationChoice::Observed { path: candidate });
            }
        }
    }
    plan_zone.push(LocationChoice::Env {
        variable: PLAN_ZONE_VAR,
        value: env.plan_zone.clone(),
    });
    plan_zone.push(LocationChoice::Operator);
    plan_zone.push(LocationChoice::None);

    let mut docs_scratch: Vec<LocationChoice> = Vec::new();
    for leaf in DOCS_SCRATCH_LEAVES {
        let candidate = Utf8PathBuf::from(*leaf);
        if held(&candidate) {
            docs_scratch.push(LocationChoice::Observed { path: candidate });
        }
    }
    docs_scratch.push(LocationChoice::Env {
        variable: DOCS_SCRATCH_VAR,
        value: env.docs_scratch.clone(),
    });
    docs_scratch.push(LocationChoice::Operator);
    docs_scratch.push(LocationChoice::None);

    Proposals {
        plan_zone,
        docs_scratch,
    }
}

fn names(choice: &LocationChoice, path: &Utf8Path) -> bool {
    matches!(choice, LocationChoice::Observed { path: held } if held == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(home: &str) -> UserEnv {
        UserEnv {
            home: Some(Utf8PathBuf::from(home)),
            ..UserEnv::default()
        }
    }

    #[test]
    fn the_agent_root_table_has_two_rows_and_resolves_each_with_its_source() {
        assert_eq!(AGENT_ROOTS.len(), 2);
        let resolved = env("/h").agent_roots(&[AgentId::Claude, AgentId::Agents]);
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].path, "/h/.claude/skills");
        assert_eq!(resolved[0].source, PathSource::Default);
        assert_eq!(resolved[0].variable, None);
        assert_eq!(resolved[1].path, "/h/.agents/skills");
        assert_eq!(resolved[1].id, AgentId::Agents);
    }

    #[test]
    fn claude_config_dir_relocates_the_claude_root_and_nothing_else() {
        let moved = UserEnv {
            claude_config_dir: Some(Utf8PathBuf::from("/elsewhere/claude")),
            ..env("/h")
        };
        let resolved = moved.agent_roots(&[AgentId::Claude, AgentId::Agents]);
        assert_eq!(resolved[0].path, "/elsewhere/claude/skills");
        assert_eq!(resolved[0].source, PathSource::Env);
        assert_eq!(resolved[0].variable, Some(CLAUDE_CONFIG_DIR_VAR));
        assert_eq!(resolved[1].path, "/h/.agents/skills");
        assert_eq!(resolved[1].source, PathSource::Default);
    }

    #[test]
    fn an_empty_claude_config_dir_is_treated_as_unset() {
        // `variable` is what strips it, so the resolver never sees a blank.
        let blank = UserEnv {
            claude_config_dir: None,
            ..env("/h")
        };
        assert_eq!(
            blank.agent_root(AgentId::Claude).map(|root| root.path),
            Some(Utf8PathBuf::from("/h/.claude/skills"))
        );
    }

    #[test]
    fn two_selected_roots_that_resolve_to_one_path_are_returned_once() {
        let collided = UserEnv {
            claude_config_dir: Some(Utf8PathBuf::from("/h/.agents")),
            ..env("/h")
        };
        let resolved = collided.agent_roots(&[AgentId::Claude, AgentId::Agents]);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].path, "/h/.agents/skills");
        assert_eq!(resolved[0].id, AgentId::Claude);
    }

    #[test]
    fn the_cache_root_follows_xdg_cache_home_and_its_default() {
        assert_eq!(
            env("/h").cache_root(),
            Some(PathEntry::default_at("/h/.cache/spec-driven-docs"))
        );
        let moved = UserEnv {
            xdg_cache_home: Some(Utf8PathBuf::from("/c")),
            ..env("/h")
        };
        assert_eq!(
            moved.cache_root(),
            Some(PathEntry::from_env("/c/spec-driven-docs"))
        );
    }

    #[test]
    fn the_state_root_stays_home_relative_until_the_skills_stop_spelling_it() {
        assert_eq!(
            env("/h").state_root(),
            Some(PathEntry::default_at("/h/.local/state/spec-driven-docs"))
        );
    }

    #[test]
    fn the_user_paths_hang_off_the_two_roots() {
        let paths = env("/h").user_paths().unwrap();
        assert_eq!(
            paths.skill_receipt.path,
            "/h/.local/state/spec-driven-docs/skills.json"
        );
        assert_eq!(
            paths.shared_root.path,
            "/h/.local/state/spec-driven-docs/skills/shared"
        );
        assert_eq!(
            paths.plan_store.path,
            "/h/.local/state/spec-driven-docs/plans"
        );
        assert_eq!(
            paths.bundle_cache.path,
            "/h/.cache/spec-driven-docs/bundles"
        );
    }

    #[test]
    fn the_home_relative_constants_agree_with_the_resolved_roots() {
        let paths = env("/h").user_paths().unwrap();
        assert_eq!(
            paths.skill_receipt.path,
            Utf8Path::new("/h").join(SKILL_RECEIPT_PATH)
        );
        assert_eq!(
            paths.shared_root.path,
            Utf8Path::new("/h").join(SHARED_ROOT)
        );
    }

    #[test]
    fn a_candidate_set_exists_for_every_profile() {
        let candidates = candidates();
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates["codebase"].destinations.specs.path, "docs/specs");
        assert_eq!(
            candidates["knowledge-base"].destinations.specs.path,
            "_docs/specs"
        );
        assert_eq!(
            candidates["codebase"].destinations.declaration.source,
            PathSource::Default
        );
        assert_eq!(
            candidates["codebase"].destinations.specs.source,
            PathSource::Profile
        );
    }

    #[test]
    fn a_recorded_location_reports_as_recorded_and_an_override_as_env() {
        let plain = env("/h");
        assert_eq!(
            plan_zone_location(
                &PlanZone::Tracked {
                    path: Utf8PathBuf::from("docs/plan")
                },
                &plain
            ),
            ProjectLocation::Tracked {
                path: Utf8PathBuf::from("docs/plan"),
                source: PathSource::Recorded,
            }
        );
        let overridden = UserEnv {
            plan_zone: Some("elsewhere".to_string()),
            ..env("/h")
        };
        assert_eq!(
            plan_zone_location(&PlanZone::Env, &overridden),
            ProjectLocation::Env {
                variable: PLAN_ZONE_VAR,
                value: Some("elsewhere".to_string()),
            }
        );
    }

    #[test]
    fn a_scratch_beside_the_checkout_reports_as_external() {
        let plain = env("/h");
        assert_eq!(
            docs_scratch_location(Some(Utf8Path::new("../x.docs-scratch")), &plain),
            ProjectLocation::External {
                path: Utf8PathBuf::from("../x.docs-scratch"),
                source: PathSource::Recorded,
            }
        );
        assert_eq!(
            docs_scratch_location(Some(Utf8Path::new(".docs-scratch")), &plain),
            ProjectLocation::Untracked {
                path: Utf8PathBuf::from(".docs-scratch"),
                source: PathSource::Recorded,
            }
        );
        assert_eq!(docs_scratch_location(None, &plain), ProjectLocation::None);
    }

    #[test]
    fn a_proposal_carries_a_path_only_where_the_target_holds_one() {
        let plain = env("/h");
        let bare = proposals(&plain, |_| false);
        assert!(
            !bare
                .plan_zone
                .iter()
                .any(|choice| matches!(choice, LocationChoice::Observed { .. }))
        );
        assert_eq!(bare.plan_zone.last(), Some(&LocationChoice::None));

        let observed = proposals(&plain, |path| path == Utf8Path::new("docs/plan"));
        assert_eq!(
            observed.plan_zone.first(),
            Some(&LocationChoice::Observed {
                path: Utf8PathBuf::from("docs/plan")
            })
        );
    }
}
