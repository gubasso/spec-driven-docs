# Release

Day-to-day release workflow. First time on a repository: [release-setup.md](./release-setup.md).

`Cargo.toml` is the version source of truth. release-plz reads Conventional Commits, bumps the version, writes the changelog, tags, and publishes. Never author a tag; never move a published one — fix a bad release with the next version.

1. Land the work on `develop` with Conventional Commit messages (`feat:` bumps minor, `fix:` bumps patch):

   ```bash
   just check
   ```

2. Push. release-plz opens the release pull request:

   ```bash
   git push origin develop
   # check: run succeeded, release pull request is open
   gh run list --repo gubasso/spec-driven-docs --workflow release-plz.yml --limit 1
   gh pr list --repo gubasso/spec-driven-docs --state open
   ```

3. Merge it, once its checks are green. `develop` takes direct pushes, so it carries no required status check and nothing on GitHub's side refuses a red merge; this chain is the gate (ADR-gate-the-release-merge-in-the-recipe). `gh pr checks --watch` blocks until every check settles and exits non-zero on a failure, so the merge never fires on a red or pending pull request. Automation then tags `v<version>`, publishes over OIDC, builds installers, fast-forwards `master`:

   ```bash
   gh pr checks <pr number> --repo gubasso/spec-driven-docs --watch \
     && gh pr merge <pr number> --repo gubasso/spec-driven-docs --squash
   # check: post-merge run succeeded
   gh run list --repo gubasso/spec-driven-docs --workflow release-plz.yml --limit 1
   ```

4. Verify:

   ```bash
   cargo info spec-driven-docs                                    # crates.io serves the new version
   gh release view v<version> --repo gubasso/spec-driven-docs     # release exists, installers attached
   git fetch origin && git rev-parse v<version> origin/master     # master sits on the tag
   sdd --version                                                  # installed binary reports it
   ```
