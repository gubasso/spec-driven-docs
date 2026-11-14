//! The environment probe catalog.
//!
//! One catalog, read by every caller that needs to know whether this host
//! is ready: `sdd doctor` runs it whole and reports by class. Each probe
//! answers with a status, a message, and — on failure — the remediation
//! printed verbatim wherever the probe is consulted. A probe failure is a
//! result, not an error, so nothing here returns `Err`.

use std::process::Command;

use camino::Utf8PathBuf;
use serde::Serialize;

use crate::domain::ownership::Sha256;
use crate::domain::paths::{AgentId, OFFLINE_VAR, SKILL_FILE, SKILL_RECEIPT_FILE, UserEnv};
use crate::domain::skill_record::SkillRecord;

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
pub const SKILL_PROBES: [&str; 1] = ["skill-shadow"];

/// Run the whole catalog, in its stable order.
#[must_use]
pub fn run_all() -> Vec<ProbeResult> {
    vec![
        state_root(),
        skill_shadow(),
        registry(),
        pin_fresh(),
        tool(
            "git",
            "SDD_GIT_BIN",
            "git",
            "git; retiring a migrated document is safe only where version control restores it",
            &["--version"],
        ),
        tool(
            "prek",
            "SDD_PREK_BIN",
            "prek",
            "prek; the delivered gates run through it",
            &["--version"],
        ),
    ]
}

/// A host tool answers its version call. `env_override` names the
/// substitute, which is also what keeps tests hermetic; presence is the
/// whole question, because the tools here take no configuration.
///
/// The probe only reports. No command reads its result to pick a tool or
/// to skip a step: a command that needs the tool runs it, and fails where
/// the host lacks it.
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

/// The pin of the current folder against the newest release the
/// version-check cache records, offline.
///
/// Soft, because a pin that is behind is the operator's to move, and
/// unknown where the folder pins nothing or no check has run yet. A pin
/// that is behind fails with the pending text as its remediation.
///
/// SATISFIES acquisition:every-report-names-a-pending-move
fn pin_fresh() -> ProbeResult {
    use crate::self_depend::status::{PinFreshness, freshness};
    let id = "pin-fresh";
    let Some(folder) = std::env::current_dir()
        .ok()
        .and_then(|dir| Utf8PathBuf::from_path_buf(dir).ok())
    else {
        return ProbeResult::ok(
            id,
            ProbeClass::Soft,
            "unknown: the current folder is unreadable",
        );
    };
    match freshness(&folder, &UserEnv::from_process()) {
        PinFreshness::Unknown => ProbeResult::ok(
            id,
            ProbeClass::Soft,
            "unknown: no pin or no version check recorded here",
        ),
        PinFreshness::Current { pin } => ProbeResult::ok(
            id,
            ProbeClass::Soft,
            format!("ok: the pin {pin} names the newest recorded release"),
        ),
        PinFreshness::Behind(pending) => ProbeResult::failed(
            id,
            ProbeClass::Soft,
            format!(
                "behind: sdd {} is out, the pin names {}",
                pending.latest, pending.pin
            ),
            pending.text,
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
    let url = format!("{}/config.json", crate::self_depend::registry::INDEX_ROOT);
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

/// No copy of a payload skill under a home agent root shadows the copy an
/// instance pins.
///
/// An agent prefers a personal skill over a project skill of the same name,
/// or shows both, so a `SKILL.md` an earlier release installed under the
/// home hides the one the project landed. A folder holding only notes
/// passes. A manual the receipt no longer vouches for is the operator's
/// edit, which the uninstall keeps, so the probe asks for a hand removal.
///
/// SATISFIES distribution:the-doctor-names-a-shadowing-user-copy
fn skill_shadow() -> ProbeResult {
    let id = "skill-shadow";
    let record = receipt();
    let mut vouched = Vec::new();
    let mut edited = Vec::new();
    for root in agent_roots() {
        for name in crate::embedded::skill_names() {
            let manual = root.join(name).join(SKILL_FILE);
            let Ok(bytes) = std::fs::read(&manual) else {
                continue;
            };
            if record.wrote(&manual, &Sha256::of(&bytes)) {
                vouched.push(manual);
            } else {
                edited.push(manual);
            }
        }
    }
    if vouched.is_empty() && edited.is_empty() {
        return ProbeResult::ok(
            id,
            ProbeClass::Soft,
            "no sdd skill under a home agent root shadows an instance copy",
        );
    }
    let mut found: Vec<String> = vouched.iter().map(ToString::to_string).collect();
    found.extend(edited.iter().map(|path| format!("{path} (edited)")));
    let remediation = if edited.is_empty() {
        "sdd skill uninstall --apply".to_string()
    } else if vouched.is_empty() {
        "read each edited manual, keep what you need elsewhere, and remove it by hand".to_string()
    } else {
        "sdd skill uninstall --apply, then read each edited manual it keeps and remove it by hand"
            .to_string()
    };
    ProbeResult::failed(
        id,
        ProbeClass::Soft,
        format!(
            "a home copy shadows the skill each instance pins: {}",
            found.join(", ")
        ),
        remediation,
    )
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    #[test]
    fn the_skill_probes_are_in_the_catalog() {
        for id in SKILL_PROBES {
            assert!(run_all().iter().any(|probe| probe.id == id), "{id}");
        }
    }
}
