# Record the plan zone at install

## Context and Problem Statement

The typed-clause rule needs to know where a project's entry documents are. That location differs per project, so the seeded spec carried the path in one verification command. An adopter had two answers: retarget the command, which edits a seed and leaves a permanent drift line, or delete a rule they can honor. Neither works where the records live outside the checkout.

## Considered Options

- `record the location in the instance manifest at install` — chosen.
- `discover the clauses by scanning the repository` — rejected: the specs carry `ADDED`, `MODIFIED`, and `REMOVED` as prose, so a scan would report the rule's own statement as a breach.
- `pass the zone as hook arguments` — rejected: the hook wiring sits inside the marker-hashed managed block, so an edit there raises `CONFLICT` on the next upgrade.
- `template the value into the seeded spec` — rejected: every seed is byte-exact, and a substitution mechanism for one value costs more than it buys.

## Decision Outcome

Chosen option: `record the location in the instance manifest at install` — the manifest is the one project-writable store an upgrade already preserves, and `SDD_PLAN_ZONE` overrides it.

The value carries a kind rather than a bare path, because the three absent cases are not one case. A `tracked` zone is in every clone, so the gate checks it and an absent directory is drift. An `untracked` zone and an `env` zone are absent on a fresh clone, so the gate reports nothing and a reviewer holds the rule.

Enforced by `spec-to-code:a-spec-change-is-typed`.

## Consequences

- Good: no adopter edits a seeded spec, so the seed stays byte-exact and `sdd verify` stays quiet.
- Good: the kind makes the skipped case explicit rather than silent, which the gates chapter then declares unenforced.
- Bad: the manifest schema moved to version 3, so an older binary refuses a new instance until it is upgraded.
- Bad: a project on `env` gets no check where the variable is unset.

## Status

Accepted
