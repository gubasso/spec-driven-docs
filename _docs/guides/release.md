# Release

`Cargo.toml` is the source of truth. release-plz reads Conventional Commits to choose the bump,
maintains `CHANGELOG.md`, creates the `v<version>` tag when the release pull request merges, and
publishes to crates.io over trusted publishing. Nothing derives the other way, and nobody authors
a tag by hand.

## Cut a release

1. Confirm the tree is green and merge the work into `develop` with Conventional Commit messages.
   release-plz reads them: a `feat:` proposes a minor bump, a `fix:` a patch.

   ```bash
   just check
   ```

2. Write the migration guide leading into the proposed version. Required from the second release
   onward, because `sdd upgrade` walks guides and an instance cannot step into a version that has
   none. The canon test suite refuses a release commit without exactly one guide into the current
   version.

   ```bash
   cp migrations/TEMPLATE-migration.md migrations/<previous>-to-<version>.md
   ```

3. Regenerate the canon manifest so it carries the new version, and let release-plz open its
   release pull request against `develop`.

   ```bash
   just manifest
   ```

4. Merge the release pull request. This is the one human release decision: release-plz then tags
   `v<version>`, publishes the crate over OIDC, cargo-dist builds the installers, and the promote
   job fast-forwards `master` onto the tag.

5. Verify the release end to end: the crates.io version, the GitHub release's installers, and
   `sdd --version` from a fresh install.

## Never move a published tag

`pre-commit` caches a repository by its `rev`, and cargo caches a crate by its version, so a moved
tag or a re-published version serves the old payload to consumers who already resolved it and the
new payload to everyone else, with no error on either side. A release that shipped wrong is
corrected by the next version, never by retagging; `cargo yank` is a speed bump, not a recall.

The `release-tags` ruleset over `refs/tags/v*` blocks updates and deletions at the forge so this
cannot happen by accident.

## What checks what

| Check                      | Runs                | Holds                                                            |
| -------------------------- | ------------------- | ---------------------------------------------------------------- |
| `cargo-test` (canon tests) | every commit        | the manifest carries the crate version and a guide leads into it |
| release-plz                | on merge to develop | the bump, the changelog, the tag, and the publish                |
| `release-tags` ruleset     | at the forge        | no `v*` tag is ever moved or deleted                             |
| promote job                | after a release     | `master` fast-forwards to the tag, never past it                 |

## First-time setup

The trusted publisher, the GitHub App, and the branch rulesets are registered once; the sequence
lives with the workflows under `.github/workflows/` and follows the release-plz documentation:
first publish manually with a `publish-new`-scoped token, register `release-plz.yml` as the
crates.io trusted publisher, then revoke the token.
