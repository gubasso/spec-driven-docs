//! The Markdown linter the delivered configuration is written for.
//!
//! One owner for what this release knows about markdownlint-cli2: the two
//! configuration families its discovery reads in a directory, the hook
//! revision the managed block pins, and the relative-links rule package.
//!
//! The rule version is named a second time in `nix/markdownlint.nix`, which
//! bundles the rule beside the devshell linter. Nix cannot read a Rust
//! constant, so a canon test holds the two equal, and holds the rendered
//! hook's dependency to the same value.

/// The `.markdownlint-cli2.*` names the linter discovers in a directory.
///
/// A file of this family carries options as well as rules: `customRules`,
/// `ignores`, and `overrides` live here and nowhere else.
pub const CLI2_NAMES: [&str; 4] = [
    ".markdownlint-cli2.jsonc",
    ".markdownlint-cli2.yaml",
    ".markdownlint-cli2.cjs",
    ".markdownlint-cli2.mjs",
];

/// The `.markdownlint.*` names the linter discovers in a directory.
///
/// A file of this family holds rule configuration alone, and it replaces the
/// `config` of a `.markdownlint-cli2.*` file in the same directory.
pub const LIBRARY_NAMES: [&str; 6] = [
    ".markdownlint.jsonc",
    ".markdownlint.json",
    ".markdownlint.yaml",
    ".markdownlint.yml",
    ".markdownlint.cjs",
    ".markdownlint.mjs",
];

/// Every discovery name of both families, CLI2 first.
pub fn discovery_names() -> impl Iterator<Item = &'static str> {
    CLI2_NAMES.into_iter().chain(LIBRARY_NAMES)
}

/// The linter's pre-commit repository.
pub const REPO: &str = "https://github.com/DavidAnson/markdownlint-cli2";

/// The linter revision the managed block pins.
pub const REV: &str = "v0.23.3";

/// The custom rule that judges relative links, by its npm name.
pub const RELATIVE_LINKS_PACKAGE: &str = "markdownlint-rule-relative-links";

/// The relative-links version this release pins, in the hook and in the
/// devshell linter alike.
pub const RELATIVE_LINKS_VERSION: &str = "5.1.3";

/// The pinned relative-links package as pre-commit installs it.
#[must_use]
pub fn relative_links_dependency() -> String {
    format!("{RELATIVE_LINKS_PACKAGE}@{RELATIVE_LINKS_VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_families_do_not_overlap_and_every_name_is_hidden() {
        for name in discovery_names() {
            assert!(name.starts_with(".markdownlint"), "{name}");
        }
        for name in CLI2_NAMES {
            assert!(!LIBRARY_NAMES.contains(&name), "{name}");
        }
        assert_eq!(discovery_names().count(), 10);
    }

    #[test]
    fn the_dependency_pins_the_version() {
        assert_eq!(
            relative_links_dependency(),
            "markdownlint-rule-relative-links@5.1.3"
        );
    }
}
