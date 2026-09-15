//! The venue axis, and the matrix it forms with the manager axis.
//!
//! A venue joins the list only where this repository's own release path
//! publishes to it: the crate release-plz publishes to the registry, the
//! flake this repository serves at every tag, and the archives cargo-dist
//! attaches to each forge release. Every manager and venue pair carries one
//! verdict below, and a manual pair carries a reason from a closed set.

use serde::Serialize;

use crate::self_depend::manager::Manager;

/// Where a release of this tool is published.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Venue {
    /// The crate on crates.io, built from source by the consumer.
    Crates,
    /// The flake this repository serves at every tag.
    Flake,
    /// The prebuilt archives attached to a forge release.
    GithubRelease,
}

impl Venue {
    /// Every venue, in the order a manager is offered them.
    pub const ALL: [Self; 3] = [Self::Crates, Self::Flake, Self::GithubRelease];

    /// The kebab-case word the command line and the report use.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Crates => "crates",
            Self::Flake => "flake",
            Self::GithubRelease => "github-release",
        }
    }
}

impl std::fmt::Display for Venue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why a pair is manual, from the closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    /// No package-set attribute is known for that manager.
    AttributeUnknown,
    /// A source hash the tool cannot compute offline is required.
    HashNeeded,
    /// The manager has no backend for that venue.
    NoBackend,
    /// No published plugin installs the tool.
    PluginUnknown,
}

impl Reason {
    /// The sentence a report prints.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AttributeUnknown => "no package-set attribute is known for this manager",
            Self::HashNeeded => "a source hash this tool cannot compute offline is required",
            Self::NoBackend => "the manager has no backend for this venue",
            Self::PluginUnknown => "no published plugin installs this tool",
        }
    }
}

/// What one manager and venue pair can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum Verdict {
    /// The tool renders the exact fragment the pair needs.
    Renders,
    /// The operator wires the pair by hand, for the stated reason.
    Manual {
        /// Why the tool renders nothing.
        reason: Reason,
    },
}

/// The one verdict for a pair.
#[must_use]
pub const fn verdict(manager: Manager, venue: Venue) -> Verdict {
    match (manager, venue) {
        (Manager::Flake | Manager::Devbox, Venue::Flake)
        | (Manager::Mise, Venue::Crates | Venue::GithubRelease) => Verdict::Renders,
        (Manager::Flake | Manager::Devbox, Venue::Crates) => Verdict::Manual {
            reason: Reason::AttributeUnknown,
        },
        (Manager::Flake | Manager::Devbox, Venue::GithubRelease) => Verdict::Manual {
            reason: Reason::HashNeeded,
        },
        (Manager::Mise, Venue::Flake) => Verdict::Manual {
            reason: Reason::NoBackend,
        },
        (Manager::Asdf, _) => Verdict::Manual {
            reason: Reason::PluginUnknown,
        },
    }
}

/// The first venue a manager renders a fragment for, in venue order.
#[must_use]
pub fn default_venue(manager: Manager) -> Option<Venue> {
    Venue::ALL
        .into_iter()
        .find(|venue| matches!(verdict(manager, *venue), Verdict::Renders))
}

/// One row of the matrix, as the report prints it.
#[derive(Debug, Clone, Serialize)]
pub struct Pair {
    /// The manager.
    pub manager: Manager,
    /// The venue.
    pub venue: Venue,
    /// What the pair can do.
    #[serde(flatten)]
    pub verdict: Verdict,
}

/// Every pair with its verdict, in manager then venue order.
#[must_use]
pub fn matrix() -> Vec<Pair> {
    let mut rows = Vec::new();
    for manager in Manager::ALL {
        for venue in Venue::ALL {
            rows.push(Pair {
                manager,
                venue,
                verdict: verdict(manager, venue),
            });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VERIFIES acquisition:every-pair-carries-a-verdict
    #[test]
    fn every_pair_carries_a_verdict_and_every_manual_pair_a_reason() {
        let rows = matrix();
        assert_eq!(rows.len(), Manager::ALL.len() * Venue::ALL.len());
        for row in &rows {
            match row.verdict {
                Verdict::Renders => {}
                Verdict::Manual { reason } => assert!(!reason.as_str().is_empty()),
            }
        }
    }

    #[test]
    fn the_rendering_pairs_are_the_four_this_release_proves() {
        let renders: Vec<(Manager, Venue)> = matrix()
            .into_iter()
            .filter(|row| row.verdict == Verdict::Renders)
            .map(|row| (row.manager, row.venue))
            .collect();
        assert_eq!(
            renders,
            [
                (Manager::Flake, Venue::Flake),
                (Manager::Mise, Venue::Crates),
                (Manager::Mise, Venue::GithubRelease),
                (Manager::Devbox, Venue::Flake),
            ]
        );
    }

    #[test]
    fn a_manager_defaults_to_its_first_rendering_venue() {
        assert_eq!(default_venue(Manager::Flake), Some(Venue::Flake));
        assert_eq!(default_venue(Manager::Mise), Some(Venue::Crates));
        assert_eq!(default_venue(Manager::Devbox), Some(Venue::Flake));
        assert_eq!(default_venue(Manager::Asdf), None);
    }
}
