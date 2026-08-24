# Release

`VERSION` is the source of truth. The tag, the instance manifest, and every consumer's pin derive
from it, and nothing derives the other way.

## Cut a release

1. Confirm the tree is green and clean.

   ```bash
   just check
   git status --porcelain
   ```

2. Write the migration guide leading into this version. Required from the second release onward,
   because `upgrade.sh` walks guides and an instance cannot step into a version that has none.

   ```bash
   cp migrations/TEMPLATE-migration.md migrations/<previous>-to-<version>.md
   ```

3. Preview the tag. This runs every precondition and creates nothing.

   ```bash
   just release
   ```

4. Create the tag.

   ```bash
   just release-tag
   ```

5. Push the branch first, then the tag. A tag that names a commit no branch reaches is a tag nobody
   can fetch.

   ```bash
   git push origin develop
   git push origin v<version>
   ```

6. Raise `VERSION` for the next release and align the instance manifest.

   ```bash
   printf '<next>\n' > VERSION
   just manifest
   ```

   The version gate refuses any further commit until this lands: with `v<version>` tagged, a change
   under that same number could only ship by moving the tag.

## Never move a published tag

`pre-commit` caches a repository by its `rev`, so a moved tag serves the old payload to consumers who
already resolved it and the new payload to everyone else, with no error on either side. A release
that shipped wrong is corrected by the next version, never by retagging.

Protect the tag namespace at the forge so this cannot happen by accident: on GitHub, a ruleset over
`refs/tags/v*` that blocks updates and deletions.

## What checks what

| Check                     | Runs                | Holds                                                                            |
| ------------------------- | ------------------- | -------------------------------------------------------------------------------- |
| `version-source-of-truth` | every commit        | `VERSION` is semantic, the instance manifest agrees, and the version is untagged |
| `release.sh`              | `just release`      | the tree is clean and the migration guide into this version exists               |
| `release.sh --verify`     | CI, on a pushed tag | the tag name is `v` plus `VERSION` at the commit it points at                    |
| `just test-release`       | `just test`         | each refusal above, against a throwaway clone                                    |
