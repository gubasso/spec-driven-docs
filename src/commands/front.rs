//! What each compatibility verb is willing to serve.
//!
//! A front says what the operator meant to do. The target says what it is.
//! Where the two disagree, the front refuses and names the verb that
//! serves what was actually found, because landing seeds over a settled
//! corpus starts a second convention beside the first and no flag makes
//! that safe.

use camino::Utf8Path;

use crate::error::AppError;
use crate::landing::classify::Intent;

/// Refuse where this verb does not serve what the target turned out to be.
///
/// # Errors
///
/// [`AppError::Refused`] naming the verb, the classification, and the next
/// command. An unreadable target raises rather than being classified.
pub fn serves(intent: Intent, target: &Utf8Path) -> Result<(), AppError> {
    // An absent target is the landing verb's own business: it creates one.
    if !target.exists() {
        return Ok(());
    }
    let classified = crate::services::status::classify_target(target)?;
    // An incremental landing keeps the corpus where it is, so it serves a
    // corpus nobody archived, never one an open migration already moved.
    if intent == Intent::Incremental {
        let rule = crate::domain::rule_id::RuleId::IncrementalLandingIsNamed;
        if let Some(id) = &classified.migration {
            return Err(AppError::Refused(format!(
                "the migration {id} is open, so its corpus is archived; close it before an incremental landing ({rule})"
            )));
        }
        return intent
            .accepts(classified.classification)
            .map_err(|refusal| AppError::Refused(format!("{refusal} ({rule})")));
    }
    // While a migration is open, a landing verb serves what the archive left
    // behind: the corpus is in the workspace, so the files read as the
    // greenfield target the migration lands its instance in.
    let found = if classified.migration.is_some() {
        classified.beneath
    } else {
        classified.classification
    };
    intent
        .accepts(found)
        .map_err(|refusal| AppError::Refused(refusal.to_string()))
}
