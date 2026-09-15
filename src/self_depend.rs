//! How a consumer obtains this tool, and how its pin stays fresh.
//!
//! A project pins this tool through the manager it already runs, at a
//! version its manager file records. This module reads that pin, serves the
//! fragment a manager needs to take one, and moves the pin to a newer
//! release in one transaction. It writes into a file the project owns only
//! where the project authorized it: the pin the sync line moves.
//!
//! Two independent axes decide every verdict. A manager is what the project
//! declares its development tools in ([`manager::Manager`]). A venue is where
//! a release of this tool is published ([`venue::Venue`]). Their cross is the
//! matrix in [`venue`], and every pair carries a verdict there and nowhere
//! else.

pub mod fragments;
pub mod leftovers;
pub mod manager;
pub mod pin;
pub mod registry;
pub mod stamp;
pub mod status;
pub mod txn;
pub mod venue;

use crate::domain::manifest::CANON_SOURCE;

/// The binary this crate installs, as an archive names it.
pub const BINARY_NAME: &str = "sdd";

/// The line a project's shell loader runs on every directory entry.
pub const SYNC_LINE: &str = "sdd self-depend sync --apply || true";

/// The shell loader file a project keeps at its root.
pub const ENVRC: &str = ".envrc";

/// The forge owner and repository this tool is published from, read from
/// the one place the canon's home is declared.
#[must_use]
pub fn coordinates() -> (&'static str, &'static str) {
    let path = CANON_SOURCE
        .trim_start_matches("https://github.com/")
        .trim_end_matches(".git");
    path.split_once('/').unwrap_or((path, ""))
}

/// The `owner/repo` slug the forge and the flake reference use.
#[must_use]
pub fn slug() -> String {
    let (owner, repo) = coordinates();
    format!("{owner}/{repo}")
}

/// The version a release tag names, from any of the spellings a caller
/// uses: `v0.2.16`, `0.2.16`, or the release page URL ending in the tag.
///
/// # Errors
///
/// A message naming the value where it is not a released triple.
pub fn parse_tag(value: &str) -> Result<semver::Version, String> {
    let tail = value.trim().rsplit('/').next().unwrap_or(value).trim();
    let bare = tail.strip_prefix('v').unwrap_or(tail);
    let version: semver::Version = bare
        .parse()
        .map_err(|_| format!("{value} is not a release tag: expected v<major>.<minor>.<patch>"))?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err(format!("{value} is not a released triple"));
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a test panics as its failure signal")]

    use super::*;

    #[test]
    fn the_coordinates_come_from_the_canon_source() {
        let (owner, repo) = coordinates();
        assert!(!owner.is_empty());
        assert_eq!(repo, "spec-driven-docs");
        assert!(CANON_SOURCE.ends_with(&slug()));
    }

    #[test]
    fn a_tag_parses_in_every_spelling() {
        assert_eq!(parse_tag("v0.2.16").unwrap().to_string(), "0.2.16");
        assert_eq!(parse_tag("0.2.16").unwrap().to_string(), "0.2.16");
        assert_eq!(
            parse_tag("https://example.invalid/releases/tag/v1.2.3")
                .unwrap()
                .to_string(),
            "1.2.3"
        );
        assert!(parse_tag("v1.2.3-rc.1").is_err());
        assert!(parse_tag("latest").is_err());
    }
}
