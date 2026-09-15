# Report a partial skill install rather than undo it

## Context and Problem Statement

A skill install replaced every package file under the user's home as one transaction: back up each destination, journal, replace, and roll the set back on any failure. The journal was the only reason this tool kept copies of the user's files, and its recovery path is one nobody invokes directly.

## Considered Options

- `replace one file at a time and report what finished` — chosen.
- `keep the journal and the backup store` — rejected: it holds the user's bytes in a second place for a failure whose recovery is running the command again.
- `roll back in memory without a journal` — rejected: a process that dies cannot run its own rollback, which is the case the journal existed for.

## Decision Outcome

Chosen option: `replace one file at a time and report what finished`. Each destination is staged beside itself and renamed into place, so every file is whole at every moment. The receipt is written after all of them, and until it lands the previous one still describes the home. A run that stops names what it observed itself complete, and running it again finishes the rest.

The receipt vouches for every package destination holding the payload's bytes rather than only the ones one run wrote. That is what lets a rerun finish a stopped run. A scratch file a stopped run left is reused rather than refused, because a rerun that needs hand cleanup is not a rerun.

Enforced by `distribution:a-skill-install-restores-on-failure` and `distribution:a-user-scope-receipt-is-required-state`.

## Consequences

- Good: the tool keeps no copy of the user's files.
- Good: one recovery path, and it is the command itself.
- Bad: a stopped run can leave one agent reading a newer skill than another until the rerun.
- Bad: what happened is read from a report rather than from a record on disk.

## Status

Accepted

Supersedes `ADR-make-the-skill-receipt-required-state` on the failure path alone, unedited. The receipt stays required state, and it is written last.
