# 07 — Lifecycle

Documents change, and the changes have to leave the corpus consistent. This chapter owns how a spec
changes, how exploratory material becomes durable, and how facts that expire are kept honest.

## Changing a spec

- When current behavior changes, the author MUST update the spec in the same change.
- A change MUST state what was added, modified, or removed, and MUST NOT restate what it left alone.

Three operations, and nothing else.

| Operation | Means                                           | Leaves behind       |
| --------- | ----------------------------------------------- | ------------------- |
| Add       | a new requirement binds                         | a new rule ID       |
| Modify    | an existing requirement now says something else | the same rule ID    |
| Remove    | a requirement stops binding                     | nothing in the spec |

Modify keeps the ID when the rule is still about the same thing, so every commit and comment citing it
still resolves. It gets a new ID when the subject changed, which makes it an add and a remove.

Remove deletes the requirement outright. No strikethrough, no `deprecated` marker, no note saying the
rule used to exist. The commit holds the diff and a decision record holds the reasoning.

## When a change earns a decision record

Add a record when the change clears the threshold in [04 — Decisions](./04-decisions.md): cross-cutting,
expensive to reverse, constraining, or rejecting a plausible alternative.

A change that adds an enforceable rule and clears the threshold produces two artifacts in one commit:
the requirement in the spec, and the record explaining the choice. The record cites the rule ID.

Most changes clear neither bar and produce only a spec edit. That is the normal case.

## Adopting a rule over an existing corpus

A rule lands after the material it binds. Applying it to what predates it is ordinary work, not a
precondition for stating it.

- A project adopting a rule MUST decide in the same change whether it is gated or held by review.
- A project MUST NOT defer a rule because existing material breaches it.

Three shapes, and the choice is about cost rather than principle.

| Shape                           | When it fits                                                  |
| ------------------------------- | ------------------------------------------------------------- |
| Fix and gate at once            | few enough breaches to clear inside the adopting change       |
| Gate with a shrinking exemption | many breaches, and each one changes what a document means     |
| State it and hold it by review  | many breaches, and none of them changes what a document means |

The exemption list is the one that goes wrong. It must fail once a path it names no longer needs it,
or it stops shrinking the day it stops being read; [06 — Format](./06-format.md) owns that rule.

Clearing breaches is gate work and runs at whatever pace the project chooses, a file at a time or in
one pass. What is not available is a fourth shape where a rule is stated, ungated, and not declared
unenforced. That is the state readers learn to ignore, and [08 — Gates](./08-gates.md) exists to keep
a project out of it.

## Drafts

`.draft/` at the project root is the workshop, and it is gitignored. Discovery notes, raw outlines,
copied issue text, and half-shaped arguments live there.

- Exploratory material MUST stay outside the docs root.
- Promotion MUST be a rewrite into the owning zone.
- A promoted draft MUST be deleted.

Promotion is a rewrite because a draft contains uncertainty, repeated facts, and abandoned options,
and the durable document should contain only the result. Keeping the draft afterwards leaves a second
place for a reader to mistake for truth.

If a draft tangles exploration with something the project is already working under, split it: the
binding part goes to the plan zone under version control, and the rest stays in `.draft/` until it
resolves.

Promotion also strips project-private context when the target is project-agnostic material. Replace
local people, hosts, incidents, and workspace paths with placeholders; keep concrete public names only
where they are necessary examples.

## Perishable facts

Some documents hold facts that expire without any local change: benchmark results, vendor pricing,
model and tool rosters, external API shapes, security advisories, dependency lifecycle dates, platform
support matrices.

- A document holding a fact that depends on an external source MUST have an entry in the tracking
  registry.

The registry is machine-readable, one entry per artifact, and it records enough for a human or an
agent to know what is stale and how to check it.

```yaml
# <root>/reference/tracking.yaml
tracked:
  - path: _docs/reference/model-pricing.md
    last_checked: 2026-06-20
    cadence: 30d
    why: provider prices change without notice
    revalidate: re-fetch from the provider's official pricing page
    dependents:
      - _docs/guides/cost-estimation.md
```

The registry describes; it never becomes a second copy of the fact it tracks.

Revalidation is a scan for overdue entries, a re-fetch from the authoritative source, an update to the
artifact, and a bump to `last_checked`. Overdue is deterministic and belongs to tooling. Deciding what
the new truth is requires judgment and belongs to a person.

- An agent that cannot re-verify a claim MUST report it rather than overwrite it.

Do not track stable conceptual documents because they are old. A rule does not expire on a timer; it
is changed or removed by a decision.

## External-system bugs

A bug in a dependency, platform, or service that the project must work around is recorded once, in
reference, with the workaround and the condition that retires it.

- A temporary workaround MUST record the condition under which it is removed.

```yaml
# <root>/reference/known-issues/KI-<slug>.md
---
upstream: https://github.com/<org>/<repo>/issues/1234
affects: <component>
workaround: <what the project does instead>
retire_when: upstream release >= 2.4.0
---
```

- A known-issue record MUST be named `KI-<slug>.md`, and that name is the case id.

The case id is the filename, so a suppression that names `KI-krun-mangles-newlines` resolves to one
file by inspection. A counter would not: an external bug is found by whoever trips over it, on
whatever branch they are on, and two people allocating the next number is the same collision a
decision record avoids the same way ([04 — Decisions](./04-decisions.md)).

The retire condition is what stops a workaround outliving its bug. Without it the workaround becomes
permanent by default, and the next reader assumes it was a design choice.

- A record MUST walk its mechanism step by step, as a run a reader can follow, under a
  `## How it works` heading.

A record that only names the defect, as in "the formatter moves the marker", is unfalsifiable to
everyone but its author, and the next reader re-derives the mechanism from scratch. The shape that
avoids that: name the two or three moving parts, run one concrete case, and show what each step
leaves behind — the input, the command that acts on it, the state after it — so the wrong step is
visible rather than asserted. Close with the cause in one line, the reading a triager reaches for
first and why it is wrong, and the smallest fix. Quote captured output verbatim, tied to the step it
belongs to.

The walkthrough is what the case id buys. A suppression cites that id from source, and whoever
follows the citation arrives knowing nothing about the bug; the walkthrough is the only part of the
record written for them.

- A record filed upstream MUST carry the body it was filed with, in the tracker's own markup.

Filing is where the record leaves the repository. A record written only for the project is written
a second time in the tracker, under whatever deadline made the bug worth filing — and the two drift
from that moment on. A `## Report` section that is the filed text, title on its own line and body in
the tracker's markup, makes the filing a paste and keeps the repository holding what was actually
said upstream.

A record carries one state, and the state says what has to happen next.

| State           | Means                                                                |
| --------------- | -------------------------------------------------------------------- |
| `investigating` | reproduced, not yet root-caused                                      |
| `mitigated`     | a permanent guard exists and stays after the upstream fix            |
| `masked`        | a temporary workaround is in the tree, carrying its retire condition |
| `monitoring`    | the upstream fix is believed deployed                                |

- A record MUST carry exactly one state.
- A masked record MUST carry a retire condition, and a mitigated one MUST NOT.

The pair that earns the vocabulary is `mitigated` against `masked`, because they look alike in the
diff and retire oppositely. A readiness check that waits for a service to report healthy is part of
the design and outlives the bug that exposed the need for it. A fixed sleep that hides the same race
is a mask, and it leaves when the bug does. One case can carry both.

When the condition is met, remove the workaround and delete the record in the same change. The record
existed to describe a live constraint; a constraint that lifted leaves no trace in prose.

- Where a resolved symptom could recur and be misread, the author MUST leave a diagnostic entry.

That entry is the durable half, and it is the only half. It carries the symptom, the signal that
identifies it, and what the signal means — the shape
[11 — Operational](./11-operational.md) owns — so a reader meeting the symptom again recognizes it in
one lookup. An archive of resolved cases is not that: it is kept because the cases existed, and the
log already holds them.

## Sources

- OpenSpec, on delta descriptions stating what changes without restating the unchanged:
  <https://github.com/Fission-AI/OpenSpec/blob/main/docs/concepts.md>
