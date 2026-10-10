# Migrate from an archive

## Context and Problem Statement

A migration rewrote each source beside the old corpus and retired it afterwards, following a checklist the agent authored. Two conventions lived side by side until the last entry closed, and only the checklist knew what had moved. Each old decision record became a new one, so the history kept every superseded choice at full length.

## Considered Options

- `archive the corpus, land greenfield, rebuild from the archive` — chosen.
- `rewrite beside the corpus, then retire` — rejected: two conventions coexist for the whole migration.
- `keep a run log under the home directory` — rejected: a second record that another clone, another operator, or CI never sees.
- `carry each old decision record as its own record` — rejected: the history grows without growing truer.

## Decision Outcome

Chosen option: `archive the corpus, land greenfield, rebuild from the archive`. The archive moves every document but the root metadata into a workspace under the state root, and refuses while a document is uncommitted, so Git, the archive, and a rebuild from Git each hold every byte. A registry beside it records each source and a dated history only the binary appends to. The declaration names the open migration, so every clone sees it.

A migration keeps state because it spans many runs, which is the one exception to a run keeping none. Old decision records that share a topic merge into one record whose considered options name each of them. They were a previous convention's records, never this instance's, so the permanence rule binds only what lands.

Enforced by `setup:an-archive-moves-only-committed-documents`, `setup:mark-appends-and-never-rewrites`, `setup:close-refuses-an-unfinished-source`, and `decision-records:merged-record-is-permanent`.

## Consequences

- Good: the repository holds one convention at every moment.
- Good: a migration resumes on any clone from Git and the declaration.
- Bad: the repository holds no documentation between the archive and the close.

## Status

Accepted
