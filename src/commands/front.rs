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
use crate::landing::observe::observe;

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
    let observation = observe(target)?;
    let found = crate::landing::classify::classify(crate::landing::classify::Signals {
        invalid: observation.invalid.is_some(),
        installed: observation.installation.is_some(),
        // The destination is this binary's own release for every front.
        at_destination: observation.installation.as_ref().is_some_and(|installed| {
            installed.canon_version == crate::domain::version::CanonVersion::current()
        }),
        drifted: observation
            .installation
            .as_ref()
            .is_some_and(crate::landing::observe::Installation::drifted),
        settled: observation.corpus.settled(),
    });
    intent
        .accepts(found)
        .map_err(|refusal| AppError::Refused(refusal.to_string()))
}
