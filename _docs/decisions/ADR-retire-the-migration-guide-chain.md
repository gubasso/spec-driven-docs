# Retire the migration guide chain

## Context and Problem Statement

Every release carried a `migrations/<previous>-to-<version>.md` guide, and `sdd upgrade` walked
those filenames as a version chain, refusing any instance whose installed version had no guide
out of it. The chain forced an authored prose file per release — even a fix release with nothing
to migrate — and the first missing link stranded instances: the last shell release had no guide
out of it, so no published instance could reach the binary era at all. Comparable tools carry no
such ceremony: spec-kit keeps one living upgrade page, OpenSpec re-runs its template update, and
copier — the model this upgrader follows — runs versioned migrations only where one is declared
and lets every other version pass through.

## Considered Options

- `Retire the chain; the upgrade is mechanical and the changelog is the record` — chosen.
- `Keep the chain, require a guide only when managed files or rule IDs change` — rejected: the
  gap-detection logic and the per-release judgment remain, for a briefing the upgrade report
  already prints.
- `Keep the chain as authored` — rejected: a mandatory file whose sections read `None` is
  ceremony, and one missed link blocks every older instance.

## Decision Outcome

Chosen option: `Retire the chain`. `sdd upgrade` takes any older instance straight to the
binary's version: refuse atomically on locally edited managed files, reinstall from the embedded
payload, prune what is no longer owned, and report the rule-ID diff. What changed between
versions is `CHANGELOG.md`'s business, maintained by release-plz from Conventional Commits. The
`migrations/` directory, the `sdd migration` reader, `--show-guides`, and the rule
`release:a-release-carries-its-migration-guide` with its canon test are removed.

## Consequences

- Good: a release needs no authored artifact beyond the commits that made it.
- Good: no instance can be stranded by a missing or misnamed link.
- Bad: no per-step briefing remains; an operator skipping many versions reads one aggregated
  changelog rather than a curated path.

## Status

Accepted
