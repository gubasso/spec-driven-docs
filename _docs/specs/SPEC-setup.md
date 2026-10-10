# Setup Specification

<!--TOC-->

- [Purpose](#purpose)
- [Requirements](#requirements)
  - [`setup:status-reports-what-an-older-release-left` — Status reports what an older release left](#setupstatus-reports-what-an-older-release-left--status-reports-what-an-older-release-left)
  - [`setup:env-reports-the-loader-and-each-item` — Env reports the loader and each item](#setupenv-reports-the-loader-and-each-item--env-reports-the-loader-and-each-item)
  - [`setup:env-writes-one-item-through-the-recorded-loader` — Env writes one item through the recorded loader](#setupenv-writes-one-item-through-the-recorded-loader--env-writes-one-item-through-the-recorded-loader)
  - [`setup:lint-reports-each-finding-with-its-rule` — Lint reports each finding with its rule](#setuplint-reports-each-finding-with-its-rule--lint-reports-each-finding-with-its-rule)
  - [`setup:inventory-lists-every-document` — Inventory lists every document](#setupinventory-lists-every-document--inventory-lists-every-document)
  - [`setup:an-archive-moves-only-committed-documents` — An archive moves only committed documents](#setupan-archive-moves-only-committed-documents--an-archive-moves-only-committed-documents)
  - [`setup:an-archive-rebuilds-from-git` — An archive rebuilds from Git](#setupan-archive-rebuilds-from-git--an-archive-rebuilds-from-git)
  - [`setup:mark-appends-and-never-rewrites` — Mark appends and never rewrites](#setupmark-appends-and-never-rewrites--mark-appends-and-never-rewrites)
  - [`setup:close-refuses-an-unfinished-source` — Close refuses an unfinished source](#setupclose-refuses-an-unfinished-source--close-refuses-an-unfinished-source)
  - [`setup:an-open-migration-is-declared` — An open migration is declared](#setupan-open-migration-is-declared--an-open-migration-is-declared)
  - [`setup:an-incremental-landing-is-named` — An incremental landing is named](#setupan-incremental-landing-is-named--an-incremental-landing-is-named)

<!--TOC-->

## Purpose

Rules governing the verbs that serve a setup run a result instead of a procedure: the report of what an older release left on the host, the environment report and its writes, the lint report, and the migration verbs that archive a corpus and track its rebuild. Each verb computes, writes, or proves one deterministic step, so the setup skill keeps only the judgment and the questions to the operator. No instance adopts this spec: its subject is the binary, so an instance holding these rules holds obligations it cannot violate. How a target is classified is stated in `SPEC-distribution.md`. How a stage, a comparison, and a landing behave is stated in `SPEC-staging.md`. What the lint configuration must hold is stated in `SPEC-instance.md`.

## Requirements

### `setup:status-reports-what-an-older-release-left` — Status reports what an older release left

`sdd status` MUST list each directory or file a release that stored plans left under the state root or the cache root, by its exact path, its kind, and whether it still binds a run, and MUST act on none of them.

No verb of this release acts on that state, so each entry is a path the operator removes by hand, under the operator's own authority. An entry that still binds a run, an unfinished plan or the journal of an unfinished run, is finished or abandoned with the older binary first.

#### Scenario: A host upgraded from a release that stored plans

- GIVEN a state root holding the plan store an older release wrote
- WHEN `sdd status --json` runs
- THEN `legacy_state` names the store's path with the kind `plan-store` and `active` true while it holds a plan, and nothing under it changes

Verify: `cargo nextest run -E 'binary(cmd_status)'`

### `setup:env-reports-the-loader-and-each-item` — Env reports the loader and each item

`sdd env` MUST report the shell loader the repository records, `direnv` for an `.envrc`, `dotenv` for an `.env`, or `none`, and the state of the docs scratch variable and of the ignore entry for a docs scratch inside the repository, writing nothing.

The loader is read from the repository's own files, never from the host, because a loader the repository does not carry is one no other contributor runs.

#### Scenario: A repository carries an `.envrc` and no scratch variable

- GIVEN a repository with an `.envrc` and an instance recording a docs scratch inside it
- WHEN `sdd env --json` runs
- THEN it reports the loader `direnv`, the variable `unwired`, and the ignore entry `unwired` where `.gitignore` does not cover the scratch

Verify: `cargo nextest run -E 'binary(cmd_env)'`

### `setup:env-writes-one-item-through-the-recorded-loader` — Env writes one item through the recorded loader

`sdd env --apply <item>` MUST write only that item: the variable's line through the recorded loader with the loader file's ignore entry, or the docs scratch's ignore entry, and MUST refuse where no loader is recorded or the docs scratch sits outside the repository.

With direnv the per-developer line goes to `.envrc.local`, which `.envrc` sources and `.gitignore` covers. Every line already present is left as it is, so a second apply writes nothing.

#### Scenario: The operator approves both items after a landing

- GIVEN a landed instance with an `.envrc` and a docs scratch inside the repository
- WHEN `sdd env --apply docs-scratch-variable --value <path>` and `sdd env --apply docs-scratch-ignore` run
- THEN `.envrc.local` carries the export, `.envrc` sources it, `.gitignore` covers both, and a second run of each writes nothing

Verify: `cargo nextest run -E 'binary(cmd_env)'`

### `setup:lint-reports-each-finding-with-its-rule` — Lint reports each finding with its rule

`sdd lint` MUST read the linter's discovery locations for the active profile, or the candidate profile before a landing, and MUST report each finding with the rule it breaks and the kind of edit it needs, reporting `unknown` for a configuration whose grammar this binary does not read.

The findings are a configuration that replaces the delivered shapes, a linked documentation root or scan root, a root configuration that does not load or enable `relative-links`, and a reserved Markdown path the linter still reads. The report proposes no edit of its own: the operator approves each one, and `sdd lint prove` is the proof that follows.

#### Scenario: A stray configuration sits beside the specs

- GIVEN a landed instance with a `.markdownlint.yaml` beneath its specs directory
- WHEN `sdd lint --json` runs
- THEN it reports that path with `instance:the-lint-configuration-composes` and the edit `fold-into-root`, and exits 0

Verify: `cargo nextest run -E 'binary(cmd_lint)'`

### `setup:inventory-lists-every-document` — Inventory lists every document

`sdd migrate inventory` MUST list every documentation file the target holds with its kind, its SHA-256, and whether its home is recognized, and MUST list root metadata as `in-place`, writing nothing.

#### Scenario: A corpus holds decision records and a README

- GIVEN a repository with records under `docs/adr/` and a root `README.md`
- WHEN `sdd migrate inventory --json` runs
- THEN each record reads `decision` and `archived`, and the README reads `metadata` and `in-place`

Verify: `cargo nextest run -E 'binary(cmd_migrate)'`

### `setup:an-archive-moves-only-committed-documents` — An archive moves only committed documents

If a listed documentation file's bytes differ from the trunk's copy, then `sdd migrate archive` MUST refuse and write nothing, and an applied archive MUST record the trunk's commit as the base commit and leave no documentation file but root metadata in the target, holding each moved byte in the workspace under the checksum the registry records.

The trunk is `--trunk <ref>`, or the local branch the remote's `HEAD` names. A file differs when the trunk does not hold its working-tree bytes as a regular file, unfiltered: an edit not yet integrated, an ignored or untracked file, a link, or a checkout that converts line endings. A document the trunk holds and the target lacks differs too, because its deletion is an edit that integrates first. Git holds every byte at the base commit before the move, the archive holds it with its checksum, and the rebuild recreates it from Git, so a move has three copies behind it. The base commit is the trunk's, so it outlives the branch the migration ran on.

#### Scenario: One record was edited on the branch

- GIVEN a corpus whose trunk holds every document, and a branch that edited one record
- WHEN `sdd migrate archive --apply` runs on that branch
- THEN it exits 73 naming the record, and the target and the state root are unchanged

Verify: `cargo nextest run -E 'binary(cmd_migrate)'`

### `setup:an-archive-rebuilds-from-git` — An archive rebuilds from Git

`sdd migrate archive --rebuild --apply` MUST recreate every archived source from Git at the base commit, byte for byte. Where the workspace holds no registry, it MUST take as the base the first parent of the oldest commit on `HEAD`'s first-parent chain that recorded the migration in the declaration, and select the sources the way the inventory does.

After a squash merge or a merge commit, the commit on that chain that recorded the migration is the trunk's, and its parent holds the trunk's bytes as the migration merged. A rebuild with a registry recreates the archived bytes, and one without a registry recreates the trunk's bytes at the merge, which a later edit on the trunk may have changed.

#### Scenario: A second clone continues the migration

- GIVEN a clone whose state root holds no workspace for the open migration
- WHEN `sdd migrate archive --rebuild --apply` runs
- THEN the workspace holds every archived source with its recorded checksum, and the registry names the same base commit

Verify: `cargo nextest run -E 'binary(cmd_migrate)'`

### `setup:mark-appends-and-never-rewrites` — Mark appends and never rewrites

`sdd migrate mark` MUST append one dated entry to the history of each source it moves and MUST leave every earlier entry as it was.

The history is the migration log, so a rewritten entry is a log that no longer says what happened.

#### Scenario: A group is verified after its sources were mapped

- GIVEN two sources marked `mapped` into one group
- WHEN the group is marked `verified` with its destination
- THEN each source holds both entries, the first unchanged

Verify: `cargo nextest run -E 'test(mark_appends_history_and_never_rewrites_an_earlier_entry) | binary(cmd_migrate)'`

### `setup:close-refuses-an-unfinished-source` — Close refuses an unfinished source

While a source is `pending`, `mapped`, or `written`, or a verified source's group names no destination that exists as a file inside the target, `sdd migrate close` MUST refuse naming each one, and an applied close MUST leave the declaration as a greenfield landing of the same choices writes it.

#### Scenario: Three tagged records become one

- GIVEN three archived records filed into one group, and the instance a migration landed
- WHEN the group is marked `verified` with its one destination and `sdd migrate close --apply` runs
- THEN the migration closes, the status reads `current`, and the declaration equals a greenfield landing's

Verify: `cargo nextest run -E 'binary(cmd_migrate)'`

### `setup:an-open-migration-is-declared` — An open migration is declared

While a migration is open, the declaration MUST carry `migration: <id>`, whose id MUST be the UTC second the archive opened, `%Y%m%dT%H%M%SZ`, followed by `-<n>` from 2 up with no leading zero where an earlier archive took that second, and `sdd status` MUST report that id and the classification `migration` whatever the files beneath it show.

The key is in the repository, so every clone sees that the documentation is in the archive and not lost.

#### Scenario: The instance a migration landed is verified

- GIVEN an open migration whose instance is landed and current
- WHEN `sdd status --json` runs
- THEN `classification` reads `migration` and `migration` names the id, until the close removes the key

Verify: `cargo nextest run -E 'binary(cmd_migrate)'`

### `setup:an-incremental-landing-is-named` — An incremental landing is named

`sdd init --incremental` MUST land only on a target classified `migration` with no migration open and no debt file, MUST leave the corpus where it is, and MUST record each budget finding of the corpus as debt. It MUST measure that debt before it writes, so a corpus it cannot measure lands nothing. On any other target it MUST refuse with exit 73 and write nothing, citing this rule.

The full migration is the default, so the narrower scope runs only where the operator names it, and the debt it inherits is written down rather than hidden.

#### Scenario: The operator names the incremental scope

- GIVEN a committed corpus with no instance and an `AGENTS.md` over its line budget
- WHEN `sdd init --incremental` lands a stage with `--apply`
- THEN every document stays in place, the instance is landed, and the debt file records the digest

Verify: `cargo nextest run -E 'binary(cmd_migrate)'`
