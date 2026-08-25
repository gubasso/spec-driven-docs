# Release

Day-to-day release workflow. First time on a repository: complete [release-setup.md](./release-setup.md) first.

`Cargo.toml` is the version source of truth. release-plz reads Conventional Commits, bumps the version, writes the changelog, tags, and publishes. Never author a tag; never move a published tag or version — fix a bad release with the next version.

1. Land the work on `develop` with Conventional Commit messages (`feat:` bumps minor, `fix:` bumps patch) and a green tree:

   ```bash
   just check
   ```

2. Push `develop`. release-plz opens the release pull request carrying the version bump, the changelog, and the regenerated canon manifest:

   ```bash
   git push origin develop
   # check: latest run completed with success
   gh run list --repo gubasso/spec-driven-docs --workflow release-plz.yml --limit 1
   # check: the release pull request is open
   gh pr list --repo gubasso/spec-driven-docs --state open
   ```

3. Merge the release pull request. Automation tags `v<version>`, publishes to crates.io over OIDC, builds the installers, and fast-forwards `master` to the tag:

   ```bash
   gh pr merge <pr number> --repo gubasso/spec-driven-docs --squash
   # check: the post-merge run completed with success
   gh run list --repo gubasso/spec-driven-docs --workflow release-plz.yml --limit 1
   ```

4. Verify the release landed everywhere:

   ```bash
   # check: crates.io serves the new version
   cargo info spec-driven-docs
   # check: the GitHub release exists with installers attached
   gh release view v<version> --repo gubasso/spec-driven-docs
   # check: master sits on the tag
   git fetch origin && git rev-parse v<version> origin/master
   # check: the installed binary reports the new version
   sdd --version
   ```
