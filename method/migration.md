# Migration

How a project moves from however it documents itself today to this method, without losing a durable fact and without a step nobody can retrace. The subject is the corpus: every place the project has written itself down, converging on one home per fact. Placement of each fact is the question of [Placement](./placement.md). Retiring a rule-breaching document is the question of [Lifecycle](./lifecycle.md). This chapter owns the procedure that carries a whole corpus across.

A migration archives the old corpus, lands a clean greenfield instance, and rebuilds the documentation from the archive, one group of sources at a time. Every step below names its `sdd migrate` verb, its gated writes, and its check.

## Classification before anything lands

A migration begins with a verdict about the target, read from evidence rather than judged by feel.

- greenfield: no durable documentation beyond root metadata, such as a README, a license, contributor notes. Land the zones and start writing. There is nothing to migrate.
- brownfield: a settled corpus under a documentation root, or a methodology marker such as a site generator's configuration, a decision-record directory, a specs tree. The corpus is migrated, never overwritten and never left as a second convention beside the new one.
- needs-decision: durable prose outside any recognized home. The operator says what it is before any plan claims to know.

An installed instance is a fourth state and not a classification: it routes to [Landing](./landing.md), which owns every landing and upgrade over an instance that already exists. No instance does not mean no documents. A brownfield target with no instance is the common case. Treating it as green is how a project ends up running two conventions at once. Once a migration is open, `sdd status` reports the state as `migration` until it closes, whatever the files beneath it show.

The plan a target gets reports the three kinds of debt separately. A structural finding is a document in a home the method does not recognize. A budget finding is a document over a size or shape budget. A style candidate is prose the writing convention would change, and the engine raises it unjudged: it never labels prose compliant or noncompliant.

## Scope

The full migration is the default: every durable fact ends in exactly one home, and the old convention is retired. The reason is the structural finding. A document in an unrecognized home is evidence that two conventions would coexist, and the archive is what retires the second one.

Narrowing the scope is the `migration-scope` decision, and it is not offered: it remains an exception the operator asks for by name. The incremental answer is `sdd init --incremental`: it lands the instance beside the corpus with no archive, records each budget finding as debt under the project's `SPEC-budget-debt.md`, and each document converts on its next edit. Taking it costs one thing, stated plainly: nothing retires.

## The procedure

```text
pre-flight  every documentation file matches the trunk
1 inventory sdd migrate inventory
2 archive   sdd migrate archive --apply              (gated)
3 land      the setup skill's land task
4 map       tags and groups, approved by the operator
5 loop      one group at a time: write, verify, mark
6 close     sdd migrate close --apply                (gated)
```

### Pre-flight

Every documentation file matches the trunk's copy byte for byte, so Git holds its bytes at the trunk commit the archive records as `base_commit`. An edit to an old document integrates onto the trunk before the archive runs. The archive refuses while any listed file differs from the trunk, and writes nothing: an edit not yet integrated, a deletion not yet integrated, an ignored or untracked file, a link, or a checkout that converts line endings. The trunk is `--trunk <ref>`, or the local branch the remote's `HEAD` names.

The repository holds no documentation between the archive and the close. The branch the work happens on, the working copy, and when the work merges are the operator's. The archive, its registry, and `base_commit` are the fallback this tool provides.

### 1. Inventory

`sdd migrate inventory` lists every documentation file with its kind, its checksum, and whether its home is recognized. Root metadata, the `README.md`, the license, the contributor guide, and the agent files, is listed `in-place`: the project still needs it during the migration, so it is registered as a source and rewritten where it sits.

Check: every document the operator knows of appears. A missing one is a document the inventory cannot read, and the plan names it before the archive.

### 2. Archive

`sdd migrate archive` previews the move, and `sdd migrate archive --apply` performs it as a gated step. It moves every listed file but the root metadata into a workspace under this tool's state root, `migrations/<repository>/<id>/sources/`, and writes `registry.yaml` beside it. It records the open migration in the declaration as `migration: <id>`, so every clone sees that a migration is open.

Moving is safe because three copies of every byte exist first. Git holds it at `base_commit`, on the trunk, so it outlives the branch. The archive holds it with its checksum. `sdd migrate archive --rebuild --apply` recreates the archive from Git at `base_commit`, on this clone or another one. Without a registry, the base is the first parent of the commit on the trunk's first-parent chain that recorded the migration. After a merge, that commit is the trunk's, and its parent holds the trunk's bytes as the migration merged: the bytes the archive matched, unless the trunk edited a document since.

Check: `sdd migrate status` lists every source as `pending`, and none reads as changed.

### 3. Land

The target now reads as greenfield beneath the open migration, so the setup skill's land task lands the instance through the landing pipeline. The declaration the archive wrote is a seed still to land: the landing writes the whole seed and carries the migration key into it.

Check: `sdd verify --target .` passes, and `sdd status --json` still reports `migration`.

### 4. Map

The agent reads the sources in groups and proposes the mapping. A source rewrites into one destination, splits into several, merges with others into one, or is dropped with its reason recorded. Related sources are read together, so the agent sees the whole picture before it writes.

Each source of a previous convention's decision records gets topic tags, and the records that share a tag form one group. The default result is one new decision record for the group, whose considered options name each old decision with its why and its how. The history stays lean, and Git and the archive keep the original bytes.

The operator approves the map. The approved map is stored through the one verb that writes the registry:

```bash
sdd migrate mark <source> mapped --group <group> --tag <topic>
sdd migrate mark <group> mapped --destination <path>
sdd migrate mark <source> dropped --reason "<why>"
```

Check: `sdd migrate status` shows every source in a group or dropped.

### 5. Loop

One group at a time:

1. Read the whole group from the archive into one context.
2. Write its destinations under the authoring rules, with a slug for every new name. No rename verb exists: the `relative-links` gate proves the links.
3. Verify the destinations: the gates that govern the zone, and a read of each destination against its sources.
4. Mark the group `written`, then `verified`. Each mark appends a dated entry to every source's history and never rewrites an earlier one, so the registry is also the migration log.

A discovery is a document the inventory missed, a split the map did not name, or a deletion nobody approved. It returns to the map. It is never handled inline, because a scope widened one source at a time was never approved at any size.

Check: `sdd migrate status` reports the group's sources `verified`, and a changed archived source reads as changed.

### 6. Close

`sdd migrate close` refuses while any source is `pending`, `mapped`, or `written`, while a verified source has no group destination, and while a destination does not exist. `sdd migrate close --apply` then removes the declaration key, and the declaration reads as a greenfield landing of the same choices wrote it.

The archive stays. Keeping or removing it is the operator's gated step, by its exact path.

Check: `sdd status --json` reports `current`, and `sdd verify --target .` and the full gate run pass.

## The registry

`registry.yaml` is the one record of the migration. Only `sdd migrate` verbs write it.

```yaml
schema_version: 1
id: 20261008T140200Z
base_commit: 4f1c2e0
sources:
  - id: docs-adr-0007-use-postgres-md
    path: docs/adr/0007-use-postgres.md
    sha256: 9f2c…
    kind: decision
    placement: archived
    tags: [storage]
    group: storage
    status: verified
    history:
      - {at: 2026-10-08T14:02:00Z, status: pending}
      - {at: 2026-10-08T15:40:00Z, status: verified}
groups:
  - id: storage
    sources: [docs-adr-0003-use-postgres-md, docs-adr-0007-use-postgres-md]
    destinations: [docs/decisions/ADR-store-state-in-postgres.md]
```

The statuses are `pending`, `mapped`, `written`, `verified`, and `dropped`. A migration keeps this state across runs because it spans many of them, which no other setup task does.

## Boundary tests

- A target with old docs and no instance is brownfield, not greenfield: classify by the corpus, never by the installation.
- A verified mark on a group whose destination fails its gate is a false completion: verification precedes the mark.
- A document that differs from the trunk stops the archive: a byte the trunk does not hold has no recovery path once the branch is gone, so nothing moves.
- A fact with no destination is not migrated: its source stays unfinished, and the close refuses to end past it.
- An archived source whose bytes changed reads as changed: the registry's checksum is what notices.

## Sources

- Diátaxis on migrating existing documentation gradually rather than by upheaval.
- Evans, Domain-Driven Design, on strangler-style convergence: the old convention retires piece by piece as the new one takes each piece over.
