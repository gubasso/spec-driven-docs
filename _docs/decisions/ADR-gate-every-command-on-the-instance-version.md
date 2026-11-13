# Gate every command on the instance version

## Context and Problem Statement

An instance records the sdd that landed it, and the binary on `PATH` can be newer or older. A command that runs against a mismatched instance writes files that no release would write, or judges them by rules the instance never received. Nothing told the reader which command fixes the mismatch.

## Considered Options

- `one gate class per action, checked before the handler` — chosen.
- `check only in the verbs that write` — rejected: a read-only verb then judges a stale instance in silence.
- `refuse every mismatch` — rejected: `sdd verify` and `sdd gate` run as hooks on every commit, so a pin move would stop every commit until the upgrade lands.

## Decision Outcome

Chosen option: `one gate class per action, checked before the handler` — one table states which actions the gate holds and how, as cargo-dist refuses a mismatch with its recorded version and names `dist init` as the fix.

The classes are `exempt`, `pre-landing`, `upgrade`, `read-only`, and `writing`. A writing action refuses every mismatch and names `sdd init`, `sdd upgrade`, or the sdd of the project's environment. A read-only action refuses a missing instance and a newer record, and warns once on stderr where the binary is newer. The hooks then keep their exit code and judge a record of an older schema by what it records. Only `init` and `upgrade` write the landing, and the gate itself never writes.

The exact equality and the split between writing and read-only actions are this convention's own policy. No single source pairs them.

Enforced by `setup:every-command-has-a-gate-class` and `setup:a-gated-command-names-its-fix`.

## Consequences

- Good: every mismatch names the command that fixes it.
- Good: a pin move leaves the commit hooks working.
- Bad: every new action needs a row, and a test walks the command tree to hold that.
- Bad: no escape hatch exists for an operator who wants to run a writing action on a mismatched instance.

## Status

Accepted
