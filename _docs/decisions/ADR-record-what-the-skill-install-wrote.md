# Record what the skill install wrote

## Context and Problem Statement

`sdd skill install` writes under the user's home, outside any instance, so no manifest records those files. Its only reference was the payload it carries, which makes a copy an older release wrote indistinguishable from a file the user edited. Every release that touched a skill therefore refused on destinations nobody had touched, breaking the canon's own `just install`, and said "locally changed bytes" in exactly the case where that was false.

## Considered Options

- `record what a successful apply wrote` — chosen.
- `pass --force in the install recipe` — rejected: it removes the protection while leaving the appearance of it.
- `compile in every released payload` — rejected: the binary would carry every historical skill forever.
- `record them in the instance manifest` — rejected: `distribution:user-scope-files-stay-unrecorded` keeps home-directory files out of the record that drives verification.

## Decision Outcome

Chosen option: `record what a successful apply wrote`. `$HOME/.local/state/spec-driven-docs/skills.json` maps each destination to the digest written there, and an apply refuses only on bytes matching neither the payload nor that record. `--force` still overrides.

The record is state, not a manifest. No verification reads it, and every unreadable shape resolves to an empty record, so a lost one costs only the benefit of the doubt. It is home-relative rather than XDG-relative because the roots it speaks for are `$HOME/.agents` and `$HOME/.claude`, which no XDG variable moves.

The same change makes an apply restore on failure. The write loop crossed two roots with no backup, so a refusal on the second left the first upgraded and two agents on different versions of one skill.

Enforced by `distribution:a-stale-skill-is-not-a-conflict` and `distribution:a-skill-install-restores-on-failure`.

## Consequences

- Good: `just install` is idempotent across releases, and the refusal is true.
- Good: a genuine edit still refuses, because the record vouches for bytes rather than paths.
- Bad: the tool keeps state outside every instance, which the design had avoided.
- Bad: a home installed before this change has no record, so its next install still asks for `--force`.

## Status

Accepted
