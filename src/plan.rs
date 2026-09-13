//! One typed document that every landing write comes from.
//!
//! Three verbs used to walk one projection on three paths, which is three
//! places for a rule to drift. A plan is one place: the engine computes it,
//! an operator reads and approves it, and an apply executes exactly its
//! operations or refuses because its inputs moved.
//!
//! The document keeps five kinds apart, and an addition inside one kind is
//! additive rather than a new shape:
//!
//! - Evidence is what was observed.
//! - Analysis is what the engine derived: findings, operations.
//! - Policy is the requirement each precondition carries.
//! - Decisions are workflow state the operator owns.
//! - Postconditions are what proves completion.
//!
//! The planner is pure. Observation, resolution, and the clock are inputs,
//! so the same inputs produce the same plan and the same fingerprint.

pub mod apply;
pub mod classify;
pub mod decision;
pub mod derive;
pub mod evidence;
pub mod finding;
pub mod fingerprint;
pub mod observe;
pub mod operation;
pub mod planner;
pub mod readiness;
pub mod store;

use serde::{Deserialize, Serialize};

use crate::domain::ownership::Sha256;
use crate::domain::profile::ProfileId;
use crate::plan::classify::Classification;
use crate::plan::decision::Decision;
use crate::plan::evidence::Evidence;
use crate::plan::finding::{Finding, StyleCandidate};
use crate::plan::observe::{Corpus, Host, Installation, Repository};
use crate::plan::operation::Operation;
use crate::plan::readiness::{Precondition, Readiness};

/// The machine schema a plan declares.
///
/// Independent of the status, record, payload, and manifest schemas. They
/// version different things on different dates.
pub const PLAN_SCHEMA: &str = "sdd.plan/1";

/// Which plan this is, and what produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    /// The machine schema of this object.
    pub schema: String,
    /// The plan's own identifier, which is its fingerprint.
    pub plan_id: String,
    /// When it was computed, as the caller's clock reported it.
    pub created_at: String,
    /// The engine that computed it.
    pub engine_version: String,
}

/// What the plan is aiming at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesiredState {
    /// What the caller asked for, before resolution.
    pub selector: String,
    /// The exact release the selector resolved to.
    pub release: String,
    /// The digest over that release's content.
    pub release_sha256: Sha256,
    /// The profile the target takes.
    pub profile: Option<ProfileId>,
    /// What that release declares it lands, as counts a reader can check.
    pub declared: Declared,
}

/// How much one release's declaration covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Declared {
    /// The protocol version between the engine and the bundle.
    pub payload_schema: u32,
    /// How many byte projections the canon keeps owning.
    pub managed: usize,
    /// How many seeds the instance owns from the moment they land.
    pub adopted: usize,
    /// How many sentinels the release authorizes.
    pub sentinels: usize,
}

/// What the plan found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedState {
    /// The repository itself.
    pub repository: Repository,
    /// What is installed there, where anything is.
    pub installation: Option<Installation>,
    /// The host this command ran on.
    pub host: Host,
    /// What the corpus looks like.
    pub corpus: Corpus,
    /// What each of the above rests on.
    pub evidence_refs: Vec<String>,
}

/// Where the release came from, and whether it was verified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseSource {
    /// The release's version.
    pub version: String,
    /// Where its facts came from.
    pub provenance: String,
    /// The registry checksum, where a registry served it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_checksum: Option<Sha256>,
    /// Whether the registry marks it yanked.
    pub yanked: bool,
    /// What each of the above rests on.
    pub evidence_refs: Vec<String>,
}

/// One typed check the apply runs at the end and reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Postcondition {
    /// A stable identifier.
    pub id: String,
    /// What must be true once the apply has finished.
    pub statement: String,
}

/// The whole document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// Which plan this is.
    pub identity: Identity,
    /// What the target is.
    pub classification: Classification,
    /// What the corpus shows.
    pub findings: Vec<Finding>,
    /// Documents written before the instance, named and never judged.
    pub style_candidates: Vec<StyleCandidate>,
    /// What the plan is aiming at.
    pub desired_state: DesiredState,
    /// What the plan found.
    pub observed_state: ObservedState,
    /// Where the release came from.
    pub release: ReleaseSource,
    /// Every write the apply will make, in apply order.
    pub operations: Vec<Operation>,
    /// Everything that must hold first.
    pub preconditions: Vec<Precondition>,
    /// Everything the operator decides.
    pub decisions: Vec<Decision>,
    /// Everything the apply proves at the end.
    pub postconditions: Vec<Postcondition>,
    /// What was observed, and how.
    pub evidence: Vec<Evidence>,
    /// Whether the plan may be applied.
    pub readiness: Readiness,
    /// The digest over the semantic inputs an approval binds to.
    pub input_fingerprint: Sha256,
}

impl Plan {
    /// Whether an apply may proceed.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self.readiness, Readiness::Ready)
    }

    /// Every finding of one kind.
    #[must_use]
    pub fn findings_of(&self, kind: finding::FindingKind) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|found| found.kind == kind)
            .collect()
    }

    /// How many structural findings the corpus shows.
    #[must_use]
    pub fn structural_count(&self) -> usize {
        self.findings_of(finding::FindingKind::Structural).len()
    }
}
