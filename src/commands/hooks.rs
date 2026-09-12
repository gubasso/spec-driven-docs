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

use crate::adapters::fs::write_within;
use crate::cli::hooks::HooksArgs;
use crate::context::AppContext;
use crate::domain::instance_config::InstanceConfig;
use crate::domain::manifest::MANIFEST_PATH;
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
///
/// A target with no manifest is not an instance, and nothing records the
/// region there. A manifest that exists must be readable and must carry
/// exactly one record for the region, or the rewrite cannot be brought into
/// agreement with its record and the caller puts the region back.
fn record_block_hash(target: &Utf8Path, path: &str, hash: Option<Sha256>) -> Result<(), AppError> {
    let manifest_relative = Utf8Path::new(MANIFEST_PATH);
    let text = match std::fs::read_to_string(target.join(manifest_relative)) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let hash = hash.ok_or_else(|| {
        AppError::ManifestInvalid(format!("the rewritten {path} carries no managed block"))
    })?;
    let mut document: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;
    let Some(blocks) = document
        .get_mut("integration_blocks")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return Err(AppError::ManifestInvalid(
            "integration_blocks is not an array".to_string(),
        ));
    };
    let mut matched = 0usize;
    for block in blocks.iter_mut() {
        if block.get("path").and_then(serde_json::Value::as_str) == Some(path) {
            block["marker_hash"] = serde_json::Value::String(hash.to_string());
            matched += 1;
        }
    }
    if matched != 1 {
        return Err(AppError::ManifestInvalid(format!(
            "the manifest records {matched} integration blocks for {path}; expected one"
        )));
    }
    let rendered = serde_json::to_string_pretty(&document)
        .map_err(|error| AppError::ManifestInvalid(error.to_string()))?;
    write_within(
        target,
        manifest_relative,
        format!("{rendered}\n").as_bytes(),
    )?;
    Ok(())
}

/// Put one region back after its record could not be written, and say so
/// where even that fails.
fn restored(target: &Utf8Path, relative: &str, previous: &[u8], cause: &AppError) -> AppError {
    match write_within(target, Utf8Path::new(relative), previous) {
        Ok(()) => AppError::Refused(format!(
            "{relative} was rewritten and its record could not be updated, so it was put back: {cause}"
        )),
        Err(error) => AppError::Refused(format!(
            "{relative} was rewritten, its record could not be updated ({cause}), and restoring it failed ({error}); verify {relative} by hand"
        )),
    }
}

/// What the root `AGENTS.md` holds against what the declaration renders.
#[derive(Debug)]
enum Agents {
    /// The block is present and agrees with the declaration.
    Current,
    /// The block is present and disagrees; the host as it would be written.
    Stale(String),
    /// The install recorded a block and the host no longer carries one, or
    /// the host is gone.
    Missing(&'static str),
    /// The install recorded no block here, so none is owed: this
    /// repository's own root digest is release-kit-owned and carries none.
    Unmanaged,
}

/// Whether the manifest records a documentation block in `AGENTS.md`.
fn agents_block_recorded(target: &Utf8Path) -> bool {
    crate::services::verifier::read_manifest(target).is_ok_and(|manifest| {
        manifest
            .integration_blocks
            .iter()
            .any(|block| block.path.as_str() == AGENTS)
    })
}

/// Read the documentation block's state.
///
/// A host that cannot be read for a reason other than absence is an error,
/// never a silent "current".
fn agents_state(
    target: &Utf8Path,
    docs_root: &str,
    declaration: &InstanceConfig,
) -> Result<Agents, AppError> {
    let path = target.join(AGENTS);
    if path.is_symlink() {
        return Err(AppError::Refused(
            "AGENTS.md is a symlink; refusing to write the documentation block through it"
                .to_string(),
        ));
    }
    // The record is what says whether a block is owed here. A block the
    // install never recorded is the project's own text, whatever it looks
    // like, and this verb has no record to bring it into agreement with.
    if !agents_block_recorded(target) {
        return Ok(Agents::Unmanaged);
    }
    let host = match std::fs::read_to_string(&path) {
        Ok(host) => host,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Agents::Missing("the file is absent"));
        }
        Err(error) => return Err(error.into()),
    };
    if marker::block_region_with(&host, marker::AGENTS_BEGIN, marker::AGENTS_END).is_none() {
        return Ok(Agents::Missing("its managed block is gone"));
    }
    let block = crate::services::agents_render::render_block(docs_root, &declaration.writing_style);
    let placed = marker::place_agents_block(&host, &block)?;
    Ok(if placed == host {
        Agents::Current
    } else {
        Agents::Stale(placed)
    })
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
    let agents = agents_state(target, &docs_root, &declaration)?;
    let agents_path = target.join(AGENTS);

    if args.check {
        let mut count = 0;
        if spliced != host {
            output::line(format!(
                "FAIL {path} does not match the declaration; run 'sdd hooks --apply'"
            ));
            count += 1;
        }
        match &agents {
            Agents::Stale(..) => {
                output::line(format!(
                    "FAIL the documentation block in {agents_path} does not match the declaration; run 'sdd hooks --apply'"
                ));
                count += 1;
            }
            Agents::Missing(why) => {
                output::line(format!(
                    "FAIL the install recorded a documentation block in {agents_path} and {why}; run 'sdd init --apply' to restore it"
                ));
                count += 1;
            }
            Agents::Current | Agents::Unmanaged => {}
        }
        if count == 0 {
            return Ok(());
        }
        return Err(AppError::Violations { count });
    }

    // A recorded block that is gone is not this verb's to rewrite: the
    // install owns placing it, and a rewrite here would recreate the block
    // without knowing what else the operator removed.
    if let Agents::Missing(why) = &agents {
        return Err(AppError::Refused(format!(
            "the install recorded a documentation block in {agents_path} and {why}; run 'sdd init --apply' to restore it"
        )));
    }
    let agents_placed = match agents {
        Agents::Stale(placed) => Some(placed),
        Agents::Current | Agents::Unmanaged | Agents::Missing(_) => None,
    };
    if spliced == host && agents_placed.is_none() {
        output::line(format!("OK {path} already matches the declaration"));
        return Ok(());
    }
    // Every write is bounded to the target and atomic, and the manifest
    // record moves with each region: the block-tamper check reads it, so
    // a rewrite that left the record behind would report the instance as
    // tampered the moment it was made correct. A record that cannot be
    // written puts the region back.
    if spliced != host {
        write_within(target, Utf8Path::new(CONFIG), spliced.as_bytes())?;
        if let Err(error) = record_block_hash(target, CONFIG, marker::block_hash(&spliced)) {
            return Err(restored(target, CONFIG, host.as_bytes(), &error));
        }
        output::line(format!("OK rewrote the managed region in {path}"));
    }
    if let Some(placed) = agents_placed {
        let previous = std::fs::read(&agents_path)?;
        write_within(target, Utf8Path::new(AGENTS), placed.as_bytes())?;
        if let Err(error) = record_block_hash(
            target,
            AGENTS,
            marker::block_hash_with(&placed, marker::AGENTS_BEGIN, marker::AGENTS_END),
        ) {
            return Err(restored(target, AGENTS, &previous, &error));
        }
        output::line(format!(
            "OK rewrote the documentation block in {agents_path}"
        ));
    }
    Ok(())
}
