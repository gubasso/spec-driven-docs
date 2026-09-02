# Release

Day-to-day release workflow under the release-kit trunk convention. First time on a repository: [release-setup.md](./release-setup.md). The generic runbook is `rk guide release`; this guide carries the sequence with this repository's own facts filled in.

`Cargo.toml` is the version source of truth. release-plz reads the Conventional Commit titles squash-merged onto `master`, maintains one release pull request carrying the bump and the changelog, and merging that request is the release: it tags, publishes to crates.io over OIDC, and hands the tag to cargo-dist, which builds and attests the installers (ADR-adopt-the-release-kit-trunk-convention). Never author a tag; never move a published one — fix a bad release with the next version.

## Preconditions

- `gh` authenticated for this repository: `gh auth status`
- `rk` on `PATH`: `rk --version`
- The landing and the forge setup are green: `rk status --check --target .` and `rk setup check --target .` both exit 0

## At a glance

The whole sequence, in order. Each step is expanded below. `<repo>` is `gubasso/spec-driven-docs`.

```bash
# 1. land the work through squash-merged pull requests; the bot refreshes its release request
gh pr list --repo <repo> --state open

# 2. read the changelog on the request, and correct it on its branch just before merging

# 3. merge the release request; this is the release decision
gh pr merge <release pr> --repo <repo> --squash

# 4. the merge tags and publishes; watch the publish half
gh run list --repo <repo> --workflow release-plz.yml --limit 1

# 5. wait for the build that creates the GitHub release
gh run watch --repo <repo> --exit-status <release.yml run>

# 6. verify
```

Two of these are easy to skip and both have bitten this repository. Step 2 is the only point a changelog correction still reaches the release, and release-plz rewrites its branch — corrections included — whenever work lands on `master` while the request is open. Step 5 is why a check run straight after the merge reports the release as not found: cargo-dist creates it after every platform builds, about six minutes later.

1. Land the work on `master` through its one path: a short-lived branch in its worktree, a pull request whose title is a scoped Conventional Commit, a squash merge. Each landing makes release-plz refresh the release pull request so it always proposes releasing the trunk's tip:

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

   Correct it on the release pull request branch, just before merging. A later push to `master` makes release-plz rewrite that branch and the correction with it, so correct when the trunk is quiet and merge before it moves:

   ```bash
   git fetch origin <release branch> && git switch --detach FETCH_HEAD
   # edit CHANGELOG.md, then commit and push it back
   git push origin HEAD:<release branch>
   # check: the release pull request shows the corrected entry
   ```

3. Merge the release pull request, once its checks are green. This is the release decision: the squash lands the bump on `master`, and the push of that squash is what the publish half keys on. Closing the request instead abandons the release at no cost:

   ```bash
   gh pr checks <pr number> --repo gubasso/spec-driven-docs --watch \
     && gh pr merge <pr number> --repo gubasso/spec-driven-docs --squash
   # check: exits 0; master's tip carries the version bump and the changelog
   ```

4. Watch the publish half. On the bump push, `release-plz.yml` tags `v<version>` and publishes to crates.io over OIDC; the tag, pushed with the bot's token, triggers `release.yml`:

   ```bash
   gh run list --repo gubasso/spec-driven-docs --workflow release-plz.yml --limit 1
   # check: the newest run on master concluded success
   ```

5. Wait for the installer build before verifying anything. cargo-dist creates the GitHub release in its host job, after every platform has built and its artifacts are attested, so for about six minutes after the merge there is no release to look at and `gh release view` reports that it is not found:

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

   release-plz writes an annotated tag, so `v<version>` names a tag object rather than a commit; `^{commit}` is what makes the values comparable.

Recovery — a failed publish, a wedged run, a yank, a hand publish while CI is down — is `rk method recovery`, and the changelog-correction window above is the only pre-merge repair a release needs.
