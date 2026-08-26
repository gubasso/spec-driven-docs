//! Regenerate the canon's own instance manifest.
//!
//! The canon is an instance of itself, but not one the installer can
//! produce: its files sit where it authors them rather than under the
//! vendored directory. Recording their hashes by hand is how the manifest
//! drifts from the payload, so it is generated from the payload instead —
//! and only in the canon checkout, which is recognised by its own crate
//! manifest.

use camino::Utf8Path;

use crate::adapters::fs::sha256_file;
use crate::domain::manifest::{CANON_SOURCE, MANIFEST_PATH, Manifest, SCHEMA_VERSION};
use crate::domain::ownership::{AdoptedEntry, IntegrationBlock, ManagedEntry};
use crate::domain::profile::{DocsRoot, ProfileId};
use crate::domain::version::CanonVersion;
use crate::error::AppError;

pub(crate) fn is_canon_checkout(root: &Utf8Path) -> bool {
    std::fs::read_to_string(root.join("Cargo.toml"))
        .is_ok_and(|cargo| cargo.contains("name = \"spec-driven-docs\""))
}

fn sorted_files(root: &Utf8Path, dir: &str, matches: impl Fn(&str) -> bool) -> Vec<String> {
    let mut names: Vec<String> = root
        .join(dir)
        .read_dir_utf8()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string())
                .filter(|name| matches(name))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
        .into_iter()
        .map(|name| format!("{dir}/{name}"))
        .collect()
}

/// Every `skills/<name>/SKILL.md` path in the checkout, sorted by name.
fn skill_files(root: &Utf8Path) -> Vec<String> {
    let mut names: Vec<String> = root
        .join("skills")
        .read_dir_utf8()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
        .into_iter()
        .map(|name| format!("skills/{name}/SKILL.md"))
        .collect()
}

/// Regenerate `.spec-driven-docs/manifest.json` in the canon checkout.
///
/// # Errors
///
/// [`AppError::Refused`] outside the canon checkout, and I/O errors when a
/// recorded file cannot be read or the manifest cannot be written.
pub fn regenerate(root: &Utf8Path) -> Result<String, AppError> {
    if !is_canon_checkout(root) {
        return Err(AppError::Refused("not the canon checkout".to_string()));
    }

    let mut managed = Vec::new();
    // The payload convention is lowercase; the shell glob this replaces was
    // case-sensitive too.
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    let jsonc = |name: &str| name.ends_with(".jsonc");
    for path in sorted_files(root, ".markdownlint", jsonc) {
        managed.push(ManagedEntry {
            source: path.clone().into(),
            destination: path.clone().into(),
            sha256: sha256_file(&root.join(&path))?,
        });
    }
    for path in skill_files(root) {
        managed.push(ManagedEntry {
            source: path.clone().into(),
            destination: path.clone().into(),
            sha256: sha256_file(&root.join(&path))?,
        });
    }

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    let spec = |name: &str| name.starts_with("SPEC-") && name.ends_with(".md");
    let mut adopted_paths = sorted_files(root, "_docs/specs", spec);
    adopted_paths.push("_docs/decisions/TEMPLATE-adr.md".to_string());
    adopted_paths.push("_docs/reference/TEMPLATE-agents-digest.md".to_string());
    let mut adopted = Vec::new();
    for path in adopted_paths {
        let digest = sha256_file(&root.join(&path))?;
        adopted.push(AdoptedEntry {
            source: path.clone().into(),
            destination: path.into(),
            sha256: digest.clone(),
            baseline_sha256: digest,
        });
    }

    let config = std::fs::read_to_string(root.join(".pre-commit-config.yaml"))?;
    let marker_hash = crate::domain::marker::block_hash(&config).ok_or_else(|| {
        AppError::Refused("no managed block in .pre-commit-config.yaml".to_string())
    })?;

    let installed_at = std::fs::read_to_string(root.join(MANIFEST_PATH))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| {
            value
                .get("installed_at")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| {
            jiff::Timestamp::now()
                .strftime("%Y-%m-%dT%H:%M:%SZ")
                .to_string()
        });

    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        canon_version: CanonVersion::current(),
        canon_source: CANON_SOURCE.to_string(),
        profile: ProfileId::KnowledgeBase,
        docs_root: DocsRoot::UnderscoreDocs,
        installed_at,
        managed_files: managed,
        adopted_files: adopted,
        integration_blocks: vec![IntegrationBlock {
            path: ".pre-commit-config.yaml".into(),
            marker_hash,
        }],
    };
    crate::adapters::fs::write_file(&root.join(MANIFEST_PATH), manifest.to_json().as_bytes())?;
    Ok(format!("OK regenerated {MANIFEST_PATH}"))
}
