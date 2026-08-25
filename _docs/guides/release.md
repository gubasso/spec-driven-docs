# Release

`Cargo.toml` is the version source of truth. release-plz reads Conventional Commits, bumps the
version, writes the changelog, tags, and publishes. Nobody authors a tag by hand, and a published
tag or version is never moved — a bad release is fixed by the next version.

## Cut a release

1. Land the work on `develop` with Conventional Commit messages (`feat:` bumps minor, `fix:`
   bumps patch) and a green tree:

   ```bash
   just check
   ```

2. Push `develop`. release-plz opens the release pull request — the version bump, the changelog,
   and the regenerated canon manifest ride in it; merging it is the release. Automation then tags
   `v<version>`, publishes to crates.io over OIDC, builds the installers, and fast-forwards
   `master` to the tag.

3. Verify:

   ```bash
   cargo info spec-driven-docs
   gh release view v0.3.0 --repo gubasso/spec-driven-docs
   sdd --version
   ```

## First-time setup

Once, in this order.

1. Make `develop` the default branch:

   ```bash
   gh repo edit gubasso/spec-driven-docs --default-branch develop
   ```

2. Let Actions write and open pull requests:

   ```bash
   gh api -X PUT repos/gubasso/spec-driven-docs/actions/permissions/workflow \
     -f default_workflow_permissions=write \
     -F can_approve_pull_request_reviews=true
   ```

3. Create the GitHub App at <https://github.com/settings/apps/new>: name `gubasso-ci-bot`,
   permissions Contents read-write and Pull requests read-write, webhook off. Install it on the
   repository, generate a private key, then:

   ```bash
   gh secret set RELEASE_PLZ_APP_ID --repo gubasso/spec-driven-docs --body '<app id>'
   gh secret set RELEASE_PLZ_APP_PRIVATE_KEY --repo gubasso/spec-driven-docs \
     < gubasso-ci-bot.private-key.pem
   ```

4. Create three rulesets under Settings, Rules, Rulesets:

   - `master-protection` on `master`: require linear history, require the `test` status check,
     block force pushes and deletion; bypass actor `gubasso-ci-bot`.
   - `develop-protection` on `develop`: block force pushes and deletion.
   - `release-tags` on `v*` tags: block update and deletion.

5. Publish 0.2.0 by hand — trusted publishing only attaches to an existing crate:

   ```bash
   scripts/publish-dry
   # create a publish-new scoped token at https://crates.io/settings/tokens
   cargo login
   scripts/publish
   ```

6. Register the trusted publisher on the crate's Settings page at crates.io: repository
   `gubasso/spec-driven-docs`, workflow filename `release-plz.yml` — never `release.yml` or
   `ci.yml`. Revoke the bootstrap token.

7. After the first automated release succeeds over OIDC, enable "Require trusted publishing for
   all new versions" on the same crates.io settings page, so every token publish is rejected.

8. After the first automated release creates `master`, delete `main`:

   ```bash
   gh api -X DELETE repos/gubasso/spec-driven-docs/git/refs/heads/main
   ```
