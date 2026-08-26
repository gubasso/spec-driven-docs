# Release setup

One-time bootstrap for automated releases. Run once, in order: each step requires the one before it. Day-to-day releases: [release.md](./release.md).

Every command below is a script under `scripts/release-setup/`, run from the repository root, and each one prints its own check. They read `SDD_REPO`, which `.envrc` derives from the `repository` field in `Cargo.toml`, and the credentials from `.envrc.local`, which is gitignored. `direnv allow` loads both; without direnv, export the same names by hand. `.envrc` also blanks `GH_PAGER` so the scripts never stall in a pager, and the ruleset scripts update in place, so a rerun is safe.

1. Make `develop` the default branch, so pushes to `develop` drive release-plz:

   ```bash
   scripts/release-setup/default-branch
   ```

2. Let Actions write and open pull requests, so the workflow can act on that branch:

   ```bash
   scripts/release-setup/actions-permissions
   ```

3. Provide the GitHub App `gubasso-ci-bot` and grant it this repository. The app REST endpoints need app JWT auth, so a user token cannot query them — do the checks on the web pages.

   1. Open <https://github.com/settings/apps>.
      - `gubasso-ci-bot` is listed: go to substep 3.
      - It is missing: go to substep 2.
   2. Create the app at <https://github.com/settings/apps/new>.
      - Name: `gubasso-ci-bot`.
      - Homepage URL: any URL you own; it is unused.
      - Webhook: clear the Active checkbox.
      - Repository permissions: Contents read and write, Pull requests read and write, Metadata read-only.
      - Click Create GitHub App, then go to substep 3.
   3. Open <https://github.com/settings/apps/gubasso-ci-bot/installations> and find the `gubasso` row under "Choose an account to install gubasso-ci-bot on".
      - The row shows an active Install button: click it, then go to substep 5.
      - The row shows a greyed-out Installed button and a gear icon: click the gear, then go to substep 4.
   4. The gear opens `https://github.com/settings/installations/<installation id>`. Read the "Repository access" radio group.
      - "All repositories" is selected: the repository is already covered, go to substep 6.
      - "Only select repositories" is selected: open its dropdown, select `spec-driven-docs`, click Save, then go to substep 6.
   5. The install screen shows the same "Repository access" radio group.
      - Select "Only select repositories".
      - Open the dropdown and select `spec-driven-docs`.
      - Click Install.
   6. Check the grant landed.
      - Open <https://github.com/gubasso/spec-driven-docs/settings/installations>.
      - It lists `gubasso-ci-bot` with a Configure button.
   7. Open <https://github.com/settings/apps/gubasso-ci-bot> and collect the credentials.
      - Read the App ID from the "About" section at the top.
      - Under "Private keys", click Generate a private key; the `.pem` downloads once and is never shown again.
   8. Fill in `SDD_APP_ID` and `SDD_APP_PEM` in `.envrc.local`, then store them as repository secrets:

      ```bash
      cp -n .envrc.local.example .envrc.local
      $EDITOR .envrc.local
      direnv allow
      scripts/release-setup/app-secrets
      ```

4. Create three rulesets. The app from step 3 must exist first: a bypass actor is named by numeric id, so `SDD_APP_ID` must already be set from substep 3.8. Rulesets on a private repository require a paid plan.

   1. Protect `master`, naming the app as a bypass actor so the promote job can fast-forward it:

      ```bash
      scripts/release-setup/ruleset-master
      ```

   2. Protect `develop`, which nothing in the pipeline force-pushes or deletes:

      ```bash
      scripts/release-setup/ruleset-develop
      ```

   3. Make release tags immutable:

      ```bash
      scripts/release-setup/ruleset-tags
      ```

   4. Check all three exist and that the bypass actor landed:

      ```bash
      scripts/release-setup/rulesets-check
      ```

5. Publish the first version by hand, because a trusted publisher can only attach to a crate that already exists.

   1. Confirm the package builds and is publishable:

      ```bash
      scripts/publish-dry
      ```

   2. Create a scoped token at <https://crates.io/settings/tokens>.
      - Scopes: `publish-new`.
      - Crates: leave unrestricted; the crate does not exist yet.
      - Expiry: the shortest offered; step 7 revokes it either way.
   3. Authenticate and publish:

      ```bash
      cargo login
      scripts/publish
      # check: prints the crate with the published version
      cargo info spec-driven-docs
      ```

6. Register the trusted publisher, so the workflow can publish over OIDC instead of a token.

   1. Open the crate's Settings tab at <https://crates.io/crates/spec-driven-docs/settings>.
   2. Next to "Trusted Publishing", click Add.
      - Repository owner: `gubasso`.
      - Repository name: `spec-driven-docs`.
      - Workflow filename: `release-plz.yml` — never `release.yml`, which builds installers, and never `ci.yml`.
      - Environment: leave empty; the release job declares no environment.
   3. Check on the same page: the publisher is listed under Trusted Publishing.

7. Revoke the bootstrap token from step 5, so the crate has exactly one publishing path.

   1. Open <https://crates.io/settings/tokens>.
   2. Revoke the token created in substep 5.2.
   3. Check on the same page: the token is gone.

8. Cut the first automated release, which is what proves OIDC works before step 9 makes it mandatory.

   1. Follow [release.md](./release.md) end to end.
   2. Check: its verify step passes — crates.io serves the new version, the tag exists, and `master` sits on it.

9. Require trusted publishing, now that step 8 proved it works.

   1. Open <https://crates.io/crates/spec-driven-docs/settings>.
   2. Enable "Require trusted publishing for all new versions", which makes crates.io reject every token publish.
   3. Check on the same page: the setting shows enabled.

10. Delete `main`, now that step 8 created `master`:

    ```bash
    scripts/release-setup/delete-main
    ```
