# Count only the project text in the digest budget

## Context and Problem Statement

`agents-digest-size` held the root `AGENTS.md` to 100 lines and counted every line, the managed blocks of sdd and release-kit included. Each tool rewrites its block at each upgrade. In one project the file held 68 lines of its own and 32 lines of blocks, so a release that added one line to a block failed a project that changed nothing. The project had no say in how a block grew.

## Considered Options

- `count the lines outside the blocks, and cap each block at 20` — chosen.
- `count every line` — rejected: the gate judges text that its owner does not write.
- `count the lines outside the blocks, and cap nothing` — rejected: a project could wrap its own text in a made-up pair and leave the budget.
- `exempt only the sdd and release-kit markers` — rejected: the gate would name projects rather than a grammar.

## Decision Outcome

Chosen option: `count the lines outside the blocks, and cap each block at 20`. A block is a whole-line `<!-- BEGIN <label> -->` and the END of the same label, outside any fence. A marker that does not pair fails on its line and hides nothing. A failure names the block lines beside the count. Each tool also tests its own rendered block against the cap, so the tool fails at its release rather than in a project.

Enforced by `docs-format:author-instructions-stay-within-budget`.

## Consequences

- Good: a release that grows its block within the cap never fails a project.
- Good: one grammar serves every tool that writes a marked block.
- Bad: several made-up blocks of 20 lines each still pass, and review catches them.
- Bad: a project with a recorded ceiling fails as stale at its first upgrade, until `sdd debt tighten --apply` lowers it.

## Status

Accepted
