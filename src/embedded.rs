//! Compile-time embedded assets: the payload and the method, in the binary.
//!
//! Every `include_dir!`/`include_str!` in the crate lives here, and each
//! embeds from its canonical authored path, so the repository file and the
//! shipped copy cannot diverge — the build reads the real thing. This module
//! only holds bytes and typed accessors; deciding where an asset lands in an
//! instance is the profiles' and installer's business.

use std::collections::BTreeSet;
use std::path::Path;

use include_dir::{Dir, include_dir};

pub use crate::payload_roots::PAYLOAD_ROOTS;

/// The spec seeds an instance adopts, and the canon-only specs beside them.
pub static SPECS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/_docs/specs");
/// The stable document templates.
pub static TEMPLATES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates");
/// The documentation root's lint configuration, which an instance receives
/// managed.
pub static MARKDOWNLINT: &[u8] = include_bytes!("../_docs/.markdownlint-cli2.jsonc");
/// What a release says about itself, plus what it seeds and splices.
///
/// One root rather than a root per subdirectory: the projection
/// declaration sits beside the seeds it describes, and a root per
/// subdirectory would make the declaration a one-file exception to the
/// payload inventory.
pub static INSTANCE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/instance");
/// The method chapters and glossary.
pub static METHOD: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/method");
/// The cross-agent skills, one `SKILL.md` per directory.
pub static SKILLS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/skills");
/// The artifacts every skill shares, materialized into every package.
pub static SKILL_SHARED: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/skill-shared");
/// The comparison documents, which a stage carries as reference material.
pub static COMPARISON_DOCS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/comparison-docs");
/// The prior-art shelf, which a stage carries as reference material.
pub static PRIOR_ART: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/reference/prior-art");
/// The tracker-markup shelf, which the format chapter routes a reader to.
pub static TRACKER_MARKUP: Dir<'static> =
    include_dir!("$CARGO_MANIFEST_DIR/reference/tracker-markup");

/// The combined license statement naming both halves.
pub static LICENSE: &str = include_str!("../LICENSE");
/// The MIT license covering the distribution.
pub static LICENSE_MIT: &str = include_str!("../LICENSE-MIT");
/// The CC BY 4.0 license covering the method.
pub static LICENSE_CC_BY: &str = include_str!("../LICENSE-CC-BY-4.0");
/// The attribution notice for every third-party source the payload derives from.
pub static THIRD_PARTY_NOTICES: &str = include_str!("../THIRD_PARTY_NOTICES.md");
/// The release notes, which a stage carries as this version's own history.
pub static CHANGELOG: &str = include_str!("../CHANGELOG.md");

/// Every embedded directory root paired with the authored path it came
/// from. With [`EMBEDDED_FILES`] it is [`PAYLOAD_ROOTS`] in order: a unit test
/// holds the union equal to the declaration, so a root embedded here but
/// missing from the declaration — or the reverse — fails the build rather
/// than shipping unscanned.
const EMBEDDED_ROOTS: &[(&str, &Dir<'static>)] = &[
    ("_docs/specs", &SPECS),
    ("templates", &TEMPLATES),
    ("instance", &INSTANCE),
    ("method", &METHOD),
    ("skills", &SKILLS),
    ("skill-shared", &SKILL_SHARED),
    ("comparison-docs", &COMPARISON_DOCS),
    ("reference/prior-art", &PRIOR_ART),
    ("reference/tracker-markup", &TRACKER_MARKUP),
];

/// Every payload root that is one file, paired with its bytes.
///
/// A directory static cannot hold a file that sits beside other authored
/// content, such as a configuration at the top of the documentation root,
/// so such a root is embedded by itself.
const EMBEDDED_FILES: &[(&str, &[u8])] = &[("_docs/.markdownlint-cli2.jsonc", MARKDOWNLINT)];

/// Every embedded root paired with the authored path it came from.
///
/// The candidate reads its sources through this, so a root added to the
/// declaration reaches the projection without a second list.
#[must_use]
pub const fn roots() -> &'static [(&'static str, &'static Dir<'static>)] {
    EMBEDDED_ROOTS
}

/// Every skill name, sorted; a name is the skill's directory.
#[must_use]
pub fn skill_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = SKILLS
        .dirs()
        .filter_map(|dir| dir.path().as_os_str().to_str())
        .collect();
    names.sort_unstable();
    names
}

/// One skill's `SKILL.md` text, by skill name.
#[must_use]
pub fn skill(name: &str) -> Option<&'static str> {
    SKILLS
        .get_file(format!("{name}/SKILL.md"))
        .and_then(include_dir::File::contents_utf8)
}

/// Why a skill package cannot be composed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PackageError {
    /// The payload carries no skill by this name.
    #[error("the payload carries no skill named {0}")]
    Unknown(String),
    /// The skill and the shared source both claim one reference name.
    #[error(
        "{skill}: {path} is claimed by the skill and by the shared source; author it once (skill-format:shared-content-is-authored-once)"
    )]
    Collision {
        /// The skill whose package was being composed.
        skill: String,
        /// The package-relative path both sources claim.
        path: String,
    },
    /// The skill directory holds a file the package does not carry.
    #[error(
        "{skill}: {path} is neither SKILL.md nor a file under references/ (skill-format:a-package-holds-the-manual-and-its-references)"
    )]
    Unlisted {
        /// The skill whose package was being composed.
        skill: String,
        /// The skill-relative path outside the package closure.
        path: String,
    },
}

/// Every file of one installed skill package, as `(path relative to the
/// package root, bytes)`, sorted by path.
///
/// A package is the unit the Agent Skills format and every documented host
/// resolve against: one directory holding `SKILL.md` and supporting files
/// beside it. A skill's own reference files are authored in its directory,
/// and the shared artifacts are authored once under `skill-shared/` and
/// materialized here into every package, so a fix lands in one file and
/// reaches every root the installer writes.
///
/// # Errors
///
/// [`PackageError::Unknown`] for a name the payload does not carry, and the
/// composition errors of [`compose_package`].
pub fn skill_package(name: &str) -> Result<Vec<(String, &'static [u8])>, PackageError> {
    let dir = SKILLS
        .get_dir(name)
        .ok_or_else(|| PackageError::Unknown(name.to_string()))?;
    compose_package(name, dir, &SKILL_SHARED)
}

/// Compose one package from a skill directory and the shared source.
///
/// The closure is an allowlist: `SKILL.md` and the files directly under the
/// skill's `references/`, then every shared artifact under `references/`.
/// A file outside the allowlist, or a reference name both sources claim, is
/// refused rather than dropped or shadowed, because either would ship a
/// package that differs from what its author wrote.
///
/// # Errors
///
/// [`PackageError::Unknown`] where the directory holds no `SKILL.md`,
/// [`PackageError::Unlisted`] for a file outside the allowlist, and
/// [`PackageError::Collision`] for a name both sources claim.
pub fn compose_package(
    name: &str,
    skill: &'static Dir<'static>,
    shared: &'static Dir<'static>,
) -> Result<Vec<(String, &'static [u8])>, PackageError> {
    use crate::domain::paths::{SKILL_FILE, SKILL_REFERENCES_DIR};

    let mut files = Vec::new();
    for (relative, bytes) in files_relative_to(skill) {
        let allowed = relative == SKILL_FILE
            || relative
                .strip_prefix(&format!("{SKILL_REFERENCES_DIR}/"))
                .is_some_and(|file| !file.is_empty() && !file.contains('/'));
        if !allowed {
            return Err(PackageError::Unlisted {
                skill: name.to_string(),
                path: relative,
            });
        }
        files.push((relative, bytes));
    }
    if !files.iter().any(|(path, _)| path == SKILL_FILE) {
        return Err(PackageError::Unknown(name.to_string()));
    }
    for (path, bytes) in files_relative_to(shared) {
        let relative = format!("{SKILL_REFERENCES_DIR}/{path}");
        if files.iter().any(|(claimed, _)| *claimed == relative) {
            return Err(PackageError::Collision {
                skill: name.to_string(),
                path: relative,
            });
        }
        files.push((relative, bytes));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

/// Every file under `dir`, at any depth, by its path relative to `dir`.
fn files_relative_to(dir: &'static Dir<'static>) -> Vec<(String, &'static [u8])> {
    fn walk(base: &Path, dir: &'static Dir<'static>, out: &mut Vec<(String, &'static [u8])>) {
        for file in dir.files() {
            let relative = file
                .path()
                .strip_prefix(base)
                .unwrap_or_else(|_| file.path());
            if let Some(relative) = relative.to_str() {
                out.push((relative.to_string(), file.contents()));
            }
        }
        for sub in dir.dirs() {
            walk(base, sub, out);
        }
    }
    let mut out = Vec::new();
    walk(dir.path(), dir, &mut out);
    out
}

/// Every artifact the skills share, as `(path under the root, bytes)`,
/// sorted by path.
///
/// This is the authored view. What lands is [`skill_package`], which copies
/// each of these into every package as a reference relative to the skill's
/// own root.
#[must_use]
pub fn shared_artifacts() -> Vec<(String, &'static [u8])> {
    let mut out = files_relative_to(&SKILL_SHARED);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Resolve a payload source path — as a profile projection names it — to its
/// embedded bytes.
#[must_use]
pub fn asset(source: &str) -> Option<&'static [u8]> {
    EMBEDDED_FILES
        .iter()
        .find_map(|(path, bytes)| (*path == source).then_some(*bytes))
        .or_else(|| {
            EMBEDDED_ROOTS.iter().find_map(|(root, dir)| {
                let rest = source.strip_prefix(root)?.strip_prefix('/')?;
                dir.get_file(rest).map(include_dir::File::contents)
            })
        })
}

/// Every file under one embedded root, by the logical path that names it.
///
/// A root that is one file yields that file alone. The paths come back
/// sorted, so a caller that copies them writes the same tree every time.
#[must_use]
pub fn assets_under(root: &str) -> Vec<(String, &'static [u8])> {
    if let Some((path, bytes)) = EMBEDDED_FILES.iter().find(|(path, _)| *path == root) {
        return vec![((*path).to_string(), *bytes)];
    }
    let Some((name, dir)) = EMBEDDED_ROOTS.iter().find(|(name, _)| *name == root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_under(name, dir, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn collect_under(root: &str, dir: &'static Dir<'static>, out: &mut Vec<(String, &'static [u8])>) {
    for file in dir.files() {
        if let Some(rest) = file.path().to_str() {
            out.push((format!("{root}/{rest}"), file.contents()));
        }
    }
    for sub in dir.dirs() {
        collect_under(root, sub, out);
    }
}

/// Every `` ### `domain:rule` `` requirement address the embedded specs define.
#[must_use]
pub fn spec_rule_ids() -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for file in SPECS.files() {
        let Some(text) = file.contents_utf8() else {
            continue;
        };
        ids.extend(rule_ids_in(text));
    }
    ids
}

/// The text of every spec this binary carries.
pub fn spec_texts() -> impl Iterator<Item = &'static str> {
    SPECS.files().filter_map(include_dir::File::contents_utf8)
}

/// The requirement addresses one spec document defines.
pub fn rule_ids_in(text: &str) -> impl Iterator<Item = String> + '_ {
    text.lines().filter_map(|line| {
        let candidate = line.strip_prefix("### `")?;
        let (id, _) = candidate.split_once('`')?;
        let (domain, rule) = id.split_once(':')?;
        let is_slug = |part: &str| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        };
        (is_slug(domain) && is_slug(rule)).then(|| id.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::profile::ProfileId;
    use crate::domain::rule_id::RuleId;

    #[test]
    fn rule_id_enum_matches_the_embedded_specs() {
        let from_specs = spec_rule_ids();
        let from_enum: BTreeSet<String> = RuleId::ALL
            .iter()
            .map(|rule| rule.as_str().to_string())
            .collect();
        assert_eq!(
            from_specs, from_enum,
            "RuleId and the specs disagree; update the enum and the specs together"
        );
    }

    #[test]
    fn the_embedded_roots_are_the_declared_payload_roots() {
        let mut embedded: Vec<&str> = EMBEDDED_ROOTS
            .iter()
            .map(|(root, _)| *root)
            .chain(EMBEDDED_FILES.iter().map(|(path, _)| *path))
            .collect();
        embedded.sort_unstable();
        let mut declared = PAYLOAD_ROOTS.to_vec();
        declared.sort_unstable();
        assert_eq!(
            embedded, declared,
            "payload_roots.rs and the embedded statics disagree; a root missing from the declaration ships unscanned"
        );
    }

    #[test]
    fn a_single_file_root_resolves_through_both_readers() {
        for (path, bytes) in EMBEDDED_FILES {
            assert_eq!(asset(path), Some(*bytes), "{path}");
            assert_eq!(assets_under(path), vec![((*path).to_string(), *bytes)]);
        }
        assert!(asset("_docs/.markdownlint-cli2.jsonc/x").is_none());
    }

    #[test]
    fn every_profile_projection_resolves_to_an_embedded_asset() {
        for id in [ProfileId::Codebase, ProfileId::KnowledgeBase] {
            let profile = id.profile();
            for entry in profile.managed.iter().chain(profile.adopted) {
                assert!(
                    asset(&entry.source).is_some(),
                    "{id}: {} is not embedded",
                    entry.source
                );
            }
        }
    }

    #[test]
    fn the_method_and_licenses_are_carried() {
        assert!(METHOD.get_file("glossary.md").is_some());
        assert!(METHOD.files().count() >= 15);
        assert!(LICENSE.contains("LICENSE-MIT") && LICENSE.contains("LICENSE-CC-BY-4.0"));
    }

    use include_dir::{DirEntry, File};

    static SHARED: Dir<'static> = Dir::new(
        "",
        &[DirEntry::File(File::new("plan-gate.md", b"shared gate"))],
    );
    static OWN_REFERENCE: Dir<'static> = Dir::new(
        "demo",
        &[
            DirEntry::File(File::new("demo/SKILL.md", b"manual")),
            DirEntry::Dir(Dir::new(
                "demo/references",
                &[DirEntry::File(File::new(
                    "demo/references/land.md",
                    b"task",
                ))],
            )),
        ],
    );
    static CLAIMED_TWICE: Dir<'static> = Dir::new(
        "demo",
        &[
            DirEntry::File(File::new("demo/SKILL.md", b"manual")),
            DirEntry::Dir(Dir::new(
                "demo/references",
                &[DirEntry::File(File::new(
                    "demo/references/plan-gate.md",
                    b"a copy",
                ))],
            )),
        ],
    );
    static UNLISTED: Dir<'static> = Dir::new(
        "demo",
        &[
            DirEntry::File(File::new("demo/SKILL.md", b"manual")),
            DirEntry::Dir(Dir::new(
                "demo/scripts",
                &[DirEntry::File(File::new("demo/scripts/check.sh", b"echo"))],
            )),
        ],
    );

    /// SATISFIES distribution:a-skill-package-is-self-contained
    #[test]
    fn a_skill_reference_lands_beside_the_shared_artifacts() {
        let package = compose_package("demo", &OWN_REFERENCE, &SHARED).unwrap();
        let paths: Vec<&str> = package.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(
            paths,
            ["SKILL.md", "references/land.md", "references/plan-gate.md"]
        );
    }

    /// VERIFIES skill-format:shared-content-is-authored-once
    #[test]
    fn a_name_both_sources_claim_fails_the_package() {
        let error = compose_package("demo", &CLAIMED_TWICE, &SHARED).unwrap_err();
        assert_eq!(
            error,
            PackageError::Collision {
                skill: "demo".to_string(),
                path: "references/plan-gate.md".to_string(),
            }
        );
        assert!(
            error
                .to_string()
                .contains("skill-format:shared-content-is-authored-once")
        );
    }

    /// VERIFIES skill-format:a-package-holds-the-manual-and-its-references
    #[test]
    fn a_file_outside_the_allowlist_fails_the_package() {
        let error = compose_package("demo", &UNLISTED, &SHARED).unwrap_err();
        assert_eq!(
            error,
            PackageError::Unlisted {
                skill: "demo".to_string(),
                path: "scripts/check.sh".to_string(),
            }
        );
    }

    #[test]
    fn a_name_the_payload_lacks_is_unknown() {
        assert_eq!(
            skill_package("no-such-skill"),
            Err(PackageError::Unknown("no-such-skill".to_string()))
        );
    }

    #[test]
    fn rule_id_parser_matches_heading_shape_only() {
        let text = "### `a-b:c-d` — Title\n### `Bad:Id`\nplain ### `x:y`\n### `no-colon`\n";
        let ids: Vec<String> = rule_ids_in(text).collect();
        assert_eq!(ids, vec!["a-b:c-d".to_string()]);
    }
}
