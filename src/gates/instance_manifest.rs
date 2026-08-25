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
pub const CITES: &[RuleId] = &[RuleId::ManifestIdentifiesEveryOwnedFile];

/// Judge the manifest's shape.
///
/// # Errors
///
/// None; an unreadable manifest is the violation itself.
pub fn run(ctx: &GateCtx, _files: &[String]) -> GateResult {
    let violation = |detail: &str| {
        vec![Violation::Finding(Finding::on_file(
            RuleId::ManifestIdentifiesEveryOwnedFile,
            MANIFEST_PATH,
            detail,
        ))]
    };
    let Ok(text) = std::fs::read_to_string(ctx.path(MANIFEST_PATH)) else {
        return Ok(violation("invalid manifest shape"));
    };
    match Manifest::parse(&text) {
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
        "schema_version": 2,
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
    fn accepts_a_schema_two_manifest() {
        assert!(run_with_manifest(Some(VALID)).is_empty());
    }

    #[test]
    fn rejects_a_missing_or_older_manifest() {
        let missing = run_with_manifest(None);
        assert_eq!(missing.len(), 1);
        assert!(missing[0].contains("distribution:manifest-identifies-every-owned-file"));

        let older = VALID.replace("\"schema_version\": 2", "\"schema_version\": 1");
        let out = run_with_manifest(Some(&older));
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("invalid manifest shape: manifest schema_version 1"));
    }
}
