//! `hooks` subcommand: runtime-shape.
//!
//! Renders the managed block from the registry and the project's
//! declaration, and writes it into the target's configuration on request.
//! What the render contains is the renderer's business; this handler
//! projects the flags, resolves the declaration, and owns the write.
//!
//! `--apply` exists because nothing else reaches the block after an ordinary
//! declaration edit. `sdd upgrade` returns early at the same version, and a
//! render to stdout changes no file, so without this a project could edit
//! its declaration and see nothing happen. The declaration also selects the
//! writing-style route the documentation block in `AGENTS.md` carries, so
//! `--apply` and `--check` reach that block too: one verb brings every
//! managed region back into agreement with the declaration.

use camino::Utf8Path;

use crate::adapters::fs::write_atomic;
use crate::cli::hooks::HooksArgs;
use crate::context::AppContext;
use crate::domain::instance_config::InstanceConfig;
use crate::domain::marker;
use crate::domain::ownership::Sha256;
use crate::error::AppError;
use crate::output;
use crate::services::hooks_render::{RenderOptions, render_block};

/// Where a pre-commit configuration lives in a target.
pub const CONFIG: &str = ".pre-commit-config.yaml";

/// Where the documentation block lives in a target.
pub const AGENTS: &str = "AGENTS.md";

/// Update the manifest's record of one managed region.
fn record_block_hash(target: &Utf8Path, path: &str, hash: Option<Sha256>) -> Result<(), AppError> {
    let manifest_path = target.join(".spec-driven-docs/manifest.json");
    let Ok(text) = std::fs::read_to_string(&manifest_path) else {
        // No manifest: the target is not an instance, and nothing records
        // the region. The rewrite still stands.
        return Ok(());
    };
    let Some(hash) = hash else {
        return Ok(());
    };
    let mut document: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;
    if let Some(blocks) = document
        .get_mut("integration_blocks")
        .and_then(serde_json::Value::as_array_mut)
    {
        for block in blocks.iter_mut() {
            if block.get("path").and_then(serde_json::Value::as_str) == Some(path) {
                block["marker_hash"] = serde_json::Value::String(hash.to_string());
            }
        }
    }
    let rendered = serde_json::to_string_pretty(&document)
        .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;
    write_atomic(&manifest_path, format!("{rendered}\n").as_bytes())?;
    Ok(())
}

/// The documentation block `AGENTS.md` should carry for a declaration, and
/// what it carries now, where the file holds a managed block at all.
///
/// A host with no block is left alone: this repository's own root digest
/// carries none, and a project that removed the block by hand has made a
/// choice `sdd verify` reports as a missing integration block already.
fn agents_rewrite(
    target: &Utf8Path,
    docs_root: &str,
    declaration: &InstanceConfig,
) -> Result<Option<(String, String)>, AppError> {
    let path = target.join(AGENTS);
    if path.is_symlink() {
        return Err(AppError::Refused(
            "AGENTS.md is a symlink; refusing to write the documentation block through it"
                .to_string(),
        ));
    }
    let Ok(host) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    if marker::block_region_with(&host, marker::AGENTS_BEGIN, marker::AGENTS_END).is_none() {
        return Ok(None);
    }
    let block = crate::services::agents_render::render_block(docs_root, &declaration.writing_style);
    let placed = marker::place_agents_block(&host, &block)?;
    Ok(Some((host, placed)))
}

/// Render the delivered gate set, and optionally write it.
///
/// # Errors
///
/// [`AppError::Usage`] when the declaration does not parse,
/// [`AppError::Marker`] when the target's managed region is malformed, and
/// [`AppError::Violations`] when `--check` finds the region stale.
pub fn run(_ctx: &AppContext, args: HooksArgs) -> Result<(), AppError> {
    let target = Utf8Path::new(&args.target);
    let declaration =
        InstanceConfig::read(target).map_err(|error| AppError::Usage(error.to_string()))?;
    // The recorded root, where the target is an instance. Rendering against
    // another root would write a block the installer never would.
    let docs_root = args.docs_root.unwrap_or_else(|| {
        crate::services::verifier::read_manifest(target)
            .map_or_else(|_| "_docs".to_string(), |m| m.docs_root.to_string())
    });

    if !args.apply && !args.check {
        output::line(
            render_block(&RenderOptions {
                docs_root,
                entry: args.entry,
                indent: args.indent,
                declaration,
            })
            .trim_end_matches('\n'),
        );
        return Ok(());
    }

    let path = target.join(CONFIG);
    let host = std::fs::read_to_string(&path)?;
    // A malformed marker pair is refused rather than repaired: a region this
    // command cannot read is a region it must not overwrite.
    //
    // Strip the existing region before splicing, or the splice appends a
    // second one. The indentation is measured from the stripped base, which
    // is where the installer measures it.
    let (base, _) = marker::split_block(&host)?;
    let rendered = render_block(&RenderOptions {
        docs_root: docs_root.clone(),
        entry: args.entry,
        indent: marker::splice_indent(&base)?,
        declaration: declaration.clone(),
    });
    let spliced = marker::splice(&base, &rendered)?;
    let agents = agents_rewrite(target, &docs_root, &declaration)?;
    let agents_stale = agents
        .as_ref()
        .is_some_and(|(current, placed)| current != placed);

    if args.check {
        let mut count = 0;
        if spliced != host {
            output::line(format!(
                "FAIL {path} does not match the declaration; run 'sdd hooks --apply'"
            ));
            count += 1;
        }
        if agents_stale {
            output::line(format!(
                "FAIL the documentation block in {} does not match the declaration; run 'sdd hooks --apply'",
                target.join(AGENTS)
            ));
            count += 1;
        }
        if count == 0 {
            return Ok(());
        }
        return Err(AppError::Violations { count });
    }

    if spliced == host && !agents_stale {
        output::line(format!("OK {path} already matches the declaration"));
        return Ok(());
    }
    if spliced != host {
        // Write through a sibling temporary file and rename, so a failure
        // leaves the configuration byte-identical rather than half-written.
        write_atomic(&path, spliced.as_bytes())?;
        // The manifest records this region's hash, and the block-tamper
        // check reads it. A rewrite that left the record behind would
        // report the instance as tampered the moment it was made correct.
        record_block_hash(target, CONFIG, marker::block_hash(&spliced))?;
        output::line(format!("OK rewrote the managed region in {path}"));
    }
    if let Some((_, placed)) = agents.filter(|_| agents_stale) {
        let agents_path = target.join(AGENTS);
        write_atomic(&agents_path, placed.as_bytes())?;
        record_block_hash(
            target,
            AGENTS,
            marker::block_hash_with(&placed, marker::AGENTS_BEGIN, marker::AGENTS_END),
        )?;
        output::line(format!(
            "OK rewrote the documentation block in {agents_path}"
        ));
    }
    Ok(())
}
