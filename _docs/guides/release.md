# Release

Day-to-day release workflow. First time on a repository: [release-setup.md](./release-setup.md).

`Cargo.toml` is the version source of truth. release-plz reads Conventional Commits, bumps the version, writes the changelog, tags, and publishes. Never author a tag; never move a published one — fix a bad release with the next version.

A release takes two pull requests. The first, which release-plz opens against `develop`, carries the version bump and the changelog, and merging it publishes nothing. The second is the gate: automation cuts `release/v<version>` at that merged commit and opens it into `master`, which takes no direct push and requires a passing `test` check, and merging it is what tags and publishes (ADR-cut-the-release-from-master). The gate branch is pinned to one commit, so work landing on `develop` while it is open never joins the release.

## At a glance

The whole sequence, in order. Each step is expanded below. `<repo>` is `gubasso/spec-driven-docs`.

```bash
# 1. land the work
just check

# 2. push; release-plz opens the release request
git push origin develop

# 3. read the changelog in that request, and correct it on its branch before merging

# 4. merge the release request; the bump lands on develop
gh pr merge <release pr> --repo <repo> --squash --delete-branch

# 5. the gate opens itself
gh pr list --repo <repo> --base master --state open

# 6. merge the gate; this tags and publishes
gh pr checks <gate pr> --repo <repo> --watch \
  && gh pr merge <gate pr> --repo <repo> --merge --delete-branch

# 7. back-merge
git fetch origin --tags --force \
  && git merge --ff-only origin/master && git push origin develop

# 8. wait for the build that creates the release
gh run watch --repo <repo> --exit-status <release.yml run>

# 9. verify
```

Two of these are easy to skip and both have bitten this repository. Step 3 is the only point a changelog correction still reaches the release, and release-plz drops entries whenever work lands while the request is open. Step 8 is why a check run straight after the merge reports the release as not found: cargo-dist creates it after every platform builds, about six minutes later.

1. Land the work on `develop` with Conventional Commit messages (`feat:` bumps minor, `fix:` bumps patch):

   ```bash
   just check
   ```

2. Push. release-plz opens the release pull request and realigns the canon manifest onto its branch:

   ```bash
   git push origin develop
   # check: run succeeded, release pull request is open
   gh run list --repo gubasso/spec-driven-docs --workflow release-plz.yml --limit 1
   gh pr list --repo gubasso/spec-driven-docs --state open
   ```

3. Read the `CHANGELOG.md` entry in the release pull request and confirm it names every change the release carries. release-plz writes that entry when it opens the request and does not regenerate it as later work lands, so a request left open while you keep committing ships a changelog that omits the newer work. Compare it against the range:

   ```bash
   git fetch origin --tags --force
   git log --oneline "v<previous version>^{commit}..origin/develop"
   ```

   Correct it on the release pull request branch, before merging. That is the last point a correction reaches the release: merging cuts the gate at the merge commit, the gate branch stays pinned there, and a published entry can never be edited.

   ```bash
   git fetch origin <release branch> && git switch --detach FETCH_HEAD
   # edit CHANGELOG.md, then commit and push it back
   git push origin HEAD:<release branch>
   ```

4. Merge the release pull request into `develop`. It bumps the version and writes the changelog; nothing is published yet:

   ```bash
   gh pr merge <pr number> --repo gubasso/spec-driven-docs --squash --delete-branch
   ```

5. Wait for the release gate. That merge pushes `develop`, and the `open-release-gate` job cuts `release/v<version>` at the merged commit and opens it into `master`, because the new version carries no tag yet:

   ```bash
   # check: a pull request titled "release v<version>", base master, head release/v<version>
   gh pr list --repo gubasso/spec-driven-docs --base master --state open
   ```

6. Merge the gate, once its checks are green. `master-protection` refuses the merge on its own while `test` is failing, and `gh pr checks --watch` blocks until every check settles and exits non-zero on a failure. The merge must be a merge commit: GitHub offers no fast-forward merge method, and a rebase or squash would make `master` diverge from `develop` permanently. Merging tags `v<version>` on `master`, publishes over OIDC, and builds installers:

   ```bash
   gh pr checks <pr number> --repo gubasso/spec-driven-docs --watch \
     && gh pr merge <pr number> --repo gubasso/spec-driven-docs --merge --delete-branch
   # check: post-merge run on master succeeded
   gh run list --repo gubasso/spec-driven-docs --workflow release-plz.yml --limit 1
   ```

7. Back-merge, so `develop` reaches the tagged commit and the next release diffs cleanly. It runs safely before the tag lands, because the gate job refuses a commit that is not ahead of `master` and the back-merge leaves the two equal. It is a fast-forward while `develop` has not moved since the gate was cut; if work landed meanwhile, drop `--ff-only` and take the merge commit:

   ```bash
   git fetch origin --tags --force
   git checkout develop && git merge --ff-only origin/master
   git push origin develop
   ```

8. Wait for the installer build before verifying anything. cargo-dist creates the GitHub release in its final job, after every platform has built, so for about six minutes after the merge there is no release to look at and `gh release view` reports that it is not found. The tag lands sooner, around a minute in, but a check run before either arrives fails on timing rather than on the release:

   ```bash
   gh run watch --repo gubasso/spec-driven-docs --exit-status \
     "$(gh run list --repo gubasso/spec-driven-docs --workflow release.yml \
        --limit 1 --json databaseId -q '.[0].databaseId')"
   ```

9. Verify:

   ```bash
   # crates.io serves the new version
   cargo info spec-driven-docs

   # installers attached, never empty
   gh release view v<version> --repo gubasso/spec-driven-docs --json assets \
     -q '[.assets[].name] | join(", ")'

   # the local clone may predate the tag push
   git fetch origin --tags --force

   # all three agree
   git rev-parse "v<version>^{commit}" origin/master origin/develop

   # the installed binary reports it
   sdd --version
   ```

   release-plz writes an annotated tag, so `v<version>` names a tag object rather than a commit; `^{commit}` is what makes the three values comparable.
