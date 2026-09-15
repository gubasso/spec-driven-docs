# Render the candidate this binary carries

## Context and Problem Statement

The landing engine read releases other than the one it was compiled with. To do that it carried a registry resolver, a fetch-and-verify path, a payload schema range, a catalog of releases published before that schema, stored plans, fingerprints, and a recovery journal. Every one of those exists to answer a question the operator's own project manager already answers: which version is installed.

## Considered Options

- `render only the version the running binary carries` — chosen.
- `keep the cross-release engine` — rejected: a second package manager inside a documentation tool, whose oldest decoder is the one nobody runs and nobody can retire.
- `read another release, but only from the local cache` — rejected: the cache is still a second source of truth about what a release lands, and it goes stale in the one direction that matters.

## Decision Outcome

Chosen option: `render only the version the running binary carries`. The operator installs the version they want, and that binary projects its own embedded sources. `sdd stage` renders the candidate into a persistent directory the agent reads, so the comparison the old engine performed in memory is now evidence on disk that outlives the run. Production renders again from scratch and accepts no staged byte, so no cache can land a version nobody chose.

The agent, not the program, carries the semantic migration. A stage plus a Git diff is what it reads, and the setup skill is where that judgment lives.

Enforced by `staging:the-operator-owns-acquisition`, `staging:production-reads-no-staged-byte`, and `staging:the-manifest-is-written-last`.

## Consequences

- Good: one release is one witness of what it lands, and the tool holds no opinion about another.
- Good: the stage survives the landing, so the operator compares after the fact.
- Bad: landing an older shape means installing that older binary.
- Bad: a migration across several releases is agent work rather than a protocol.

## Status

Accepted

Supersedes `ADR-read-every-release-through-one-seam`, `ADR-declare-the-projection-in-the-bundle`, `ADR-apply-only-a-stored-plan`, and `ADR-ship-guidance-in-the-bundle-per-release-that-needs-it`, all unedited.
