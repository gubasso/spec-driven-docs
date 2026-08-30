# 01 — Placement

Every durable fact has one home, decided by the reader need it serves. This chapter names the homes, gives the decision procedure that reaches exactly one of them, and places the docs root itself.

## The docs root

The root's name depends on what the project's product is.

```text
code project                  knowledge base
────────────                  ──────────────
src/          the product     articles/      the product
docs/         about it        languages/     the product
                              _docs/         about it
```

A code project's product is its source tree, so `docs/` at the root is unambiguous. A knowledge base's product is the content tree, so a bare `docs/` would read as another content directory; the underscore marks the root as metadata about the library rather than part of it.

- A project MUST use `docs/` when its product is a codebase and `_docs/` when its product is a content tree.
- Product content MUST NOT live under the docs root.

## The zones

| Zone           | Path                         | Reader question                  | Lifecycle  |
| -------------- | ---------------------------- | -------------------------------- | ---------- |
| Specifications | `<root>/specs/`              | what is true now                 | living     |
| Decisions      | `<root>/decisions/`          | why, at the time                 | frozen     |
| Guides         | `<root>/guides/<topic>/`     | how do I finish this task        | living     |
| Reference      | `<root>/reference/<topic>/`  | what is the exact value          | living     |
| Explanation    | `<root>/explanation/<topic>` | how do I understand this         | living     |
| Plan           | declared, under `<root>`     | what is next, and what bounds it | perishable |

Create a zone when it has real content. An empty zone is a promise the project has not kept.

The plan zone's exact path is declared by the planning tool that owns the record, anywhere under the docs root. This framework fixes the zone's existence and reader question, never its path.

Specifications and decisions are the two zones this shelf owns; [02 — Specs](./02-specs.md) and [04 — Decisions](./04-decisions.md) hold their rules. The other four are ordinary reader-need zones and need no chapter of their own.

## Placement procedure

Ask in order and stop at the first yes.

```text
a durable fact needs a home
│
├─ does it bind every file in the project?
│    yes → the author-instructions file (AGENTS.md or equivalent)
│
├─ is it a rule an agent or reviewer must apply, and can it be verified?
│    yes → <root>/specs/SPEC-<domain>.md
│
├─ is it why one option was chosen over others, at the time?
│    yes → <root>/decisions/ADR-<slug>.md
│
├─ is it what the project builds next, and what bounds it?
│    yes → the plan zone
│
├─ is it a sequence a reader follows to finish a task?
│    yes → <root>/guides/<topic>/
│
├─ is it a value, field, symptom, or past case a reader looks up?
│    yes → <root>/reference/<topic>/
│
├─ is it background a reader needs to understand the area?
│    yes → <root>/explanation/<topic>.md
│
├─ is it why this local code looks surprising?
│    yes → a load-bearing comment beside the code
│
└─ can a name, type, or test carry it instead?
     yes → the code. prose is the wrong home.
```

The second branch is the one that changes how a project behaves. A rule that an agent must apply belongs in a spec even when it feels like background, because background is not loaded and a spec is.

## Specs are centralized

Specs live in one tree under the docs root, keyed by domain, never beside the files they govern.

- A project MUST place every spec at `<root>/specs/SPEC-<domain>.md`.
- A spec MUST NOT be co-located with the code or content it governs.

Two reasons, and the second is the one that decides it.

A path-keyed spec rots on every reorganization. Moving a spec along with a directory refactor is exactly the chore that gets dropped, and the survivor is a spec governing a path that no longer exists. A domain key survives the refactor because it names what the rules are about, not where the files sit.

Scoping by location also fails at the load step. An agent editing one directory has no way to know which other directories hold rules that bind it. Centralization plus a small always-loaded author-instructions file is what makes the whole rule set reachable from one place; see [05 — Agent Context](./05-agent-context.md).

## Domains

A domain is a capability, not a directory. In a codebase it names what the system does: `auth`, `payments`, `ingest`. In a knowledge base it names a shelf.

- A domain name MUST be lowercase and kebab-case.
- A domain MUST have a spec only when it has rules that can be verified.

Do not create a spec for a domain that has no enforceable rules yet. An empty spec is loaded on every session and teaches nothing.

## Artifact names

A file whose kind this framework fixes carries that kind as an uppercase prefix, so a directory listing, a glob, or an agent tells the kinds apart without opening anything.

| Artifact         | Name                 | Lives in                         |
| ---------------- | -------------------- | -------------------------------- |
| Specification    | `SPEC-<domain>.md`   | `<root>/specs/`                  |
| Decision record  | `ADR-<slug>.md`      | `<root>/decisions/`              |
| Known-issue case | `KI-<slug>.md`       | `<root>/reference/known-issues/` |
| Template         | `TEMPLATE-<kind>.md` | beside what it seeds             |
| Spec companion   | `SPEC-<domain>/`     | `<root>/specs/`                  |

- A file whose kind this framework fixes MUST carry that kind as an uppercase prefix.
- A prefixed filename MUST carry the slug that identifies it, and no counter.

The prefix is uppercase because the discriminator should win over the slug when a reader scans a column of filenames. It is the first thing read and the only part that repeats.

The rule that generates the table: prefix a file when its shape is fixed and gated. A project that gates a fifth shape adds a fifth prefix by the same test.

A known-issue case is prefixed because it is the one kind cited from outside the docs root. A suppression in the source names its case, and the prefix is what makes that citation resolvable by a grep over a single directory; [07 — Lifecycle](./07-lifecycle.md) owns the record's contents.

Guides, reference pages, and explanation pages take no prefix. Their zone directory already names their reader need, and a prefix on every file would make the discriminator meaningless by making it universal.

A prefix is not part of an identifier. A spec's domain is `auth`, not `SPEC-auth`, so a rule ID reads `auth:token-expiry-is-bounded`; see [03 — Rules](./03-rules.md).

## Operational material is zone-first

Runbooks, setup procedures, diagnostics, and incident write-ups are ordinary documents and take the zone matching their reader need: a runbook is a guide, a diagnostic table is reference, a post-mortem is reference. Do not create an `ops/` directory beside the zones; it splits the same reader need across two homes.

## Drafts

`.draft/` at the project root is the workshop, and it is gitignored.

[07 — Lifecycle](./07-lifecycle.md) owns what may live there and how it leaves.

A reader must be able to tell whether a document is project state from its path alone, without opening it. That is why a filename warning like `draft-final.md` inside the docs root does not substitute for the path.

Forward-looking is not the same as provisional. A ranked plan with declared scope is binding and belongs in the plan zone under version control, however early it is. The test is not whether the document is finished but whether the project is working under it. [07 — Lifecycle](./07-lifecycle.md) owns the promotion procedure.

## A project's documents stand on their own

- A document MUST NOT carry an absolute path into a person's home directory; write `~/`, `$HOME/`, or a bracketed placeholder instead.
- A document MUST state the rules the project's own domain owns rather than sending the reader to another project's documentation for them.

Two different leaks, one test: a reader who is not the author, on a machine that is not the author's, must be able to act on the document.

The first leak is the author's terminal. A command pasted from a working session carries a home directory that resolves for one person, misleads everyone else, and names someone who never agreed to be named in a file that ships. The placeholder form costs nothing and says more, because it shows which part the reader supplies.

The second leak is harder to see, because the pointer usually works. Where the project requires a convention that another project happens to document well, linking there instead of stating it hands that project a rule this one is bound by: it changes when they edit it, and a reader without access to it cannot learn what binds them. State the rule; then cite the source beside it.

Citing outward is the opposite of leaking, and this chapter does it below. A link that supports a claim — a specification, an upstream manual, a paper — is evidence a reader may follow or ignore. A link that carries the claim is a dependency. The difference is whether deleting the link removes something the reader needed from this project.

Two exemptions, both by purpose. A file whose whole job is one person's environment — `.env`, `.envrc.local`, and the sample copies that show where those values go — is where a real path belongs. And a file the project does not track was never published in the first place.

## Boundary tests

- A document that answers two reader questions is two documents.
- A rule with no verification is either not a rule or not yet finished; see [03 — Rules](./03-rules.md).
- A document that would need editing every time a file is added was indexing the filesystem.

## Sources

- Diataxis, on organizing by reader need: <https://diataxis.fr/>
- OpenSpec, on the centralized `specs/` tree keyed by domain: <https://github.com/Fission-AI/OpenSpec/blob/main/docs/concepts.md>
