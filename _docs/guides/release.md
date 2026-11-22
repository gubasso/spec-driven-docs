# Release

Day-to-day release workflow under the trunk convention that `rk` runs. First time on a repository: [release-setup.md](./release-setup.md). The generic runbook is `rk guide release`. This guide carries the sequence with this repository's own facts filled in.

`Cargo.toml` is the version source of truth. release-plz reads the Conventional Commit messages of the squash commits on `master`. It keeps one release pull request open, which carries the bump and the changelog. Merging that request is the release.

Only `rk release merge` merges the release request. It dispatches the `release-merge` job of `release-plz.yml`, which runs as the release App. The job proves that an administrator ordered the merge, that `test` is green on the request's head, and that neither the head nor the trunk moved. It then squash-merges with the head pinned. No job merges the request by itself, so a release waits until the operator orders it.

The merge tags, publishes to crates.io over OIDC, and hands the tag to cargo-dist, which builds and attests the installers (ADR-adopt-the-release-kit-trunk-convention). Never author a tag. Never move a published one. Fix a bad release with the next version instead.

## Preconditions

- `gh` authenticated for this repository: `gh auth status`
- `rk` on `PATH`: `rk --version`
- The operator holds the administrator role on the repository, because the merge job refuses any other caller: `gh api repos/gubasso/spec-driven-docs --jq .permissions.admin` prints `true`
- The landing and the forge setup are green: `rk status --check --target .` exits 0, and `rk setup check --target .` reports no unsatisfied step. `install-bot` reads unknown on every host that lacks the App key, which is expected: the forge setup is complete

## At a glance

The whole sequence, in order. Each step is expanded below. `<repo>` is `gubasso/spec-driven-docs`.

```bash
# 1. land the work through rk integrate and a trunk push; the bot refreshes its release request
gh pr list --repo <repo> --state open

# 2. read the changelog on the request, and correct it on its branch before you order the merge

# 3. make sure that the request is not armed
rk release status --target .

# 4. wait for the checks on the request's head
gh pr checks <release pr> --repo <repo> --watch

# 5. preview the merge
rk release merge --target .

# 6. order the merge; this is the release decision
rk release merge --target . --apply

# 7. the merge tags and publishes; watch the publish half on the merge commit
gh run list --repo <repo> --workflow release-plz.yml --commit "$(git rev-parse origin/master)"

# 8. wait for the build that creates the GitHub release

# 9. verify

# 10. move this repository to the new sdd and rk releases, as every consumer does:
# edit both tags in flake.nix, then
nix flake update spec-driven-docs release-kit
```

Two of these are easy to skip and both have bitten this repository. Step 2 is the only point a changelog correction still reaches the release, and the window closes when you order the merge in step 6. release-plz rewrites its branch, corrections included, whenever work lands on `master` while the request is open. Step 8 is why a check run straight after the merge reports the release as not found: cargo-dist creates it after the x86_64 Linux build finishes, a few minutes later.

A release that must not ship needs no action, because the request waits for the order of step 6. Closing the request abandons the release at no cost. After the merge, the version is public, and the way back is the withdrawal in `rk method recovery`.

1. Land the work on `master` through its one path: a short-lived branch in its worktree, `rk integrate <branch> -m "<message>" --apply` with a scoped Conventional Commit message, and `git push origin master`. Each trunk push makes release-plz refresh the release pull request so it always proposes releasing the trunk's tip:

   ```bash
   gh pr list --repo gubasso/spec-driven-docs --state open
   # check: a request titled "chore: release v<version>" is open, and align-manifest has realigned the payload record on its branch
   # none open: rk release eligible says why; a trunk that matches the last published version has nothing to release
   ```

2. Read the `CHANGELOG.md` entry on the release pull request and confirm it names every change the release carries. Compare it against the range since the last tag:

   ```bash
   git fetch origin --tags --force
   git log --oneline "v<previous version>^{commit}..origin/master"
   # check: the changelog entry names every change this range shows
   # nothing missing: go to step 3
   ```

   Correct it on the release pull request branch before you order the merge. A pushed correction moves the head, so `ci` runs again, and step 4 waits for `test` on the corrected head. A later push to `master` makes release-plz rewrite that branch and the correction with it. As a result, correct when the trunk is quiet:

   ```bash
   git fetch origin <release branch> && git switch --detach FETCH_HEAD
   # edit CHANGELOG.md, then commit and push it back
   git push origin HEAD:<release branch>
   # check: the release pull request shows the corrected entry, under the same request number
   # a new number, or an entry without the correction: a refresh took the fix; redo it on the request that stands now
   ```

   An adopter note is the part of the changelog entry that tells a consumer what to do. Where the release asks a consumer to act, end its adopter note with the moved text that `setup:a-pin-move-names-the-skill-route` holds:

   ```text
   After the pin move, load the devshell and run the upgrade through the sdd-setup skill of the sdd in that devshell, and do every migration it names. Before `sdd upgrade`, read it with `sdd skill show sdd-setup`. After it, the project's .claude/skills/sdd-setup is that skill. Never use a host copy or an older skill.
   ```

   Where the release changes the managed documentation block of `AGENTS.md`, the adopter note names each line that the block gains or loses. The budget of `agents-digest-size` counts only the project's own lines, so a block change never fails it in a project. `the_rendered_block_fits_the_managed_block_cap` holds the block to 20 lines at this repository's commit.

3. Read the release state, and make sure that nothing armed the request to merge itself:

   ```bash
   rk release status --target .
   # check: the report names the open request and lists no request-armed symptom
   # request-armed: rk setup step merge-on-order --target . --apply disarms every open release request; then read the state again
   ```

4. Wait for the checks on the request's head:

   ```bash
   gh pr checks <release pr> --repo gubasso/spec-driven-docs --watch
   # check: every check passes, test included, and the branch is current with master
   # behind master: wait for release-plz to rebuild the request on the trunk tip, then watch again
   ```

5. Preview the merge. The preview reads the request and writes nothing:

   ```bash
   rk release merge --target .
   # check: prints the request, the trunk tip, the head, the check, and the version you read in step 2
   ```

6. Order the merge. This is the release decision: the squash lands the bump on `master`, and the push of that squash is what the publish half keys on. Never merge the request through the forge by hand:

   ```bash
   rk release merge --target . --apply
   # check: prints merged, and the rk release status --wait command for the merge commit
   # refused: the output names the proof that failed (the request moved, the trunk moved, test is not green, or the caller is no administrator); the request stays open, so go back to step 4
   ```

7. Watch the publish half. On the bump push, `release-plz.yml` tags `v<version>` and publishes to crates.io over OIDC. The tag, pushed with the bot's token, triggers `release.yml`. Select the run by the merge commit, never by recency:

   ```bash
   git fetch origin
   gh run watch --repo gubasso/spec-driven-docs --exit-status \
     "$(gh run list --repo gubasso/spec-driven-docs --workflow release-plz.yml \
        --commit "$(git rev-parse origin/master)" --event push --limit 1 \
        --json databaseId -q '.[0].databaseId')"
   # check: the watched run concludes success
   ```

8. Wait for the installer build before verifying anything. cargo-dist creates the GitHub release in its host job, after the x86_64 Linux build finishes and its artifacts are attested. As a result, no release exists for a few minutes after the merge, and `gh release view` reports that it is not found:

   ```bash
   gh run watch --repo gubasso/spec-driven-docs --exit-status \
     "$(gh run list --repo gubasso/spec-driven-docs --workflow release.yml \
        --commit "$(git rev-parse origin/master)" --event push --limit 1 \
        --json databaseId -q '.[0].databaseId')"
   # check: the watched run concludes success
   # empty run id: the tag has not landed yet; rerun after a few seconds, and if it stays empty, follow rk method recovery
   ```

9. Verify:

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
   ```

   release-plz writes an annotated tag, so `v<version>` names a tag object rather than a commit. `^{commit}` is what makes the values comparable.

10. Move this repository to the new releases of sdd and rk in one pin move. Its devshell pins the previous releases, which land, verify, and upgrade its own instances, as in every consumer (release:the-canon-runs-its-previous-release). The operator moves each pin by editing its tag (release:the-rk-pin-has-two-facts-and-one-mover). Make the pin move in a new worktree on a `build/<slug>` branch:

    1. Edit the tag of the `spec-driven-docs` input and the tag of the `release-kit` input in `flake.nix`:

       ```text
       url = "github:gubasso/spec-driven-docs/v<version>";
       url = "github:gubasso/release-kit/v<rk version>";
       ```

    2. Update the two inputs in the lock:

       ```bash
       nix flake update spec-driven-docs release-kit
       # check: flake.nix names both new tags, and git diff flake.lock shows both nodes moved
       ```

    3. Once, where the new release itself has a spec-driven-docs input and `flake.nix` does not yet cut it, add `inputs.spec-driven-docs.follows = "";` to the spec-driven-docs input of `flake.nix`, then lock:

       ```bash
       nix flake lock
       # check: jq '[.nodes | keys[] | select(startswith("spec-driven-docs"))] | length' flake.lock prints 1
       ```

    4. Load the new devshell:

       ```bash
       direnv reload && sdd --version && rk --version
       # check: prints sdd <version> and rk <rk version>
       ```

    5. Upgrade the sdd instance through the sdd-setup skill of the sdd in the devshell:

       ```bash
       sdd skill show sdd-setup
       sdd upgrade --target .
       # check: the upgrade lands, and sdd verify --target . prints OK spec-driven-docs <version>
       ```

    6. Upgrade the rk instance through the rk-setup skill of the rk in the devshell, as `AGENTS.md` requires:

       ```bash
       rk skill show rk-setup
       rk upgrade --target . --apply
       # check: the upgrade lands, and rk status --check --target . exits 0
       ```

    Commit the pin move and the upgraded instances as one change, and land it as in step 1.

Recovery is `rk method recovery`, and it covers a failed publish, a wedged run, a yank, and a hand publish while CI is down. The changelog-correction window above is the only pre-merge repair a release needs.
