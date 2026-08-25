//! Compile-time embedded assets: the payload and the method, in the binary.
//!
//! Every `include_dir!`/`include_str!` in the crate lives here, and each
//! embeds from its canonical authored path, so the repository file and the
//! shipped copy cannot diverge — the build reads the real thing. This module
//! only holds bytes and typed accessors; deciding where an asset lands in an
//! instance is the profiles' and installer's business.

use std::collections::BTreeSet;

use include_dir::{Dir, include_dir};

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

/// The combined license statement naming both halves.
pub static LICENSE: &str = include_str!("../LICENSE");
/// The MIT license covering the distribution.
pub static LICENSE_MIT: &str = include_str!("../LICENSE-MIT");
/// The CC BY 4.0 license covering the method.
pub static LICENSE_CC_BY: &str = include_str!("../LICENSE-CC-BY-4.0");

const SOURCE_ROOTS: &[(&str, &Dir<'static>)] = &[
    ("_docs/specs/", &SPECS),
    ("templates/", &TEMPLATES),
    (".markdownlint/", &MARKDOWNLINT),
    ("instance/snippets/", &SNIPPETS),
];

/// Resolve a payload source path — as a profile projection names it — to its
/// embedded bytes.
#[must_use]
pub fn asset(source: &str) -> Option<&'static [u8]> {
    SOURCE_ROOTS.iter().find_map(|(prefix, dir)| {
        let rest = source.strip_prefix(prefix)?;
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
