//! The output boundary: every byte the binary prints goes through here.
//!
//! stdout carries the result — findings, summaries, rendered documents —
//! as plain text, which is what pre-commit shows a consumer, or as one JSON
//! object when a subcommand's `--json` flag selects it. stderr carries
//! exactly one JSON error envelope for operational failures. No other
//! module (main.rs aside) prints.

use std::fmt::Display;

use crate::domain::finding::Finding;
use crate::error::AppError;

/// Print one result line to stdout.
pub fn line(text: impl Display) {
    println!("{text}");
}

/// Write pre-rendered bytes — completions, man pages — to stdout.
pub fn raw(bytes: &[u8]) {
    use std::io::Write;
    let _ = std::io::stdout().write_all(bytes);
}

/// Print one result as a single pretty-printed JSON object to stdout.
///
/// # Errors
///
/// [`AppError::Other`] when the value cannot be serialized.
pub fn json(value: &impl serde::Serialize) -> Result<(), AppError> {
    let text = serde_json::to_string_pretty(value).map_err(anyhow::Error::from)?;
    println!("{text}");
    Ok(())
}

/// Print every finding, one line each, to stdout.
pub fn findings(findings: &[Finding]) {
    for finding in findings {
        println!("{finding}");
    }
}

/// Emit the one structured error envelope to stderr.
///
/// Violations carry no envelope: their findings are already on stdout and
/// the exit code is the report.
pub fn error_envelope(error: &AppError) {
    if matches!(error, AppError::Violations { .. }) {
        return;
    }
    let envelope = serde_json::json!({
        "ok": false,
        "error": {
            "kind": error.kind(),
            "message": error.to_string(),
            "exit_code": error.exit_code(),
        }
    });
    eprintln!("{envelope}");
}
