# Check for a newer release and let the operator move the pin

## Context and Problem Statement

The shell-entry line `sdd self-depend sync --apply || true` moved a consumer's pin once a day, so a project took every release: a review, a gate, a landing, and often a release of its own. The operator ruled that a project takes a new release only when it needs one. The question was what the shell-entry line does instead.

## Considered Options

- `warn from a weekly cache and leave the move to the operator` — chosen.
- `keep the daily move behind an opt-out` — rejected: every consumer pays the move until someone notices.
- `remove the line` — rejected: a pin then falls behind with no signal.
- `read the network on every directory entry` — rejected: a slow network stalls every shell.
- `refuse only an .envrc evaluation, told apart by DIRENV_FILE` — rejected: direnv 2.38.2 sets it under `direnv exec` but not in an evaluation, which direnv does not document, so a later direnv could let an old line move pins silently.

## Decision Outcome

Chosen option: `warn from a weekly cache and leave the move to the operator` — the line becomes `sdd self-depend check || true`. It reads a cache, refreshes it at most once every seven days or one hour after a failure, under a lock, and warns on stderr. It writes no file but its cache. `sdd self-depend sync --apply` stays the one act that moves the pin. Where `DIRENV_IN_ENVRC` is set, it moves nothing and exits 1, and an old line still loads the shell through its `|| true`. `sdd self-depend clean --apply` swaps the old line. The release-workflow tool makes the same change.

Enforced by `acquisition:the-check-refreshes-weekly-and-retries-hourly`, `acquisition:the-check-warns-and-writes-only-its-cache`, `acquisition:the-off-switch-stops-the-check-alone`, `acquisition:sync-is-the-operators-act`, and `acquisition:clean-swaps-the-retired-line`.

## Consequences

- Good: a pin moves only on a person's act, and the shell still names a newer release.
- Good: a failed check warns with its reason.
- Bad: each landed `.envrc` needs one swap, and `direnv exec` cannot run the sync.

## Status

Accepted
