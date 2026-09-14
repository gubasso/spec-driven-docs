//! What a target is, read from the target and never from the request.
//!
//! A front verb says what the operator meant to do. It does not get to say
//! what the target is: `init` cannot turn a settled corpus into a
//! greenfield landing by asking nicely, and `upgrade` cannot turn an
//! absent instance into an upgrade. The classification comes from what was
//! observed, and a front constrains which classifications it will serve.
//!
//! Findings stay orthogonal. A migration with no structural finding and a
//! migration with five are the same classification and different plans.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// What the planner found the target to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    /// No instance and nothing durable written: land one.
    Setup,
    /// No instance and a settled corpus: land one and move the corpus.
    Migration,
    /// An instance older than the destination.
    Upgrade,
    /// An instance at the destination whose files have moved.
    Drift,
    /// An instance at the destination with nothing to do.
    Current,
    /// Metadata that exists and cannot be trusted.
    Invalid,
}

impl Classification {
    /// The kebab-case word, as the JSON spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Setup => "setup",
            Self::Migration => "migration",
            Self::Upgrade => "upgrade",
            Self::Drift => "drift",
            Self::Current => "current",
            Self::Invalid => "invalid",
        }
    }
}

impl std::fmt::Display for Classification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a front verb is willing to serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Whatever the target is.
    Reconcile,
    /// A first landing.
    Init,
    /// A move to a newer release.
    Upgrade,
    /// A classification alone, writing nothing.
    Assess,
}

/// A front verb asked for a target it does not serve.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{verb} does not serve a {found} target; run {next}")]
pub struct IntentRefused {
    /// The verb the operator ran.
    pub verb: &'static str,
    /// What the target turned out to be.
    pub found: Classification,
    /// What to run instead.
    pub next: &'static str,
}

impl Intent {
    /// Whether this front serves what the target turned out to be.
    ///
    /// # Errors
    ///
    /// [`IntentRefused`] naming the verb, the classification, and the next
    /// command. A front that widened its own scope here would be the
    /// classification lying about the target.
    pub const fn accepts(self, found: Classification) -> Result<(), IntentRefused> {
        match (self, found) {
            (Self::Reconcile | Self::Assess, _)
            // A landing verb still reinstalls over an instance it already
            // owns. What it may not do is land seeds beside a convention
            // that is already there, or over a record nobody can read.
            | (
                Self::Init,
                Classification::Setup
                | Classification::Upgrade
                | Classification::Drift
                | Classification::Current,
            )
            | (
                Self::Upgrade,
                Classification::Upgrade | Classification::Drift | Classification::Current,
            ) => Ok(()),
            (Self::Init, _) => Err(IntentRefused {
                verb: "sdd init",
                found,
                next: "sdd reconcile plan",
            }),
            (Self::Upgrade, _) => Err(IntentRefused {
                verb: "sdd upgrade",
                found,
                next: "sdd reconcile plan",
            }),
        }
    }
}

/// What the observation has to say before a classification can be read.
///
/// Five independent answers rather than one enumeration, because the
/// classification is what combines them and a caller that had to pick a
/// combined value would be classifying before the classifier does.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each field is one independent observation, and folding them would move the classification into the caller"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signals {
    /// Metadata exists and cannot be trusted.
    pub invalid: bool,
    /// An instance is recorded.
    pub installed: bool,
    /// The recorded release is the destination.
    pub at_destination: bool,
    /// A recorded file has moved.
    pub drifted: bool,
    /// The target documents itself already.
    pub settled: bool,
}

/// Read the classification from what was observed.
#[must_use]
pub const fn classify(signals: Signals) -> Classification {
    if signals.invalid {
        return Classification::Invalid;
    }
    if !signals.installed {
        if signals.settled {
            return Classification::Migration;
        }
        return Classification::Setup;
    }
    if !signals.at_destination {
        return Classification::Upgrade;
    }
    if signals.drifted {
        return Classification::Drift;
    }
    Classification::Current
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn signals() -> Signals {
        Signals {
            invalid: false,
            installed: false,
            at_destination: false,
            drifted: false,
            settled: false,
        }
    }

    #[test]
    fn each_of_the_seven_target_states_classifies_correctly() {
        // Empty.
        assert_eq!(classify(signals()), Classification::Setup);
        // Brownfield, whether or not it carries structural findings: the
        // findings are a separate axis and do not move the classification.
        let settled = Signals {
            settled: true,
            ..signals()
        };
        assert_eq!(classify(settled), Classification::Migration);
        // Landed and older than the destination.
        let old = Signals {
            installed: true,
            ..signals()
        };
        assert_eq!(classify(old), Classification::Upgrade);
        // Landed, at the destination, and moved.
        let drifted = Signals {
            installed: true,
            at_destination: true,
            drifted: true,
            ..signals()
        };
        assert_eq!(classify(drifted), Classification::Drift);
        // Landed, at the destination, and untouched.
        let current = Signals {
            installed: true,
            at_destination: true,
            ..signals()
        };
        assert_eq!(classify(current), Classification::Current);
        // Metadata that exists and cannot be trusted.
        let invalid = Signals {
            invalid: true,
            installed: true,
            ..signals()
        };
        assert_eq!(classify(invalid), Classification::Invalid);
    }

    #[test]
    fn malformed_metadata_is_invalid_and_never_absence() {
        // Absence and breakage are different findings, and reporting a
        // broken instance as absent would invite a destructive landing.
        let invalid = Signals {
            invalid: true,
            ..signals()
        };
        assert_eq!(classify(invalid), Classification::Invalid);
        assert_ne!(classify(invalid), Classification::Setup);
    }

    #[test]
    fn init_intent_cannot_force_a_settled_target_to_setup() {
        let error = Intent::Init.accepts(Classification::Migration).unwrap_err();
        assert_eq!(error.verb, "sdd init");
        assert!(error.to_string().contains("sdd reconcile plan"));
        assert!(Intent::Init.accepts(Classification::Setup).is_ok());
        // A reinstall over an instance the verb already owns still works.
        assert!(Intent::Init.accepts(Classification::Current).is_ok());
        assert!(Intent::Init.accepts(Classification::Upgrade).is_ok());
        assert!(Intent::Init.accepts(Classification::Invalid).is_err());
    }

    #[test]
    fn upgrade_intent_cannot_turn_an_absent_instance_into_an_upgrade() {
        assert!(Intent::Upgrade.accepts(Classification::Setup).is_err());
        assert!(Intent::Upgrade.accepts(Classification::Migration).is_err());
        assert!(Intent::Upgrade.accepts(Classification::Upgrade).is_ok());
        assert!(Intent::Upgrade.accepts(Classification::Current).is_ok());
    }

    #[test]
    fn reconcile_and_assess_serve_whatever_the_target_is() {
        for found in [
            Classification::Setup,
            Classification::Migration,
            Classification::Upgrade,
            Classification::Drift,
            Classification::Current,
            Classification::Invalid,
        ] {
            assert!(Intent::Reconcile.accepts(found).is_ok());
            assert!(Intent::Assess.accepts(found).is_ok());
        }
    }
}
