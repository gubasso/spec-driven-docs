# Share skill artifacts outside the skill directory

## Context and Problem Statement

Every skill now opens by routing the agent through one plan gate: plan, validate against what knows, then execute. Its text is identical for every skill, and a divergent copy is worse than none — two skills would bind different gates under one name. The portable Agent Skills frontmatter has no include mechanism, and the two agent roots, `~/.agents/skills` and `~/.claude/skills`, make no relative path reach one file from both. The shared text needs one installed home every skill can name.

## Considered Options

- `install shared artifacts once, outside the skill roots, named by absolute path` — chosen.
- `bundle a copy inside each skill directory` — rejected: one gate becomes a copy per skill per agent root, and a gate fix is that many edits, each a chance to diverge.
- `restate the gate inline in every skill body` — rejected: the same divergence hidden in prose, plus repeated spend against each body's 150-line budget.
- `an XDG-resolved location` — rejected: the skills naming the path live under `$HOME/.agents` and `$HOME/.claude`, which no XDG variable moves, so an XDG override would separate the file from its readers.

## Decision Outcome

Chosen option: `install shared artifacts once, outside the skill roots, named by absolute path`. The `skill-shared/` payload root lands at `~/.local/state/spec-driven-docs/skills/shared/`, beside the user-scope record, under the same install, conflict, sweep, and record machinery as every skill file. Every install writes it whichever agent was selected; an uninstall retains it while any agent root holds an installed skill. The skills are local-only by their own `compatibility` line — each requires `sdd` on PATH — so the outside home costs no portability the skills ever had.

Enforced by `distribution:shared-skill-artifacts-have-one-home` and `distribution:a-skill-plans-before-it-acts`.

## Consequences

- Good: one gate file serves every skill and both agent families; a fix lands once.
- Good: the installer's existing transactional machinery covers the new destination unchanged.
- Bad: an agent must follow one absolute path outside its skill directory, a hop a sandboxed reader may prompt on.

## Status

Accepted
