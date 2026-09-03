//! `sdd doctor`: is this host ready?
//!
//! Runs the whole probe catalog from `crate::probes` and reports by class.
//! A probe failure is a result, not an error: the exit code stays 0 and the
//! report is what a caller reads.

use serde::Serialize;

use crate::cli::doctor::DoctorArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;
use crate::probes::{self, ProbeClass, ProbeResult, ProbeStatus};

/// The machine form of the doctor report.
#[derive(Debug, Serialize)]
struct Report {
    /// The shape version of this document.
    schema: &'static str,
    /// Every probe's answer, in catalog order.
    probes: Vec<ProbeResult>,
    /// What plausibly follows.
    next: Vec<String>,
}

/// Run the catalog and report it.
///
/// # Errors
///
/// [`AppError::Other`] only when the report cannot serialize; probe
/// failures are results, not errors, and the exit code stays 0.
pub fn run(_ctx: &AppContext, args: &DoctorArgs) -> Result<(), AppError> {
    let probes = probes::run_all();
    let next = vec!["sdd status --target . reports the instance".to_owned()];
    if args.json {
        return output::json(&Report {
            schema: "sdd.doctor/1",
            probes,
            next,
        });
    }
    for class in [ProbeClass::Hard, ProbeClass::Soft] {
        output::line(match class {
            ProbeClass::Hard => "hard",
            ProbeClass::Soft => "soft",
        });
        for probe in probes.iter().filter(|probe| probe.class == class) {
            let status = match probe.status {
                ProbeStatus::Ok => "ok    ",
                ProbeStatus::Failed => "failed",
            };
            output::line(format!("  {status}  {}: {}", probe.id, probe.message));
            if let Some(remediation) = &probe.remediation {
                output::line(format!("          next: {remediation}"));
            }
        }
    }
    output::line("Next:");
    for line in &next {
        output::line(format!("  {line}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use crate::probes::{ProbeClass, ProbeResult, ProbeStatus};

    /// The complete `sdd.doctor/1` shape, held by snapshot.
    #[test]
    fn the_doctor_report_schema_snapshot_holds() {
        let report = super::Report {
            schema: "sdd.doctor/1",
            probes: vec![ProbeResult {
                id: "state-root",
                class: ProbeClass::Hard,
                status: ProbeStatus::Ok,
                message: "the state root is writable".into(),
                remediation: None,
            }],
            next: vec!["sdd status --target . reports the instance".into()],
        };
        assert_eq!(
            serde_json::to_string(&report).expect("a report serializes"),
            r#"{"schema":"sdd.doctor/1","probes":[{"id":"state-root","class":"hard","status":"ok","message":"the state root is writable"}],"next":["sdd status --target . reports the instance"]}"#
        );
    }

    /// The `sdd.doctor/1` probe shape, held by snapshot.
    #[test]
    fn the_probe_schema_snapshot_holds() {
        let ok = ProbeResult {
            id: "git",
            class: ProbeClass::Soft,
            status: ProbeStatus::Ok,
            message: "git runs".into(),
            remediation: None,
        };
        assert_eq!(
            serde_json::to_string(&ok).expect("a probe serializes"),
            r#"{"id":"git","class":"soft","status":"ok","message":"git runs"}"#
        );
        let failed = ProbeResult {
            id: "pre-commit",
            class: ProbeClass::Soft,
            status: ProbeStatus::Failed,
            message: "pre-commit is not on PATH".into(),
            remediation: Some("install pre-commit; the delivered gates run through it".into()),
        };
        assert_eq!(
            serde_json::to_string(&failed).expect("a probe serializes"),
            r#"{"id":"pre-commit","class":"soft","status":"failed","message":"pre-commit is not on PATH","remediation":"install pre-commit; the delivered gates run through it"}"#
        );
    }
}
