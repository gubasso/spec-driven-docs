#!/usr/bin/env sh
# `VERSION` states the release identity, and every other version derives from it.
#
# Two artifacts name a version and are read by different consumers: `VERSION`
# drives the upgrade path, and the instance manifest is what an installed target
# reports. Nothing reconciles them at read time, so they are reconciled here, in
# one direction -- a mismatch names `VERSION` as the value to fix rather than
# reporting a disagreement.
#
# `--files-only` drops the tag check for the one caller that runs at a commit
# the tag already points at.
set -eu

files_only=0
[ "${1:-}" = "--files-only" ] && files_only=1

[ -f VERSION ] || {
  echo 'FAIL distribution:versions-are-semantic-and-aligned VERSION: missing'
  exit 1
}
version=$(cat VERSION)

echo "$version" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$' || {
  echo "FAIL distribution:versions-are-semantic-and-aligned VERSION: '$version' is not a semantic version"
  exit 1
}

jq -e --arg v "$version" '.canon_version == $v' .spec-driven-docs/manifest.json >/dev/null || {
  echo "FAIL distribution:versions-are-semantic-and-aligned .spec-driven-docs/manifest.json: expected canon_version $version"
  exit 1
}

# A published tag is the one input this repository does not own: it is already
# in someone's `rev:`. Continuing to author under a version that is tagged
# leaves that consumer pinned to content this tree no longer holds, so the tag
# is what forces the bump. A checkout with no tags fetched reports none, which
# is why the release path re-runs this where the tags are complete.
[ "$files_only" -eq 1 ] && exit 0
command -v git >/dev/null 2>&1 || exit 0
git rev-parse --git-dir >/dev/null 2>&1 || exit 0
git rev-parse -q --verify "refs/tags/v$version" >/dev/null 2>&1 || exit 0

echo "FAIL distribution:a-released-version-is-not-re-authored VERSION: v$version is already tagged; bump VERSION"
exit 1
