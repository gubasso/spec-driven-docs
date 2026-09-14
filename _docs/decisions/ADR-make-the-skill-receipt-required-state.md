# Make the skill receipt required state

## Context and Problem Statement

The user-scope record was best-effort. A failed write to it was a note on stdout, so an apply could land every file and vouch for none. The next run then reads an empty record, calls its own bytes the user's, and refuses. That run held no lock, wrote in place rather than through a staged rename, and kept its only backup in memory. Two installs could interleave, and a killed process could leave two agent roots on two releases.

## Considered Options

- `make the receipt required, under a lock and a journal` — chosen.
- `keep the note and warn louder` — rejected: the failure surfaces one release later, to an operator told their files are in the way.
- `derive ownership from the payload alone` — rejected: the state the record was introduced to leave, where every release refuses on files nobody edited.
- `write provenance into the installed file` — rejected: a file that no longer matches the payload still needs a second record to tell it from an edit.

## Decision Outcome

Chosen option: `make the receipt required, under a lock and a journal`. An apply holds the user-scope lock for its whole run, recovers an unfinished run first, stages every write beside its destination, journals before the first replacement, and writes the receipt last. One that cannot write the receipt fails and rolls back. A receipt vouching for nothing is removed.

This amends [ADR-record-what-the-skill-install-wrote](./ADR-record-what-the-skill-install-wrote.md), whose record was state the install could lose. Reading stays forgiving: an absent, unreadable, or unknown-schema receipt reads as empty, and the caller loses only the benefit of the doubt.

Enforced by `distribution:a-user-scope-receipt-is-required-state` and `distribution:a-skill-install-restores-on-failure`.

## Consequences

- Good: what landed and what the tool vouches for can no longer disagree.
- Good: the same primitives serve the repository apply, so that recovery is a reuse.
- Bad: a home whose state root refuses writes can no longer install.
- Bad: recovery after power loss rests on the platform's sync semantics, and is not tested.

## Status

Accepted
