//! `payload` subcommand: runtime-shape.
//!
//! The diagnostic form of the release seam, and the first proof that the
//! seam answers for a release this binary does not carry. It reads and
//! never writes into a target: the only thing it can leave behind is a
//! verified archive in the cache.

use crate::cli::payload::PayloadArgs;
use crate::context::AppContext;
use crate::domain::paths::UserEnv;
use crate::error::AppError;
use crate::output;
use crate::release::crates_io::CratesIoResolver;
use crate::release::embedded::EmbeddedReleaseBundle;
use crate::release::{Provenance, ReleaseBundle, ReleaseManifest, ReleaseResolver, Role, Selector};

/// Report what one release carries.
///
/// # Errors
///
/// [`AppError::Usage`] for a version that is not semantic and for
/// `--offline` with no cache root, and the resolver's refusals.
pub fn run(_ctx: &AppContext, args: &PayloadArgs) -> Result<(), AppError> {
    let manifest = match selector(args.release.as_deref())? {
        Selector::Embedded => EmbeddedReleaseBundle::new().manifest()?,
        chosen => {
            let cache = UserEnv::from_process()
                .user_paths()
                .ok_or_else(|| AppError::Usage("no cache root resolves".to_string()))?
                .bundle_cache
                .path;
            let resolver = CratesIoResolver::new(&cache).offline(args.offline);
            resolver.resolve(&chosen)?.bundle.manifest()?
        }
    };
    if args.json {
        return output::json(&manifest);
    }
    render(&manifest);
    Ok(())
}

/// What the caller asked for.
fn selector(release: Option<&str>) -> Result<Selector, AppError> {
    match release {
        None | Some("embedded") => Ok(Selector::Embedded),
        Some("latest") => Ok(Selector::Latest),
        Some(version) => version.parse().map(Selector::Exact).map_err(|_| {
            AppError::Usage(format!(
                "--release takes embedded, latest, or a semantic version; {version} is none of those"
            ))
        }),
    }
}

fn render(manifest: &ReleaseManifest) {
    let provenance = match manifest.provenance {
        Provenance::Native => "native".to_string(),
        Provenance::LegacyAdapted => manifest.descriptor_sha256.as_ref().map_or_else(
            || "legacy-adapted".to_string(),
            |digest| format!("legacy-adapted from descriptor {digest}"),
        ),
    };
    output::line(format!("spec-driven-docs {}", manifest.version));
    output::line(format!("payload schema: {}", manifest.payload_schema));
    output::line(format!("provenance: {provenance}"));
    output::line(format!("payload digest: {}", manifest.payload_sha256));
    let payload = manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.role == Role::Payload)
        .count();
    let metadata = manifest.artifacts.len() - payload;
    output::line(format!("artifacts: {payload} payload, {metadata} metadata"));
}
