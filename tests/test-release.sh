#!/usr/bin/env sh
# The release script derives the tag, and refuses every way of not deriving it.
#
# Each control runs against a throwaway clone rather than this checkout: the
# script reads the repository it stands in, and a control that tagged here would
# leave a tag behind that the version gate then rejects every later commit
# against.
set -eu
canon=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/sdd-release.XXXXXX")
scratch=$(cd "$scratch" && pwd -P)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

fail() {
  echo "FAIL $1"
  exit 1
}

# A rejection is asserted on the status and on the rule the message names, so a
# script that aborts for an unrelated reason cannot pass for a refusal.
reject() {
  name=$1 expected=$2
  shift 2
  if out=$("$@" 2>&1); then
    fail "$name exited zero"
  fi
  case "$out" in
    *"$expected"*) ;;
    *) fail "$name did not report '$expected': $out" ;;
  esac
}

clone() {
  dest=$1
  cp -R "$canon" "$dest"
  rm -rf "$dest/.git"
  git -C "$dest" init -q
  # An annotated tag needs a committer, and this machine's identity is not the
  # control's to assume.
  git -C "$dest" config user.email control@example.invalid
  git -C "$dest" config user.name control
  git -C "$dest" add -A
  git -C "$dest" commit -q -m 'control baseline'
}

version=$(cat "$canon/VERSION")

first="$scratch/first"
clone "$first"
"$first/scripts/release.sh" | grep -q "would create v$version" ||
  fail 'preview did not name the derived tag'
reject 'tag name drift' 'distribution:a-tag-derives-from-the-version-file' \
  "$first/scripts/release.sh" --verify "v9.9.9"
"$first/scripts/release.sh" --verify "v$version" >/dev/null ||
  fail 'verify rejected the derived tag'

printf '%s\n' 'local edit' >>"$first/README.md"
reject 'dirty tree' 'distribution:a-tag-derives-from-the-version-file' \
  "$first/scripts/release.sh" --tag
git -C "$first" checkout -q -- README.md

"$first/scripts/release.sh" --tag | grep -q "created v$version" ||
  fail 'tag was not created'
git -C "$first" rev-parse -q --verify "refs/tags/v$version" >/dev/null ||
  fail 'the tag the script reported does not exist'

# Standing on the release commit is not re-authoring it, so the gate lets the
# tagged tree alone.
sh -c "cd '$first' && gates/canon/version-source-of-truth.sh" ||
  fail 'the gate refused the tree the tag points at'

# The tag is what forces the bump: staging the next commit under the released
# version is the moment the gate refuses.
printf '%s\n' 'next change' >>"$first/README.md"
git -C "$first" add README.md
reject 'released version re-authored' 'distribution:a-released-version-is-not-re-authored' \
  sh -c "cd '$first' && gates/canon/version-source-of-truth.sh"

# A second release needs the guide that steps into it, because the upgrade walks
# guides and has no other way in.
next="$scratch/next"
clone "$next"
git -C "$next" tag "v$version"
printf '0.99.0\n' >"$next/VERSION"
sed -i "s/\"canon_version\": \"$version\"/\"canon_version\": \"0.99.0\"/" \
  "$next/.spec-driven-docs/manifest.json"
git -C "$next" add -A
git -C "$next" commit -q -m 'bump'
reject 'missing migration guide' 'distribution:a-release-carries-its-migration-guide' \
  "$next/scripts/release.sh"
printf '%s\n' "# Migration from $version to 0.99.0" >"$next/migrations/$version-to-0.99.0.md"
git -C "$next" add -A
git -C "$next" commit -q -m 'guide'
"$next/scripts/release.sh" | grep -q 'would create v0.99.0' ||
  fail 'a release carrying its guide was refused'

echo 'OK release controls'
