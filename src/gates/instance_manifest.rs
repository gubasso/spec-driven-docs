//! Gate: the instance manifest is present and fits the schema.
//!
//! The manifest is what every other ownership check reads, so a shape error
//! in it disables them all at once. Only the shape is judged here; comparing
//! the recorded hashes to the disk is `sdd verify`, which the managed block
//! wires as its own hook.

use crate::domain::finding::Finding;
use crate::domain::manifest::{MANIFEST_PATH, Manifest};
use crate::domain::rule_id::RuleId;
use crate::gates::{GateCtx, GateResult, Violation};

/// The rules this gate can cite.
pub const CITES: &[RuleId] = &[RuleId::TheManifestStaysReadable];

/// Judge the manifest's shape.
///
/// # Errors
///
/// None; an unreadable manifest is the violation itself.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    let violation = |detail: &str| {
        vec![Violation::Finding(Finding::on_file(
            RuleId::TheManifestStaysReadable,
            MANIFEST_PATH,
            detail,
        ))]
    };
    // The manifest is this gate's subject here, not support: it judges the
    // file's own shape.
    if ctx.retained([MANIFEST_PATH]).is_empty() {
        return Ok(vec![]);
    }
    let Ok(text) = std::fs::read_to_string(ctx.path(MANIFEST_PATH)) else {
        return Ok(violation("invalid manifest shape"));
    };
    // An older schema is read for what it records; the version gate warns
    // that `sdd upgrade` moves it.
    match Manifest::parse_any(&text) {
        Ok(_) => Ok(vec![]),
        Err(error) => Ok(violation(&format!("invalid manifest shape: {error}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_with_manifest(json: Option<&str>) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        if let Some(json) = json {
            let instance = dir.path().join(".spec-driven-docs");
            std::fs::create_dir_all(&instance).unwrap();
            std::fs::write(instance.join("manifest.json"), json).unwrap();
        }
        let ctx = GateCtx::new(dir.path().to_str().unwrap());
        run(&ctx, &[])
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    const VALID: &str = r#"{
        "schema_version": 3,
        "canon_version": "0.2.0",
        "canon_source": "https://github.com/gubasso/spec-driven-docs",
        "profile": "knowledge-base",
        "docs_root": "_docs",
        "installed_at": "2026-08-25T00:00:00Z",
        "managed_files": [
            {"source": ".markdownlint/spec.markdownlint-cli2.jsonc",
             "destination": ".spec-driven-docs/markdownlint/spec.markdownlint-cli2.jsonc",
             "sha256": "dc17d596ae2c196cc01b439c291416f91198cc274e2376fd01a4d614c1ff60ad"}
        ],
        "adopted_files": [],
        "integration_blocks": []
    }"#;

    #[test]
    fn accepts_a_current_schema_manifest() {
        assert!(run_with_manifest(Some(VALID)).is_empty());
    }

    #[test]
    fn rejects_a_missing_or_corrupt_manifest() {
        let missing = run_with_manifest(None);
        assert_eq!(missing.len(), 1);
        assert!(missing[0].contains("instance:the-manifest-stays-readable"));

        let corrupt = run_with_manifest(Some("{ not json"));
        assert_eq!(corrupt.len(), 1);
        assert!(corrupt[0].contains("invalid manifest shape"));
    }

    /// VERIFIES setup:a-gated-command-names-its-fix
    ///
    /// After a pin move the hook keeps passing on a record of an older
    /// schema; the version gate warns that `sdd upgrade` moves it.
    #[test]
    fn the_instance_manifest_gate_passes_on_an_older_schema() {
        let older = VALID.replace("\"schema_version\": 3", "\"schema_version\": 1");
        assert!(run_with_manifest(Some(&older)).is_empty());
    }
}
