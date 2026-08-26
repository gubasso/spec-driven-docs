# Gate the release merge in the recipe

## Context and Problem Statement

Merging the release pull request is the irreversible step: it tags, publishes to crates.io over OIDC, builds installers, and fast-forwards `master`. Nothing stopped that merge while its checks were still running or already red. The obvious fix, a required status check on `develop`, also gates direct pushes — GitHub rejects a push whose head carries no passing check — and a direct push to `develop` is how the release recipe opens the pull request in the first place.

## Considered Options

- `Chain gh pr checks --watch before gh pr merge in the recipe` — chosen.
- `Require the test check on develop in the develop-protection ruleset` — rejected: required status checks gate pushes as well as merges, so `git push origin develop` would stop working and every change would need its own pull request.
- `Enable repository auto-merge and let GitHub hold the merge` — rejected: GitHub offers auto-merge only while a branch requirement is unsatisfied, so it depends on the option above.

## Decision Outcome

Chosen option: `Chain gh pr checks --watch before gh pr merge` — the watch blocks until every check settles and exits non-zero on a failure, so the merge cannot fire on a red or pending pull request, and `develop` stays open to the direct push the recipe needs.

## Consequences

- Good: `develop` keeps taking direct pushes, so the recipe stays a push and a merge rather than a branch-and-pull-request cycle.
- Good: skipped checks count as settled, so the gate waits on `test` without tripping over the installer jobs a pull request never runs.
- Bad: the gate is a convention the recipe carries, not an enforcement GitHub applies; merging from the web interface still bypasses it.
- Bad: nothing pins which checks must exist, so a workflow that stops reporting is a silent hole rather than a failure.

## Status

Implemented — `_docs/guides/release.md` step 3.
