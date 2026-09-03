# 12 — Migration

How a project moves from however it documents itself today to this method, without losing a durable fact and without a step nobody can retrace. The subject is the corpus: every place the project has written itself down, converging on one home per fact. Placement of each fact is [01 — Placement](./01-placement.md)'s question; retiring a rule-breaching document is [07 — Lifecycle](./07-lifecycle.md)'s; this chapter owns the procedure that carries a whole corpus across.

## Classification before anything lands

A migration begins with a verdict about the target, read from evidence rather than judged by feel.

- greenfield: no durable documentation beyond root metadata — a README, a license, contributor notes. Land the zones and start writing; there is nothing to migrate.
- brownfield: a settled corpus under a documentation root, or a methodology marker — a site generator's configuration, a decision-record directory, a specs tree. The corpus is migrated, never overwritten and never left as a second convention beside the new one.
- needs-decision: durable prose outside any recognized home. The operator says what it is before any plan claims to know.

An installed instance is a fourth state and not a classification: it routes to verification or upgrade, not to this chapter. No instance does not mean no documents — a brownfield target with no instance is the common case, and treating it as green is how a project ends up running two conventions at once.

## The inventory is the plan

The full sweep is the default scope: every durable fact ends the migration in exactly one home, and the old convention is retired. Narrowing that scope is an explicit decision, stated where the plan is approved.

The plan's body is a migration checklist: one entry per source document, written before the first destination write and kept under version control in the plan zone — a ranked plan with declared scope is binding, however early, per [01 — Placement](./01-placement.md). Each entry carries:

```markdown
- [ ] `old-docs/deploy-and-why.md`
  - checksum: 9f2c1e40
  - disposition: split
  - destinations: `specs/SPEC-release.md`, `decisions/ADR-cut-from-the-trunk.md`
  - approval: the migration plan, section 3
  - verified: pending
```

- The checksum pins the bytes the classification read; any digest command the project already uses serves.
- The disposition is one of: retire (the facts are dead or duplicated), merge (the facts fold into an existing document), split (the facts land in more than one home), rewrite (one document becomes one document in the owning zone's format).
- Destinations name every file the entry's facts land in. A disposition without destinations is not plannable, and one destination per fact is the point of the exercise.
- One source often carries facts with different fates: the binding half becomes a spec, the why-at-the-time half becomes a decision record, the walkthrough half becomes a guide. List the fates under the entry rather than averaging them into one.

What the checklist never carries is content. Provisional rewrites, extraction scratch, and half-sorted prose live in the workshop — a `migration/` directory under `.draft/` keeps them together — and the workshop is ignored by version control, so the checklist must hold everything a resumed migration needs to know. A target whose ignore file lacks the `.draft/` entry gains it in the approved plan, before anything stages there.

## The loop

Execution is batched and verified, never one long pass on trust.

1. Reload the checklist and re-checksum the entry's source. A changed source means the classification is stale: the entry goes back to planning, and nothing is written from it.
2. Rewrite, never move. Promotion into a zone is a rewrite into that zone's format and budgets; a moved file keeps its old shape and fails the gates it just came under.
3. Verify the destinations: the gates that govern the zone, and a read of each destination against the entry's listed facts.
4. Retire the source only after its destinations verify, only where the approved plan named the removal, and only while version control or the approved backup holds the bytes the checksum recorded: an untracked or locally modified source is committed, or backed up where the plan says, before its entry retires anything. A migration with no recovery path for an entry retires nothing from it.
5. Record completion last — check the box, update `verified` — so an interruption leaves an entry unfinished rather than falsely done.

A discovery — a document the inventory missed, a split the plan did not name, a deletion nobody approved — returns to planning. It is never handled inline, because a scope widened one entry at a time was never approved at any size.

Re-running a completed entry proves equivalence and reports it; it does not rewrite. That is what makes the loop safe to resume from any interruption.

## Closing

The migration ends with evidence, not with the last checked box.

1. A fresh inventory over the whole tree, compared against the checklist: nothing untracked remains, and no retired path still exists.
2. The project's own verification: the instance check and the full gate run.
3. The workshop's migration directory is emptied — anything still in it is either promoted or consciously dropped.
4. The checklist stays where it is, checked and closed. It is the record of what went where, and deleting it costs the one map a later reader has.

## Boundary tests

- A target with old docs and no instance is brownfield, not greenfield: classify by the corpus, never by the installation.
- A checked box with a failed destination gate is a false completion: verification precedes the mark, in every entry.
- A source edited mid-migration invalidates its entry: the checksum is what notices.
- A retirement of bytes neither version control nor the approved backup holds is a loss, not a migration: recoverability of the recorded bytes precedes every removal.
- A fact with no destination is not migrated: it is parked in the workshop or its entry stays open, and the close's fresh inventory refuses to end past it.

## Sources

- Diátaxis on migrating existing documentation gradually rather than by upheaval.
- Evans, Domain-Driven Design, on strangler-style convergence: the old convention retires piece by piece as the new one takes each piece over.
