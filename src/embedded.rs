//! Compile-time embedded assets: the payload and the method, in the binary.
//!
//! Every `include_dir!`/`include_str!` in the crate lives here, and each
//! embeds from its canonical authored path, so the repository file and the
//! shipped copy cannot diverge — the build reads the real thing. This module
//! only holds bytes and typed accessors; deciding where an asset lands in an
//! instance is the profiles' and installer's business.

use std::collections::BTreeSet;

use include_dir::{Dir, include_dir};

pub use crate::payload_roots::PAYLOAD_ROOTS;

/// The spec seeds an instance adopts, and the canon-only specs beside them.
pub static SPECS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/_docs/specs");
/// The stable document templates.
pub static TEMPLATES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates");
/// The markdownlint configurations the instance receives managed.
pub static MARKDOWNLINT: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/.markdownlint");
/// Integration snippets a consumer copies into their own files.
pub static SNIPPETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/instance/snippets");
/// The method chapters and glossary.
pub static METHOD: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/method");
/// The cross-agent skills, one `SKILL.md` per directory.
pub static SKILLS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/skills");
/// The artifacts every skill shares, installed once outside the skill roots.
pub static SKILL_SHARED: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/skill-shared");

/// The combined license statement naming both halves.
pub static LICENSE: &str = include_str!("../LICENSE");
/// The MIT license covering the distribution.
pub static LICENSE_MIT: &str = include_str!("../LICENSE-MIT");
/// The CC BY 4.0 license covering the method.
pub static LICENSE_CC_BY: &str = include_str!("../LICENSE-CC-BY-4.0");

/// Every embedded root paired with the authored path it came from, in
/// [`PAYLOAD_ROOTS`] order. A unit test holds the two equal, so a root
/// embedded here but missing from the declaration — or the reverse — fails
/// the build rather than shipping unscanned.
const EMBEDDED_ROOTS: &[(&str, &Dir<'static>)] = &[
    ("_docs/specs", &SPECS),
    ("templates", &TEMPLATES),
    (".markdownlint", &MARKDOWNLINT),
    ("instance/snippets", &SNIPPETS),
    ("method", &METHOD),
    ("skills", &SKILLS),
    ("skill-shared", &SKILL_SHARED),
];

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

/// Every artifact the skills share, as `(path under the root, bytes)`,
/// sorted by path.
///
/// These land once, outside the agent skill roots, because every skill names
/// the same absolute path for them. A copy per skill would be one file to
/// correct per agent root per skill; one copy is one.
#[must_use]
pub fn shared_artifacts() -> Vec<(String, &'static [u8])> {
    fn walk(dir: &Dir<'static>, out: &mut Vec<(String, &'static [u8])>) {
        for file in dir.files() {
            if let Some(path) = file.path().to_str() {
                out.push((path.to_string(), file.contents()));
            }
        }
        for sub in dir.dirs() {
            walk(sub, out);
        }
    }
    let mut out = Vec::new();
    walk(&SKILL_SHARED, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Resolve a payload source path — as a profile projection names it — to its
/// embedded bytes.
#[must_use]
pub fn asset(source: &str) -> Option<&'static [u8]> {
    EMBEDDED_ROOTS.iter().find_map(|(root, dir)| {
        let rest = source.strip_prefix(root)?.strip_prefix('/')?;
        dir.get_file(rest).map(include_dir::File::contents)
    })
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
        let embedded: Vec<&str> = EMBEDDED_ROOTS.iter().map(|(root, _)| *root).collect();
        assert_eq!(
            embedded,
            PAYLOAD_ROOTS.to_vec(),
            "payload_roots.rs and the embedded statics disagree; a root missing from the declaration ships unscanned"
        );
    }

    #[test]
    fn every_profile_projection_resolves_to_an_embedded_asset() {
        for id in [ProfileId::Codebase, ProfileId::KnowledgeBase] {
            let profile = id.profile();
            for entry in profile.managed.iter().chain(profile.adopted) {
                assert!(
                    asset(entry.source).is_some(),
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

    #[test]
    fn rule_id_parser_matches_heading_shape_only() {
        let text = "### `a-b:c-d` — Title\n### `Bad:Id`\nplain ### `x:y`\n### `no-colon`\n";
        let ids: Vec<String> = rule_ids_in(text).collect();
        assert_eq!(ids, vec!["a-b:c-d".to_string()]);
    }
}
