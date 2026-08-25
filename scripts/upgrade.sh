#!/usr/bin/env sh
set -eu

usage() {
  echo 'usage: upgrade.sh --target <absolute-path> --from <canon-checkout> [--dry-run]' >&2
  exit 2
}
target=
from=
dry=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --target)
      [ "$#" -ge 2 ] || usage
      target=$2
      shift 2
      ;;
    --from)
      [ "$#" -ge 2 ] || usage
      from=$2
      shift 2
      ;;
    --dry-run)
      dry=1
      shift
      ;;
    *) usage ;; esac
done
[ -n "$target" ] && [ -n "$from" ] || usage
case "$target:$from" in /*:/*) ;; *)
  echo 'FAIL paths must be absolute'
  exit 1
  ;;
esac
[ -d "$target" ] && [ -d "$from" ] || {
  echo 'FAIL unresolved path'
  exit 1
}
manifest="$target/.spec-driven-docs/manifest.json"
[ -f "$manifest" ] && [ -f "$from/VERSION" ] || {
  echo 'FAIL missing manifest or VERSION'
  exit 1
}
old=$(jq -r .canon_version "$manifest") new=$(cat "$from/VERSION")
[ "$old" != "$new" ] || {
  echo "OK already at $new"
  exit 0
}

stage=$(mktemp -d "${TMPDIR:-/tmp}/sdd-upgrade.XXXXXX")
trap 'rm -rf "$stage"' EXIT HUP INT TERM
status=0

# Each guide is announced as it is discovered. Accumulating them in one string
# and splitting it afterwards turns any path containing a space into several
# guides that do not exist, and the migration list is the only safety
# instruction the operator gets before a hard-to-reverse change.
current=$old
while [ "$current" != "$new" ]; do
  # A glob, not `find -maxdepth ... | sort -V`: neither `-maxdepth` nor `sort -V`
  # is POSIX, and version sorting is not needed once the step out of a version is
  # required to be unambiguous.
  guide=
  for candidate in "$from/migrations/$current"-to-*.md; do
    [ -f "$candidate" ] || continue
    [ -z "$guide" ] || {
      echo "FAIL ambiguous migration path from $current: more than one guide"
      exit 1
    }
    guide=$candidate
  done
  [ -n "$guide" ] || {
    echo "FAIL no migration guide from $current to $new"
    exit 1
  }
  echo "consult migration: ${guide##*/}"
  next=${guide##*-to-}
  current=${next%.md}
done

# Conflicts are collected, then reported together. The operator fixing one and
# rerunning to find the next is the shape this avoids, and a file the loop
# writes to survives the pipeline's subshell where a variable does not.
: >"$stage/conflicts"
jq -r '.managed_files[] | [.destination,.sha256] | @tsv' "$manifest" >"$stage/managed" || {
  echo 'FAIL unreadable manifest projection'
  exit 1
}
# The scan asks one question: has the operator edited a file this instance does
# not own? The recorded hash answers it against the copy on disk, and where
# upstream keeps its own copy has no bearing on the answer. Requiring the old
# source path to still exist made every relocation upstream report every managed
# file as a conflict, which is an upgrade nobody can take and a payload that can
# never be reorganized.
while IFS="$(printf '\t')" read -r destination recorded; do
  [ -f "$target/$destination" ] || {
    echo "CONFLICT missing managed file: $destination" >>"$stage/conflicts"
    continue
  }
  current=$(sha256sum "$target/$destination" | cut -d' ' -f1)
  [ "$current" = "$recorded" ] || echo "CONFLICT locally edited managed file: $destination" >>"$stage/conflicts"
done <"$stage/managed"
[ ! -s "$stage/conflicts" ] || {
  cat "$stage/conflicts"
  exit 1
}

# The instance verifies only after the conflict scan. Verifying first turns a
# locally edited managed file into an opaque `FAIL managed drift`, when the
# conflict list is the report the operator can act on.
"$target/.spec-driven-docs/verify.sh" --target "$target" --offline >"$stage/verify" || {
  cat "$stage/verify"
  exit 1
}
[ "$dry" -eq 0 ] || {
  echo "DRY RUN upgrade $old to $new"
  exit 0
}

# The reinstall is the apply, and it stages every file it writes and rolls the
# target back on any failure. Copying the whole target through a temporary
# directory instead rewrites `.git` -- loose objects are mode 444, so the copy
# fails part way and leaves the target half upgraded -- and puts the remote URLs
# in `.git/config` somewhere the operator did not choose.
profile=$(jq -r .profile "$manifest")
ref="v$new"
# What this instance owns right now, captured before the reinstall overwrites
# the manifest that records it. A file dropped from the payload between the two
# versions is only identifiable as the difference between these two lists.
cut -f1 "$stage/managed" | sort >"$stage/old-destinations"
# The installer's output is held rather than discarded: on success it is the
# list of proposed paths, which the operator did not ask for, and on failure it
# is the only account of what stopped the upgrade.
"$from/scripts/instantiate.sh" --target "$target" --profile "$profile" --canon-ref "$ref" --apply >"$stage/install" 2>&1 || {
  cat "$stage/install" >&2
  echo "FAIL upgrade aborted during reinstall from $old to $new" >&2
  exit 1
}
"$target/.spec-driven-docs/verify.sh" --target "$target" --offline >/dev/null

# A file the new version stopped managing is removed, not left behind. The
# conflict scan already proved every one of them still matches the hash this
# instance recorded, so nothing an operator wrote is at risk. Left in place, a
# dropped gate keeps running from a payload that no longer claims it, and no
# check would ever mention it again.
#
# The prefix guard is what keeps this a payload operation. Only the vendored
# directory is ever pruned, so a manifest naming a destination outside it --
# through damage or through a hand edit -- cannot turn an upgrade into a
# deletion somewhere in the project.
jq -r '.managed_files[].destination' "$manifest" | sort >"$stage/new-destinations"
comm -23 "$stage/old-destinations" "$stage/new-destinations" >"$stage/dropped"
: >"$stage/unremoved"
while IFS= read -r destination; do
  [ -n "$destination" ] || continue
  # The prefix alone is not containment. `.spec-driven-docs/../../elsewhere`
  # satisfies it and resolves outside the target, so a manifest that was damaged
  # or hand-edited could aim this deletion anywhere the operator can write. A
  # traversal segment or an absolute path is refused outright rather than
  # normalized, because there is no legitimate managed destination that needs one.
  case "$destination" in
    /* | */../* | ../* | */.. | ..)
      echo "refused to remove a destination that leaves the target: $destination"
      status=1
      continue
      ;;
  esac
  case "$destination" in
    .spec-driven-docs/*) ;;
    *) continue ;;
  esac
  # A lexical guard is not containment either. `rm` resolves the path it is
  # given, so a symlink anywhere along it -- planted in the vendored directory,
  # or left by an earlier install -- makes this unlink a file the operator never
  # named. Every component below the target is checked, and the destination
  # itself must be a regular file rather than a link to one.
  escapes=0
  prefix=$target rest=$destination
  while [ "$rest" != "${rest%%/*}" ]; do
    prefix="$prefix/${rest%%/*}"
    rest=${rest#*/}
    [ ! -L "$prefix" ] || escapes=1
  done
  if [ "$escapes" -eq 1 ] || [ -L "$target/$destination" ]; then
    echo "refused to remove a destination reached through a symlink: $destination"
    status=1
    continue
  fi
  [ -f "$target/$destination" ] || continue
  # A failure here is collected rather than fatal. The reinstall has already
  # committed the new manifest, so aborting now would leave an instance that
  # reports the new version and never prunes again -- the next run exits at
  # `already at`, with the old manifest that named this file gone. Reporting
  # every one that survived leaves the operator an exact, finishable remainder.
  if rm -f "$target/$destination" 2>/dev/null; then
    echo "removed managed file no longer owned: $destination"
  else
    printf '%s\n' "$destination" >>"$stage/unremoved"
  fi
done <"$stage/dropped"
if [ -s "$stage/unremoved" ]; then
  echo "FAIL these files are no longer owned and could not be removed; delete them by hand:"
  sed 's|^|  |' "$stage/unremoved"
  echo "the payload and manifest are upgraded to $new; only these removals remain, and"
  echo "re-running this upgrade will report 'already at $new' rather than retry them"
  status=1
fi

# The rule IDs that changed upstream are what an operator reconciles by hand, so
# the diff is reported rather than computed and dropped.
docs_root=$(jq -r .docs_root "$manifest")
# shellcheck disable=SC2016
grep -rhoE '^### `[a-z0-9-]+:[a-z0-9-]+`' "$target/$docs_root/specs" 2>/dev/null | sort -u >"$stage/local-ids" || :
# shellcheck disable=SC2016
grep -rhoE '^### `[a-z0-9-]+:[a-z0-9-]+`' "$from/_docs/specs" 2>/dev/null | sort -u >"$stage/upstream-ids" || :
comm -13 "$stage/local-ids" "$stage/upstream-ids" >"$stage/new-ids" || :
if [ -s "$stage/new-ids" ]; then
  echo 'upstream rule IDs not present locally:'
  tr -d '`' <"$stage/new-ids" | sed 's/^### /  /'
fi
[ "$status" -eq 0 ] || {
  echo "FAIL upgraded $old to $new with unfinished removals above"
  exit "$status"
}
echo "OK upgraded $old to $new"
