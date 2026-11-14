//! The fragments a rendering pair serves, and the files a seed writes.
//!
//! A fragment carries its anchor and its placement so a person or an agent
//! places it in a file the project owns. A seed is a whole file, written
//! only where the project has none.

use semver::Version;
use serde::Serialize;

use crate::self_depend::manager::Manager;
use crate::self_depend::registry::CRATE_NAME;
use crate::self_depend::venue::{Venue, Verdict, verdict};
use crate::self_depend::{BINARY_NAME, CHECK_LINE, ENVRC, SYNC_COMMAND, slug};

/// The name the flake input and the devshell reference share.
pub const FLAKE_INPUT: &str = "spec-driven-docs";

/// Why the flake input carries an empty follows line on its own name.
const SELF_FOLLOWS_WHY: [&str; 3] = [
    "Each release pins its previous release for its own development. The",
    "empty follows points that input at this flake, so flake.lock holds",
    "no node for it, and nothing here builds it.",
];

/// The comment above the empty follows line, at one indent.
fn self_follows_why(indent: &str) -> String {
    let mut out = String::new();
    for line in SELF_FOLLOWS_WHY {
        out.push_str(indent);
        out.push_str("# ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The empty follows line on this tool's own input, as a consumer's
/// flake carries it inside the input's attribute set.
pub const SELF_FOLLOWS: &str = "inputs.spec-driven-docs.follows = \"\";";

/// One fragment, with where it goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Fragment {
    /// The file the fragment belongs in, relative to the project root.
    pub file: String,
    /// The line the fragment is placed near.
    pub anchor: String,
    /// Where, relative to the anchor.
    pub placement: String,
    /// The fragment.
    pub text: String,
}

/// The flake reference for one tag.
fn flake_ref(version: &Version) -> String {
    format!("github:{}/v{version}", slug())
}

/// The fragments one rendering pair needs, or none for a manual pair.
#[must_use]
pub fn render(manager: Manager, venue: Venue, version: &Version) -> Vec<Fragment> {
    if verdict(manager, venue) != Verdict::Renders {
        return Vec::new();
    }
    match (manager, venue) {
        (Manager::Flake, Venue::Flake) => vec![
            Fragment {
                file: "flake.nix".to_string(),
                anchor: "inputs = {".to_string(),
                placement: "inside the inputs attribute set".to_string(),
                // SATISFIES acquisition:a-fragment-names-the-venue-form
                text: format!(
                    "{FLAKE_INPUT} = {{\n  url = \"{}\";\n  inputs.nixpkgs.follows = \"nixpkgs\";\n{}  {SELF_FOLLOWS}\n}};\n",
                    flake_ref(version),
                    self_follows_why("  ")
                ),
            },
            Fragment {
                file: "flake.nix".to_string(),
                anchor: "packages = [".to_string(),
                placement: format!(
                    "inside the devshell's packages list, with `{FLAKE_INPUT}` added to the outputs arguments"
                ),
                text: format!("{FLAKE_INPUT}.packages.${{system}}.default\n"),
            },
        ],
        (Manager::Mise, Venue::Crates) => vec![Fragment {
            file: "mise.toml".to_string(),
            anchor: "[tools]".to_string(),
            placement: "under the tools table".to_string(),
            text: format!("\"cargo:{CRATE_NAME}\" = \"{version}\"\n"),
        }],
        (Manager::Mise, Venue::GithubRelease) => vec![Fragment {
            file: "mise.toml".to_string(),
            anchor: "[tools]".to_string(),
            placement: "under the tools table".to_string(),
            text: format!(
                "\"ubi:{}\" = {{ version = \"{version}\", exe = \"{BINARY_NAME}\" }}\n",
                slug()
            ),
        }],
        (Manager::Devbox, Venue::Flake) => vec![Fragment {
            file: "devbox.json".to_string(),
            anchor: "\"packages\": [".to_string(),
            placement: "inside the packages array".to_string(),
            text: format!("\"{}#default\"\n", flake_ref(version)),
        }],
        _ => Vec::new(),
    }
}

/// The shell-entry line, as a fragment.
#[must_use]
pub fn envrc_fragment() -> Fragment {
    Fragment {
        file: ENVRC.to_string(),
        anchor: "use flake".to_string(),
        placement: "after the line that loads the environment".to_string(),
        text: format!("{CHECK_LINE}\n"),
    }
}

/// The whole manager file a seed writes for one rendering pair.
#[must_use]
pub fn seed(manager: Manager, venue: Venue, version: &Version) -> Option<(String, String)> {
    if verdict(manager, venue) != Verdict::Renders {
        return None;
    }
    let file = manager.files().first()?.to_string();
    let contents = match (manager, venue) {
        (Manager::Flake, Venue::Flake) => format!(
            "{{\n  description = \"Development shell\";\n\n  inputs = {{\n    nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\";\n    {FLAKE_INPUT} = {{\n      # Moved by `{SYNC_COMMAND}`, which the operator runs: the tag in\n      # this URL is the version, and flake.lock is the content pin.\n      url = \"{}\";\n      inputs.nixpkgs.follows = \"nixpkgs\";\n{}      {SELF_FOLLOWS}\n    }};\n  }};\n\n  outputs =\n    {{ nixpkgs, {FLAKE_INPUT}, ... }}:\n    let\n      system = \"x86_64-linux\";\n      pkgs = nixpkgs.legacyPackages.${{system}};\n    in\n    {{\n      devShells.${{system}}.default = pkgs.mkShell {{\n        packages = [\n          {FLAKE_INPUT}.packages.${{system}}.default\n        ];\n      }};\n    }};\n}}\n",
            flake_ref(version),
            self_follows_why("      ")
        ),
        (Manager::Mise, _) => {
            let entry = render(manager, venue, version)
                .into_iter()
                .map(|fragment| fragment.text)
                .collect::<String>();
            format!("[tools]\n{entry}")
        }
        (Manager::Devbox, Venue::Flake) => format!(
            "{{\n  \"packages\": [\n    \"{}#default\"\n  ]\n}}\n",
            flake_ref(version)
        ),
        _ => return None,
    };
    Some((file, contents))
}

/// The whole shell loader a seed writes, for the flake pair alone.
#[must_use]
pub fn envrc_seed() -> String {
    format!(
        "use flake\n\n# Warn on stderr when a newer {CRATE_NAME} release is out. The check reads\n# the network at most once every seven days, writes no file but its cache,\n# and exits 0. To move the pin, run: {SYNC_COMMAND}\n{CHECK_LINE}\n"
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test panics as its failure signal")]

    use super::*;
    use crate::self_depend::pin;

    fn version() -> Version {
        "0.10.2".parse().unwrap()
    }

    /// VERIFIES acquisition:a-fragment-names-the-venue-form
    #[test]
    fn a_registry_fragment_names_the_crate_and_an_archive_fragment_the_binary() {
        let registry = render(Manager::Mise, Venue::Crates, &version());
        assert!(registry[0].text.contains(&format!("cargo:{CRATE_NAME}")));
        assert!(!registry[0].text.contains(BINARY_NAME));
        let archive = render(Manager::Mise, Venue::GithubRelease, &version());
        assert!(
            archive[0]
                .text
                .contains(&format!("exe = \"{BINARY_NAME}\""))
        );
    }

    #[test]
    fn a_manual_pair_renders_nothing_and_seeds_nothing() {
        assert!(render(Manager::Asdf, Venue::Crates, &version()).is_empty());
        assert!(seed(Manager::Flake, Venue::Crates, &version()).is_none());
    }

    #[test]
    fn every_seed_reads_back_as_a_pin_at_the_seeded_version() {
        for (manager, venue) in [
            (Manager::Flake, Venue::Flake),
            (Manager::Mise, Venue::Crates),
            (Manager::Mise, Venue::GithubRelease),
            (Manager::Devbox, Venue::Flake),
        ] {
            let (file, contents) = seed(manager, venue, &version()).unwrap();
            assert_eq!(file, manager.files()[0]);
            let held = pin::read(manager, &contents).unwrap();
            assert_eq!(held.version, version(), "{manager} {venue}");
            assert_eq!(held.venue, Some(venue));
        }
        assert_eq!(
            pin::input_name(&seed(Manager::Flake, Venue::Flake, &version()).unwrap().1).as_deref(),
            Some(FLAKE_INPUT)
        );
    }

    /// VERIFIES acquisition:a-fragment-names-the-venue-form
    #[test]
    fn the_flake_fragment_and_the_seed_carry_the_empty_follows_line() {
        let fragment = &render(Manager::Flake, Venue::Flake, &version())[0].text;
        assert_eq!(fragment.matches(SELF_FOLLOWS).count(), 1, "{fragment}");
        let (_, seeded) = seed(Manager::Flake, Venue::Flake, &version()).unwrap();
        assert_eq!(seeded.matches(SELF_FOLLOWS).count(), 1, "{seeded}");
        assert!(seeded.contains("      # Each release pins its previous release"));
        assert!(
            seeded.contains("\n      # empty follows points"),
            "{seeded}"
        );
    }

    #[test]
    fn the_shell_loader_seed_carries_the_check_line_once() {
        assert_eq!(envrc_seed().matches(CHECK_LINE).count(), 1);
        assert_eq!(envrc_fragment().text.trim(), CHECK_LINE);
        assert!(!envrc_seed().contains(crate::self_depend::OLD_SYNC_LINE));
    }
}
