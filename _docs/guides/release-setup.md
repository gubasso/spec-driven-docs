# Release setup

One-time bootstrap for automated releases under the release-kit convention. Run once, in order: each step requires the one before it. Day-to-day releases: [release.md](./release.md). The generic runbook is `rk guide setup`, which owns each step's how; `rk method setup` owns its why. This guide carries the repository's own values and what the runbook leaves to it, and every step is rerunnable: a satisfied step reports itself satisfied rather than failing.

## Preconditions

- `rk` on `PATH`: `rk --version`
- `gh` authenticated as the repository owner with `repo` and `workflow` scope: `gh auth status`
- Clean trunk: `git status --porcelain` prints nothing
- The crates.io account owns `spec-driven-docs` with a verified email: `cargo info spec-driven-docs` prints the crate

1. Prove the package is publishable, before anything that needs credentials:

   ```bash
   rk setup step package-check --target . --apply
   # check: reports the package builds and passes the registry's dry run
   ```

2. Provide the GitHub App `gubasso-ci-bot` and grant it this repository. The field-by-field walkthrough is `rk forge github`; it happens once per account, and this account already carries the App. Collect its credentials:

   1. Open <https://github.com/settings/apps/gubasso-ci-bot>.
      - Read the App ID from the "About" section at the top.
      - Under "Private keys", click Generate a private key; the `.pem` downloads once and is never shown again.
   2. Fill `RK_BOT_APP_ID` and `RK_BOT_PRIVATE_KEY_FILE` in `.envrc.local`, then load them:

      ```bash
      cp -n .envrc.local.example .envrc.local
      $EDITOR .envrc.local
      direnv allow
      # check: printenv RK_BOT_APP_ID prints the App ID
      ```

3. Store the bot credentials as repository secrets, with step 2's exports in the environment, on a host shell:

   ```bash
   rk setup step install-bot --target . --apply
   rk setup step bot-secrets --target . --apply
   gh secret list --repo gubasso/spec-driven-docs
   # check: lists RELEASE_BOT_APP_ID and RELEASE_BOT_APP_PRIVATE_KEY
   ```

4. Assert the forge shape — default branch, single trunk, merge cleanup, CI permissions, and the protections. `test` is the CI job the trunk requires; `.github/workflows/ci.yml` names it:

   ```bash
   rk setup --target . --apply --required-check test
   rk setup check --target .
   # check: every step reports satisfied; protect-release-lines reports skipped while no older line exists
   # single-trunk refuses a candidate: it is not an ancestor of master, so land its work first; the guard failing closed is the stop, not an obstacle
   ```

5. Confirm the landed payload is current — the files `rk init` landed, the two spliced blocks, and the landing record:

   ```bash
   rk status --check --target .
   # check: exits 0 and reports the worktree mode
   # drift on an rk-owned file: rk upgrade --target . --apply takes the landing to the binary's payload
   ```

6. Register the trusted publisher, once per package, in the browser — `rk guide setup` step 6 carries the form walkthrough. The values for this repository: owner `gubasso`, repository `spec-driven-docs`, workflow filename `release-plz.yml` — never `release.yml`, which builds installers, and never `ci.yml` — and Environment left empty.

   - check: the Trusted Publishing table at <https://crates.io/crates/spec-driven-docs/settings> lists those three values
   - already listed: the publisher is registered; continue

7. Cut one release end to end, which is what proves OIDC works before step 8 makes it mandatory.

   1. Follow [release.md](./release.md) end to end.
   2. Check: its verify step passes — crates.io serves the new version, the attestation verifies, and the tag sits on `master`.

8. Require trusted publishing, now that step 7 proved it.

   1. Open <https://crates.io/crates/spec-driven-docs/settings>.
   2. Enable "Require trusted publishing for all new versions", which makes crates.io reject every token publish.
   3. Check on the same page: the setting shows enabled.
   4. The recovery hand-publish of `rk method recovery` starts by turning this off.

## Verification

```bash
rk setup check --target . && rk status --check --target .
# check: both exit 0
```
