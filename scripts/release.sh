#!/usr/bin/env sh
# Derive the release tag from `VERSION`. This is the only thing that derives it.
#
# A tag is an alias a consumer pins in a `rev:`, and git will happily move one.
# Authoring the tag by hand is what lets it name a version the tree does not
# hold, so the name is computed here and the preview runs by default -- `--tag`
# is the explicit step, the shape `instantiate.sh` uses for the same reason.
#
# Pushing is deliberately not here. A local tag is reversible; a pushed one is
# what someone else's `rev:` resolves against.
set -eu

usage() {
  echo 'usage: release.sh [--tag] [--verify <tag>]' >&2
  exit 2
}

mode=preview
claimed=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --tag)
      mode=tag
      shift
      ;;
    --verify)
      [ "$#" -ge 2 ] || usage
      mode=verify
      claimed=$2
      shift 2
      ;;
    *) usage ;;
  esac
done

root=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd -P)
cd "$root"
version=$(cat VERSION)
tag="v$version"

if [ "$mode" = verify ]; then
  gates/canon/version-source-of-truth.sh --files-only
  [ "$claimed" = "$tag" ] || {
    echo "FAIL release:a-tag-derives-from-the-version-file $claimed: VERSION says $version, so the tag is $tag"
    exit 1
  }
  echo "OK $tag derives from VERSION"
  exit 0
fi

gates/canon/version-source-of-truth.sh

git rev-parse --git-dir >/dev/null 2>&1 || {
  echo 'FAIL release:a-tag-derives-from-the-version-file: not a git checkout'
  exit 1
}

[ -z "$(git status --porcelain)" ] || {
  echo "FAIL release:a-tag-derives-from-the-version-file: the working tree is dirty, so $tag would not name what it points at"
  exit 1
}

# An upgrade steps guide by guide, so a release nobody can step into is a
# release nobody can take. The guide leading into this version is required as
# soon as any release exists to step from; the first release has none.
if [ -n "$(git tag -l 'v*')" ]; then
  guide=
  for candidate in "migrations/"*-to-"$version.md"; do
    [ -f "$candidate" ] || continue
    [ -z "$guide" ] || {
      echo "FAIL release:a-release-carries-its-migration-guide: more than one guide leads into $version"
      exit 1
    }
    guide=$candidate
  done
  [ -n "$guide" ] || {
    echo "FAIL release:a-release-carries-its-migration-guide: no migrations/<previous>-to-$version.md"
    exit 1
  }
  echo "migration guide: ${guide##*/}"
fi

[ "$mode" = tag ] || {
  echo "OK would create $tag at $(git rev-parse --short HEAD)"
  exit 0
}

git tag -a "$tag" -m "spec-driven-docs $version"
echo "OK created $tag at $(git rev-parse --short HEAD)"
echo "push it with: git push origin $tag"
