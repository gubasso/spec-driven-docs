# Retire an unedited managed file and drop an edited one

## Context and Problem Statement

A release can stop landing a managed file. The upgrade deleted such a file only under `.spec-driven-docs/` and the two skill roots, and left one elsewhere with no record that owned it. An edited one was a conflict until `--replace` named it, and was then saved and deleted. The portable rule of the agent-integration library deletes it where its bytes match the receipt, and otherwise keeps it as the project's.

## Considered Options

- `retire the unedited file anywhere, drop the edited one` — chosen.
- `keep the three roots and the conflict` — rejected: a file outside the roots stays with nobody to own or report it, and an edit stops an upgrade that loses nothing by going on.
- `delete every file the release stops landing` — rejected: an edit is the project's work, and a version moving is not permission to delete it.

## Decision Outcome

Chosen option: `retire the unedited file anywhere, drop the edited one`. The recorded digest authorizes a deletion, wherever the file lies beneath the target. A file with other bytes, an unreadable one, and one reached through a link stay on disk and leave the record. The project owns each from that landing. No departing file is a conflict, so `--replace` names only a file the release still lands.

The preview, the stage, and the comparison read the sort that the landing runs, so each names `retired` and `dropped` before the apply. Only the landing knows that a dropped `SKILL.md` still loads, so it says so then.

Enforced by `staging:a-landing-retires-or-drops-what-it-stops-landing` and `staging:compare-reports-every-destination-the-stage-carries`.

## Consequences

- Good: one rule for a departing file in sdd and in release-kit.
- Good: an edit never stops an upgrade that would not overwrite it.
- Bad: an upgrade now deletes an unedited file outside the three roots.
- Bad: after the landing, no check names a dropped file, because the record forgets it.

## Status

Accepted
