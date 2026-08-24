# 05 — Agent Context

An agent's context window is the scarcest resource in the project. This chapter decides what is loaded
before work starts, what is fetched on demand, and how large the always-loaded files may be.

## What loads

```text
always      AGENTS.md at the repo root          cross-cutting rules, and where specs live
always      AGENTS.md in the working subtree    subtree rules, when the tool loads it
on entry    <root>/specs/SPEC-<domain>.md       the domains this unit of work touches
on demand   guides, reference, explanation      when the work needs them
on ask      <root>/decisions/ADR-<slug>.md      when someone asks why
```

- An agent starting a unit of work MUST load the specs of the domains it touches.
- An agent MUST NOT load the decision log as a matter of course.

The split is what makes an unbounded decision log affordable and a spec corpus small enough to trust.

## Context pollution

Every duplicated rule, stale draft, and superseded document competes with source code, tool output,
and the user's actual request.

The failure is not that the agent runs out of room. It is that a contradicted rule set produces
arbitrary choices: when two documents disagree, a model picks one, and the one it picks is not
predictable. A single distracting passage measurably lowers retrieval accuracy, and several compound.

Agents also amplify stale documents. A human notices from surrounding context that a page is old; an
agent quotes it confidently when the path and heading look authoritative. That is why superseded
material is deleted from specs rather than annotated, and why drafts stay outside the docs root.

- A project MUST NOT state the same rule in two documents.
- When two rules conflict, the author MUST resolve the conflict rather than qualify both.

## Settled rules are applied, not re-raised

A project states a rule so the choice is made once. An agent that surfaces a stated rule as an open
question moves the cost back onto the person who already paid it.

- An agent MUST apply a rule the project states rather than raise it as a question.
- An agent MUST NOT ask whether work earns a decision record, a unit of work, or its own commit.
- An agent reporting what is open MUST report only what is open.

The second follows from the first. [04 — Decisions](./04-decisions.md) states the threshold a record
must clear, so the question is answered before it is asked.

The third is about the shape of a report. A list of open items is read to decide what to do next, so
an entry that is resolved, decided, or measured clean costs reading time and hides the live ones. A
finding that closed during the work belongs in the change that closed it.

The failure is specific to agents. A model with room to spare will re-derive a settled choice,
present the derivation as diligence, and ask for confirmation. Each round costs a reply and teaches
the reader that a stated rule is provisional, which is the same lesson an ungated rule teaches in
[08 — Gates](./08-gates.md).

Where a rule genuinely does not reach the case, ask — naming what the rule says and where it stops.
That is a different act from asking whether the rule holds.

## One entry document per unit of work

A large corpus cannot be loaded, and an agent told to read the docs will either truncate or drown. The
fix is a smaller entry, not a smaller corpus: each unit of work has exactly one document that names
the sources it needs, and the session reads that document plus the files it names.

```text
  a filter that names its sources          a filter that describes them

  session                                  session
    │                                        │
    ▼                                        ▼
  plan/stories/rate-limit.md               "read the docs"
    │                                        │
    │  Governed by:                          ├─► guides/**
    ├─► specs/SPEC-rate-limiting.md          ├─► reference/**
    ├─► specs/SPEC-auth.md                   ├─► specs/**
    └─► reference/rate-limit-tiers.md        └─► explanation/**

    3 files loaded                           truncate, or drown
```

- An entry document MUST name its sources by path, not describe them by topic.
- An entry document that changes agreed behavior MUST cite the affected rule IDs; see
  [09 — Spec to Code](./09-spec-to-code.md).
- An entry document MUST NOT name a decision record.

Where the entry document lives depends on the timescale. For work in flight it is the story or task
file. For a domain being maintained it is the spec. Both work for the same reason.

## Budget the always-loaded files

An author-instructions file is paid for on every session whether or not it is relevant, so its length
is a standing tax rather than a one-time cost.

- A root author-instructions file MUST be at or below 100 lines.
- A subtree author-instructions file MUST be at or below 150 lines.

Two habits hold the line: write each rule once and link rather than restate it, and keep the prose
lean. A file paid for every session is the worst place for a paragraph that could be a sentence.

An author-instructions file is a digest and a router. It carries the rules that bind every file, and
for everything else it names the spec that owns the rule. It is never the source of truth for a rule a
spec states.

## Modular author-instructions

When rules accumulate that only matter inside one subtree, every session pays for them while working
elsewhere. Move them.

- Subtree-local rules MUST live in an author-instructions file inside that subtree.
- Rules that apply outside the subtree MUST stay in the root file.
- The root file MUST point to the subtree file rather than import it.

An eager import pulls the detail back into every session and defeats the split.

Bridge for the tool that reads only its own filename. Claude Code reads `CLAUDE.md`, not `AGENTS.md`,
and lazy-loads a nested `CLAUDE.md` only when it touches a file in that subtree. A one-line
`<subtree>/CLAUDE.md` containing `@AGENTS.md` forwards the nested rules; an import path resolves
relative to the importing file, so the sibling is `@AGENTS.md`, not a repo-root path.

The caveat that decides what may move: not every agent lazy-loads. Some tools build the instruction
chain eagerly from the repo root down to the working directory once at launch, so a nested file is
invisible to a run started at the root. Move only rules used inside the subtree.

## References stay one level deep

- A document reachable from an entry file MUST NOT be the only route to a third document the reader
  needs.

An agent following a reference from a file that was itself referenced tends to preview rather than
read, taking the first hundred lines and proceeding on partial information. The failure is silent: the
agent does not report that it read part of a file.

Keep every source an entry document needs linked directly from that entry document. A chain of three
is a chain that gets truncated at two.

## Semantic names

Names are retrieval hints. Use names that expose purpose before the file is opened:
`SPEC-<domain>.md`, `ADR-<slug>.md`, `<task>-runbook.md`, `<topic>-diagnostics.md`.

- A filename MUST indicate what the file contains.

Avoid `notes.md`, `misc.md`, `new-plan.md`, `final-v2.md`, local jokes, temporary codenames, and
issue-only identifiers. Each forces a reader to open the file to learn whether it is relevant, and
each makes search results noisy.

Headings are retrieval hints too. Stable, direct headings let an agent skim and anchor an edit. Use
one term for one concept across the whole project: if a document is a runbook in one place and a
recovery play in another, retrieval weakens for both.

## Sources

- Anthropic, on the smallest set of high-signal tokens, and on references staying one level deep:
  <https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents>,
  <https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices>
- Chroma, on distractors lowering accuracy and degradation with input length:
  <https://www.trychroma.com/research/context-rot>
