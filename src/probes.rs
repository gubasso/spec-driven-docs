//! The environment probe catalog.
//!
//! One catalog, read by every caller that needs to know whether this host
//! is ready: `sdd doctor` runs it whole and reports by class. Each probe
//! answers with a status, a message, and — on failure — the remediation
//! printed verbatim wherever the probe is consulted. A probe failure is a
//! result, not an error, so nothing here returns `Err`.

use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::domain::ownership::Sha256;
use crate::domain::paths::{
    AgentId, LEGACY_SHARED_ROOT, OFFLINE_VAR, SKILL_FILE, SKILL_RECEIPT_FILE, SKILL_REFERENCES_DIR,
    UserEnv,
};
use crate::domain::skill_record::SkillRecord;
use crate::services::skill_installer::home;

/// Every agent skill root under this home, resolved through the table.
///
/// The table is the one place that knows a variable moved a root, so a
/// probe that joined the defaults instead would report a directory the
/// install never writes.
fn agent_roots() -> Vec<Utf8PathBuf> {
    UserEnv::from_process()
        .agent_roots(&[AgentId::Claude, AgentId::Agents])
        .into_iter()
        .map(|entry| entry.path)
        .collect()
}

/// The state root this host resolves.
fn resolved_state_root() -> Option<Utf8PathBuf> {
    UserEnv::from_process().state_root().map(|entry| entry.path)
}

/// The receipt, read from the resolved path or the home-relative one.
fn receipt() -> SkillRecord {
    let env = UserEnv::from_process();
    let Some(state) = env.state_root() else {
        return SkillRecord::new();
    };
    let legacy = env
        .legacy_state_root()
        .map_or_else(|| state.path.clone(), |root| root.join(SKILL_RECEIPT_FILE));
    SkillRecord::load_with_fallback(&state.path.join(SKILL_RECEIPT_FILE), &legacy)
}

/// Every installed package file this binary carries, under every agent root
/// that exists.
fn installed_packages() -> Vec<(Utf8PathBuf, &'static [u8])> {
    let mut planned = Vec::new();
    for root in agent_roots() {
        // An absent agent root is a choice, not a defect: `--agent` selects
        // one family and leaves the other's root untouched.
        if !root.is_dir() {
            continue;
        }
        for name in crate::embedded::skill_names() {
            let Some(package) = crate::embedded::skill_package(name) else {
                continue;
            };
            for (relative, bytes) in package {
                planned.push((root.join(name).join(relative), bytes));
            }
        }
    }
    planned
}

/// How a failure weighs at the doctor level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeClass {
    /// No install under the user's home can work without this.
    Hard,
    /// Needed only by some commands or some tasks.
    Soft,
}

/// What a probe found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeStatus {
    /// The probe passed.
    Ok,
    /// The probe failed; the remediation says what fixes it.
    Failed,
}

/// One probe's answer.
#[derive(Debug, Serialize)]
pub struct ProbeResult {
    /// The probe's stable name.
    pub id: &'static str,
    /// How the failure weighs.
    pub class: ProbeClass,
    /// What was found.
    pub status: ProbeStatus,
    /// What was found, one line.
    pub message: String,
    /// The exact fix, when the probe failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

impl ProbeResult {
    fn ok(id: &'static str, class: ProbeClass, message: impl Into<String>) -> Self {
        Self {
            id,
            class,
            status: ProbeStatus::Ok,
            message: message.into(),
            remediation: None,
        }
    }

    fn failed(
        id: &'static str,
        class: ProbeClass,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            id,
            class,
            status: ProbeStatus::Failed,
            message: message.into(),
            remediation: Some(remediation.into()),
        }
    }
}

/// The probes judging the skill installation itself, in catalog order.
///
/// Declared here rather than derived by running the catalog: the shared
/// pre-flight gate must name each of these, and the test holding it to that
/// must not have to write into the operator's home to learn what they are.
pub const SKILL_PROBES: [&str; 3] = ["skill-roots", "skill-gate", "skill-payload"];

/// Run the whole catalog, in its stable order.
#[must_use]
pub fn run_all() -> Vec<ProbeResult> {
    vec![
        state_root(),
        skill_roots(),
        skill_gate(),
        skill_payload(),
        registry(),
        tool(
            "git",
            "SDD_GIT_BIN",
            "git",
            "git; retiring a migrated document is safe only where version control restores it",
            &["--version"],
        ),
        tool(
            "pre-commit",
            "SDD_PRE_COMMIT_BIN",
            "pre-commit",
            "pre-commit; the delivered gates run through it",
            &["--version"],
        ),
    ]
}

/// A helper binary answers its version call. `env_override` names the
/// substitute, which is also what keeps tests hermetic; presence is the
/// whole question, because the tools here take no configuration.
fn tool(
    id: &'static str,
    env_override: &str,
    default_bin: &str,
    label: &str,
    args: &[&str],
) -> ProbeResult {
    let bin = std::env::var(env_override).unwrap_or_else(|_| default_bin.to_owned());
    match Command::new(&bin).args(args).output() {
        Ok(out) if out.status.success() => {
            ProbeResult::ok(id, ProbeClass::Soft, format!("{default_bin} runs"))
        }
        Ok(_) => ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!("{default_bin} does not answer {}", args.join(" ")),
            format!("repair {label}"),
        ),
        Err(_) => ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!("{default_bin} is not on PATH"),
            format!("install {label}"),
        ),
    }
}

/// The registry the release resolver reads is reachable.
///
/// Soft, because a host that cannot fetch is a constraint on a plan rather
/// than a broken install: every default path reads the embedded release,
/// and only a plan toward another release needs this. The probe reads the
/// registry's own configuration, which is the smallest document the
/// protocol defines, and writes nothing.
fn registry() -> ProbeResult {
    let id = "release-registry";
    let url = format!("{}/config.json", crate::release::crates_io::INDEX_ROOT);
    // One variable turns the read off, for a host that is deliberately
    // offline and for every test in this repository's own suite.
    if crate::domain::paths::variable(OFFLINE_VAR).is_some() {
        return ProbeResult::ok(
            id,
            ProbeClass::Soft,
            "SDD_OFFLINE is set, so no release beyond the embedded one is planned",
        );
    }
    // A probe's budget is not a fetch's budget: an unreachable registry
    // must answer this question in seconds, not in the minute a bounded
    // archive read is allowed.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(3)))
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .user_agent(format!("sdd/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    match agent.get(&url).call() {
        Ok(response) if response.status().as_u16() == 200 => {
            ProbeResult::ok(id, ProbeClass::Soft, format!("{url} answers"))
        }
        Ok(response) => ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!("{url} answered {}", response.status().as_u16()),
            "plan toward the embedded release, or retry when the registry answers",
        ),
        Err(source) => ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!("{url} could not be read: {source}"),
            "plan toward the embedded release; only a plan toward another release needs the registry",
        ),
    }
}

/// The state root accepts writes; the receipt, the lock, and the journal
/// live under it.
fn state_root() -> ProbeResult {
    let id = "state-root";
    let Some(root) = resolved_state_root() else {
        return ProbeResult::failed(
            id,
            ProbeClass::Hard,
            "HOME is not set, so no state root resolves",
            "export HOME",
        );
    };
    let probe = root.join(format!(".probe-{}", std::process::id()));
    let written = std::fs::create_dir_all(&root).and_then(|()| std::fs::write(&probe, b"probe"));
    let _ = std::fs::remove_file(&probe);
    match written {
        Ok(()) => ProbeResult::ok(id, ProbeClass::Hard, format!("{root} is writable")),
        Err(source) => ProbeResult::failed(
            id,
            ProbeClass::Hard,
            format!("{root} is not writable: {source}"),
            format!("make {root} writable"),
        ),
    }
}

/// The destinations `sdd skill install` writes accept writes: the two agent
/// roots and the resolved state root.
///
/// A root can exist and still refuse, which is what a read-only bind of an
/// agent directory produces, so what is tested is the nearest existing
/// ancestor — the directory an install would actually have to write
/// through. The probe creates nothing: a preview must still be able to
/// report a root as absent, and a probe that made it exist would take that
/// answer away.
fn skill_roots() -> ProbeResult {
    let id = SKILL_PROBES[0];
    let Ok(home) = home() else {
        return ProbeResult::failed(
            id,
            ProbeClass::Soft,
            "HOME is not set, so no skill root resolves",
            "export HOME",
        );
    };
    let mut refused = Vec::new();
    let mut roots = agent_roots();
    roots.extend(resolved_state_root());
    for root in roots {
        let Some(existing) = nearest_existing(&root) else {
            refused.push(format!("no ancestor of {root} exists"));
            continue;
        };
        if let Err(source) = accepts_a_write(&existing) {
            refused.push(format!("{existing} is not writable: {source}"));
        }
    }
    if refused.is_empty() {
        ProbeResult::ok(
            id,
            ProbeClass::Soft,
            format!("the skill roots under {home} accept writes"),
        )
    } else {
        ProbeResult::failed(
            id,
            ProbeClass::Soft,
            refused.join("; "),
            format!("make the skill roots under {home} writable"),
        )
    }
}

/// Every installed package carries the two gates it is told to read first.
///
/// This is the probe that answers the one failure a shared home produces. A
/// container, a sandbox, or a sync that carries a `SKILL.md` without the
/// `references/` beside it leaves the skill resolvable by name and unable
/// to read the gates its first section names. A skill that cannot read them
/// runs neither its pre-flight nor its plan phase, which is the whole
/// reason they are files rather than prose.
fn skill_gate() -> ProbeResult {
    let id = SKILL_PROBES[1];
    let Ok(home) = home() else {
        return ProbeResult::failed(
            id,
            ProbeClass::Soft,
            "HOME is not set, so no skill package resolves",
            "export HOME",
        );
    };
    let record = receipt();
    let references: Vec<(Utf8PathBuf, &'static [u8])> = installed_packages()
        .into_iter()
        .filter(|(path, _)| {
            path.parent()
                .is_some_and(|parent| parent.file_name() == Some(SKILL_REFERENCES_DIR))
        })
        // Only a package that landed is judged: an absent skill is the
        // payload probe's finding, not this one's.
        .filter(|(path, _)| {
            path.parent()
                .and_then(Utf8Path::parent)
                .is_some_and(|package| package.join(SKILL_FILE).is_file())
        })
        .collect();
    if references.is_empty() {
        return ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!("no installed skill package under {home} carries its gates"),
            "sdd skill install --apply",
        );
    }
    let found = judge(references, &record);
    if let Some(first) = found.missing.first() {
        // The remediation still honours what the rest of the set holds: an
        // absence beside an edit the record cannot vouch for needs the
        // force the edit needs, or the named command refuses.
        return ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!("a gate every skill reads before acting is not installed: {first}"),
            reinstall(found.all_recorded),
        );
    }
    if !found.differing.is_empty() {
        return ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!(
                "{} installed gate reference(s) are not this binary's",
                found.differing.len()
            ),
            reinstall(found.all_recorded),
        );
    }
    if let Some(leftover) = retired_shared_leftover(&home, &record) {
        return ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!("the retired shared root holds a file no receipt vouches for: {leftover}"),
            format!("read {leftover}, then remove it; every skill now carries its own gates"),
        );
    }
    ProbeResult::ok(
        id,
        ProbeClass::Soft,
        format!(
            "{} installed gate reference(s) are this binary's",
            found.matching
        ),
    )
}

/// A file under the retired shared root that no receipt accounts for.
///
/// A sweep takes back what the receipt vouches for. What it leaves is the
/// operator's, and naming it is the only honest thing a probe can do with a
/// file this tool refuses to delete.
fn retired_shared_leftover(home: &Utf8Path, record: &SkillRecord) -> Option<Utf8PathBuf> {
    let retired = home.join(LEGACY_SHARED_ROOT);
    let mut found: Vec<Utf8PathBuf> = walkdir::WalkDir::new(retired.as_std_path())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| Utf8PathBuf::from_path_buf(entry.into_path()).ok())
        .filter(|path| {
            !std::fs::read(path).is_ok_and(|held| record.wrote(path, &Sha256::of(&held)))
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

/// The skills installed under this home are the ones this binary carries.
///
/// One binary serves every repository, so a skill under an agent root and
/// the `sdd` on PATH are two artifacts that can be updated apart: a home
/// shared with a container, a sandbox, or another machine can hold skills
/// some other build installed. The probe names that drift rather than
/// leaving an agent to follow instructions the binary no longer answers.
fn skill_payload() -> ProbeResult {
    let id = SKILL_PROBES[2];
    let Ok(home) = home() else {
        return ProbeResult::failed(
            id,
            ProbeClass::Soft,
            "HOME is not set, so no agent root resolves",
            "export HOME",
        );
    };
    let record = receipt();
    let planned = installed_packages();
    if planned.is_empty() {
        return ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!("no agent skill root exists under {home}"),
            "sdd skill install --apply",
        );
    }
    let found = judge(planned, &record);
    if let Some(first) = found.missing.first() {
        // As in the gate probe: an absence beside an unvouched edit needs
        // the force the edit needs, or the named command refuses.
        return ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!(
                "{} of this binary's package files are not installed, the first at {first}",
                found.missing.len()
            ),
            reinstall(found.all_recorded),
        );
    }
    if !found.differing.is_empty() {
        return ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!(
                "{} installed package file(s) are not this binary's; sdd is {}",
                found.differing.len(),
                env!("CARGO_PKG_VERSION")
            ),
            reinstall(found.all_recorded),
        );
    }
    ProbeResult::ok(
        id,
        ProbeClass::Soft,
        format!(
            "{} installed skill destination(s) are this binary's",
            found.matching
        ),
    )
}

/// What sits at each destination the payload names.
struct Installed {
    /// Destinations the payload names that hold no readable file.
    missing: Vec<Utf8PathBuf>,
    /// Destinations holding bytes that are not this binary's.
    differing: Vec<Utf8PathBuf>,
    /// How many destinations hold exactly this binary's bytes.
    matching: usize,
    /// Whether the record vouches for every differing destination, which
    /// makes the difference a stale install rather than the operator's own
    /// edit — and decides whether the fix needs `--force`.
    all_recorded: bool,
}

/// Judge each destination the payload names against what sits on disk.
fn judge(planned: Vec<(Utf8PathBuf, &'static [u8])>, record: &SkillRecord) -> Installed {
    let mut found = Installed {
        missing: Vec::new(),
        differing: Vec::new(),
        matching: 0,
        all_recorded: true,
    };
    for (destination, bytes) in planned {
        match std::fs::read(&destination) {
            Ok(held) if held == bytes => found.matching += 1,
            Ok(held) => {
                if !record.wrote(&destination, &Sha256::of(&held)) {
                    found.all_recorded = false;
                }
                found.differing.push(destination);
            }
            Err(_) => found.missing.push(destination),
        }
    }
    found
}

/// The install that corrects a difference. Bytes the record vouches for are
/// an older release's and go without asking; bytes it cannot account for are
/// the operator's own, and overwriting those is what `--force` is.
const fn reinstall(all_recorded: bool) -> &'static str {
    if all_recorded {
        "sdd skill install --apply"
    } else {
        "sdd skill install --apply --force"
    }
}

/// The nearest ancestor of `path`, itself included, that exists as a
/// directory.
fn nearest_existing(path: &Utf8Path) -> Option<Utf8PathBuf> {
    let mut current = Some(path);
    while let Some(dir) = current {
        if dir.is_dir() {
            return Some(dir.to_owned());
        }
        current = dir.parent();
    }
    None
}

/// A directory accepts a write, leaving nothing behind.
fn accepts_a_write(dir: &Utf8Path) -> std::io::Result<()> {
    let probe = dir.join(format!(".sdd-probe-{}", std::process::id()));
    let written = std::fs::write(&probe, b"probe");
    let _ = std::fs::remove_file(&probe);
    written
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn utf8(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap())
    }

    #[test]
    fn nearest_existing_walks_up_to_the_first_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        assert_eq!(nearest_existing(&root).as_deref(), Some(root.as_path()));
        assert_eq!(
            nearest_existing(&root.join("a/b/c")).as_deref(),
            Some(root.as_path())
        );
    }

    #[test]
    fn a_write_probe_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        accepts_a_write(&root).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    /// The judge sorts every destination into exactly one bucket, and the
    /// record decides whether a differing one still counts as the tool's.
    #[test]
    fn the_judge_tells_stale_bytes_from_the_users_own() {
        let dir = tempfile::tempdir().unwrap();
        let root = utf8(&dir);
        let matching = root.join("matching.md");
        let stale = root.join("stale.md");
        let edited = root.join("edited.md");
        let missing = root.join("missing.md");
        std::fs::write(&matching, b"payload").unwrap();
        std::fs::write(&stale, b"older release").unwrap();
        std::fs::write(&edited, b"the user's own").unwrap();

        let mut record = SkillRecord::new();
        record
            .written
            .insert(stale.clone(), Sha256::of(b"older release"));

        let planned: Vec<(Utf8PathBuf, &'static [u8])> = vec![
            (matching, b"payload"),
            (stale, b"payload"),
            (missing.clone(), b"payload"),
        ];
        let found = judge(planned, &record);
        assert_eq!(found.matching, 1);
        assert_eq!(found.differing.len(), 1);
        assert_eq!(found.missing, vec![missing]);
        assert!(found.all_recorded, "the record vouches for the stale copy");

        let planned: Vec<(Utf8PathBuf, &'static [u8])> = vec![(edited, b"payload")];
        let found = judge(planned, &record);
        assert!(
            !found.all_recorded,
            "bytes the record cannot account for are the user's"
        );
    }

    #[test]
    fn the_reinstall_needs_force_only_over_the_users_bytes() {
        assert_eq!(reinstall(true), "sdd skill install --apply");
        assert_eq!(reinstall(false), "sdd skill install --apply --force");
    }
}
