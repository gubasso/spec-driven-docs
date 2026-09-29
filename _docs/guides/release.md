# Release

Day-to-day release workflow under the release-kit trunk convention. First time on a repository: [release-setup.md](./release-setup.md). The generic runbook is `rk guide release`. This guide carries the sequence with this repository's own facts filled in.

`Cargo.toml` is the version source of truth. release-plz reads the Conventional Commit messages of the squash commits on `master` and maintains one release pull request carrying the bump and the changelog. Merging that request is the release, and the `release-gate` job in `release-plz.yml` performs the merge once the `ci` workflow's `test` check is green on the request's head. The merge tags, publishes to crates.io over OIDC, and hands the tag to cargo-dist, which builds and attests the installers (ADR-adopt-the-release-kit-trunk-convention). Never author a tag. Never move a published one. Fix a bad release with the next version instead.

## Preconditions

- `gh` authenticated for this repository: `gh auth status`
- `rk` on `PATH`: `rk --version`
- The landing and the forge setup are green: `rk status --check --target .` exits 0, and `rk setup check --target .` reports no unsatisfied step. `install-bot` reads unknown on every host that lacks the App key, which is expected: the forge setup is complete

## At a glance

The whole sequence, in order. Each step is expanded below. `<repo>` is `gubasso/spec-driven-docs`.

```bash
# 1. land the work through rk integrate and a trunk push; the bot refreshes its release request
gh pr list --repo <repo> --state open

# 2. read the changelog on the request, and correct it on its branch before ci turns green

# 3. the release-gate job merges the release request once ci is green; this is the release decision
gh pr checks <release pr> --repo <repo> --watch

# 4. the merge tags and publishes; watch the publish half
gh run list --repo <repo> --workflow release-plz.yml --limit 1

# 5. wait for the build that creates the GitHub release
gh run watch --repo <repo> --exit-status <release.yml run>

# 6. verify
```

Two of these are easy to skip and both have bitten this repository. Step 2 is the only point a changelog correction still reaches the release, and the window closes when `test` turns green on the request's head. release-plz rewrites its branch, corrections included, whenever work lands on `master` while the request is open. Step 5 is why a check run straight after the merge reports the release as not found: cargo-dist creates it after the x86_64 Linux build finishes, a few minutes later.

1. Land the work on `master` through its one path: a short-lived branch in its worktree, `rk integrate <branch> -m "<message>" --apply` with a scoped Conventional Commit message, and `git push origin master`. Each trunk push makes release-plz refresh the release pull request so it always proposes releasing the trunk's tip:

   ```bash
   gh pr list --repo gubasso/spec-driven-docs --state open
   # check: a request titled "chore: release v<version>" is open, and align-manifest has realigned the canon manifest on its branch
   # none open: the trunk matches the last published version; there is nothing to release
   ```

2. Read the `CHANGELOG.md` entry on the release pull request and confirm it names every change the release carries. Compare it against the range since the last tag:

   ```bash
   git fetch origin --tags --force
   git log --oneline "v<previous version>^{commit}..origin/master"
   # check: the changelog entry names every change this range shows
   ```

   Correct it on the release pull request branch while `ci` still runs on its head. A pushed correction moves the head, so `ci` runs again and the gate judges the corrected head. A later push to `master` makes release-plz rewrite that branch and the correction with it. As a result, correct when the trunk is quiet:

   ```bash
   git fetch origin <release branch> && git switch --detach FETCH_HEAD
   # edit CHANGELOG.md, then commit and push it back
   git push origin HEAD:<release branch>
   # check: the release pull request shows the corrected entry
   ```

3. Let the `release-gate` job merge the release pull request. It wakes when `ci` completes, and it merges the request once `test` is green on that exact head. This is the release decision: the squash lands the bump on `master`, and the push of that squash is what the publish half keys on. To hold a release, mark the request as a draft with `gh pr ready <pr number> --undo`, because the gate never merges a draft. Closing the request abandons the release at no cost:

   ```bash
   gh pr checks <pr number> --repo gubasso/spec-driven-docs --watch
   gh pr view <pr number> --repo gubasso/spec-driven-docs --json state -q .state
   # check: prints MERGED, and master's tip carries the version bump and the changelog
   # OPEN after test passed: read the release-gate job log in the newest release-plz run, which names why it left the request open
   ```

4. Watch the publish half. On the bump push, `release-plz.yml` tags `v<version>` and publishes to crates.io over OIDC. The tag, pushed with the bot's token, triggers `release.yml`:

   ```bash
   gh run list --repo gubasso/spec-driven-docs --workflow release-plz.yml --limit 1
   # check: the newest run on master concluded success
   ```

5. Wait for the installer build before verifying anything. cargo-dist creates the GitHub release in its host job, after the x86_64 Linux build finishes and its artifacts are attested. As a result, no release exists for a few minutes after the merge, and `gh release view` reports that it is not found:

   ```bash
   gh run watch --repo gubasso/spec-driven-docs --exit-status \
     "$(gh run list --repo gubasso/spec-driven-docs --workflow release.yml \
        --limit 1 --json databaseId -q '.[0].databaseId')"
   # check: the watched run concludes success
   ```

6. Verify:

   ```bash
   # crates.io serves the new version
   cargo info spec-driven-docs

   # installers attached, never empty
   gh release view v<version> --repo gubasso/spec-driven-docs --json assets \
     -q '[.assets[].name] | join(", ")'

   # the artifacts attest to this repository
   gh release download v<version> --repo gubasso/spec-driven-docs \
     --pattern 'spec-driven-docs-installer.sh' --dir /tmp/rk-verify --clobber \
     && gh attestation verify /tmp/rk-verify/spec-driven-docs-installer.sh \
        --repo gubasso/spec-driven-docs
   # check: verification succeeded

   # the local clone may predate the tag push
   git fetch origin --tags --force

   # the tag and the trunk agree
   git rev-parse "v<version>^{commit}" origin/master
   # check: two identical SHAs
   # they differ: work landed after the release merge, so the tag sits one or more commits behind the tip; compare against the bump commit instead

   # the installed binary reports it
   sdd --version
   # check: prints the new version
   ```

   release-plz writes an annotated tag, so `v<version>` names a tag object rather than a commit. `^{commit}` is what makes the values comparable.

Recovery is `rk method recovery`, and it covers a failed publish, a wedged run, a yank, and a hand publish while CI is down. The changelog-correction window above is the only pre-merge repair a release needs.
