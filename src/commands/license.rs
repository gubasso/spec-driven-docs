//! `license` subcommand: runtime-shape.
//!
//! Prints the license statement, or a specific half's full text, so the
//! terms travel with the binary. The texts live in `embedded`.

use crate::cli::license::LicenseArgs;
use crate::context::AppContext;
use crate::error::AppError;
use crate::output;

/// Print license terms.
///
/// # Errors
///
/// None.
pub fn run(_ctx: &AppContext, args: LicenseArgs) -> Result<(), AppError> {
    if !args.method && !args.payload && !args.third_party {
        output::line(crate::embedded::LICENSE.trim_end_matches('\n'));
        return Ok(());
    }
    if args.method {
        output::line(crate::embedded::LICENSE_CC_BY.trim_end_matches('\n'));
    }
    if args.payload {
        output::line(crate::embedded::LICENSE_MIT.trim_end_matches('\n'));
    }
    if args.third_party {
        output::line(crate::embedded::THIRD_PARTY_NOTICES.trim_end_matches('\n'));
    }
    Ok(())
}
