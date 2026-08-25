# Release setup

One-time bootstrap for automated releases. Run once, in order: each step requires the one before it. Day-to-day releases: [release.md](./release.md).

1. Make `develop` the default branch, so pushes to `develop` drive release-plz:

   ```bash
   gh repo edit gubasso/spec-driven-docs --default-branch develop
   # check: prints "develop"
   gh repo view gubasso/spec-driven-docs --json defaultBranchRef -q .defaultBranchRef.name
   ```

2. Let Actions write and open pull requests, so the workflow can act on that branch:

   ```bash
   gh api -X PUT repos/gubasso/spec-driven-docs/actions/permissions/workflow \
     -f default_workflow_permissions=write \
     -F can_approve_pull_request_reviews=true
   # check: prints "write true"
   gh api repos/gubasso/spec-driven-docs/actions/permissions/workflow \
     -q '"\(.default_workflow_permissions) \(.can_approve_pull_request_reviews)"'
   ```

3. Create the GitHub App at <https://github.com/settings/apps/new> — name `gubasso-ci-bot`, permissions Contents read-write and Pull requests read-write, webhook off. Install it on the repository, generate a private key, then store its credentials:

   ```bash
   gh secret set RELEASE_PLZ_APP_ID --repo gubasso/spec-driven-docs --body '<app id>'
   gh secret set RELEASE_PLZ_APP_PRIVATE_KEY --repo gubasso/spec-driven-docs \
     < gubasso-ci-bot.private-key.pem
   # check: prints "gubasso-ci-bot"
   gh api repos/gubasso/spec-driven-docs/installation -q .app_slug
   # check: lists both secrets
   gh secret list --repo gubasso/spec-driven-docs
   ```

4. Create three rulesets under Settings, Rules, Rulesets — the app from step 3 must exist to be named as bypass actor:

   - `master-protection` on `master`: require linear history, require the `test` status check, block force pushes and deletion; bypass actor `gubasso-ci-bot`.
   - `develop-protection` on `develop`: block force pushes and deletion.
   - `release-tags` on `v*` tags: block update and deletion.

   ```bash
   # check: lists the three ruleset names
   gh api repos/gubasso/spec-driven-docs/rulesets -q '.[].name'
   ```

5. Publish the first version by hand — trusted publishing only attaches to an existing crate:

   ```bash
   scripts/publish-dry
   # create a publish-new scoped token at https://crates.io/settings/tokens
   cargo login
   scripts/publish
   # check: prints the crate with the published version
   cargo info spec-driven-docs
   ```

6. Register the trusted publisher on the crate's Settings page at crates.io: repository `gubasso/spec-driven-docs`, workflow filename `release-plz.yml` — never `release.yml` or `ci.yml`. Check on the same page: the publisher is listed under Trusted Publishing.

7. Revoke the bootstrap token from step 5. Check at <https://crates.io/settings/tokens>: the token is gone.

8. Cut the first automated release: follow [release.md](./release.md). Its verify step is the check; it must publish over OIDC and fast-forward `master`.

9. Enable "Require trusted publishing for all new versions" on the crates.io settings page, so every token publish is rejected — safe only after step 8 proved OIDC works. Check on the same page: the setting shows enabled.

10. Delete `main`, now that step 8 created `master`:

    ```bash
    gh api -X DELETE repos/gubasso/spec-driven-docs/git/refs/heads/main
    # check: prints nothing
    git ls-remote --heads origin main
    ```
