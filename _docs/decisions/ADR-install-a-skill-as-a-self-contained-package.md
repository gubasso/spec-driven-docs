# Install a skill as a self-contained package

## Context and Problem Statement

The install unit was one file. Each skill landed as `SKILL.md` alone, and the two gates every skill reads landed once under the state root, at an absolute path each skill spelled out. The Agent Skills specification and the Claude Code and Codex documentation all treat the skill directory as the unit and resolve a supporting file relative to it. So every skill asked its host for a hop outside its own directory that the format never promised. A sandboxed reader may prompt on that hop, and one with no home access cannot take it.

## Considered Options

- `materialize one authored source into every package` — chosen.
- `keep the outside home` — rejected: the superseded record already named that hop as its bad consequence.
- `author a copy inside each skill directory` — rejected: what the superseded record refused, and correctly. One gate becomes one file per skill to correct.
- `restate each gate inline in every skill body` — rejected: the same divergence hidden in prose, at repeated cost to each body's line budget.

## Decision Outcome

Chosen option: `materialize one authored source into every package`. `skill-shared/` stays one authored root. The installer composes each package as `SKILL.md` plus `references/<file>` for every file under that root, and writes the whole package into every selected agent root.

This supersedes [ADR-share-skill-artifacts-outside-the-skill-directory](./ADR-share-skill-artifacts-outside-the-skill-directory.md). That record weighed an outside home against copies an author maintains. It never weighed copies an installer writes from one source, which keeps the single authored file and removes the hop.

Enforced by `distribution:a-skill-package-is-self-contained`.

## Consequences

- Good: a skill resolves its gates the way its own format says a skill resolves a supporting file.
- Good: the authored source stays one file, so a gate fix still lands once.
- Bad: the same bytes sit under every agent root, so a home holds several copies of one gate.
- Bad: a home the retired release installed keeps two files under the old shared root until the first install sweeps them.

## Status

Accepted
