#!/usr/bin/env sh
set -eu

usage() {
  echo 'usage: verify.sh --target <absolute-path> [--offline] [--check-upstream <checkout>]' >&2
  exit 2
}
target=
upstream=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --target)
      [ "$#" -ge 2 ] || usage
      target=$2
      shift 2
      ;;
    --offline) shift ;;
    --check-upstream)
      [ "$#" -ge 2 ] || usage
      upstream=$2
      shift 2
      ;;
    *) usage ;;
  esac
done
[ -n "$target" ] || usage
case "$target" in /*) ;; .) target=$(pwd) ;; *)
  echo 'FAIL target must be absolute or .' >&2
  exit 1
  ;;
esac
[ -d "$target" ] || {
  echo "FAIL unresolved target: $target" >&2
  exit 1
}
target=$(cd "$target" && pwd -P)
manifest="$target/.spec-driven-docs/manifest.json"
[ -f "$manifest" ] || {
  echo "FAIL missing manifest: $manifest"
  exit 1
}

# awk, dirname, find, head, sed, tr and wc are listed because the installed
# hooks need them -- the size gates count with `wc`, the shared library reads the
# manifest with `head`, and every gate resolves that library with `dirname` --
# not because this script does. The check runs before the work directory is
# made, so `mktemp` and `rm` are named here rather than met as a
# command-not-found after the preflight said the environment was complete. cat, cut, grep, sort and uniq are listed because
# this script does, and a pipeline hides a missing one: `sh` has no pipefail, so
# `sort ... | uniq -d` with no `sort` reports the status of `uniq`, which
# succeeds over an empty stream and turns the duplicate-ID check into a green
# light.
for tool in awk cat cut dirname find grep head jq mktemp rm sed sha256sum sort tr uniq wc; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "FAIL missing required tool: $tool"
    exit 1
  }
done
jq -e '.schema_version == 1 and (.canon_version|test("^[0-9]+\\.[0-9]+\\.[0-9]+$")) and (.managed_files|type=="array") and (.managed_files|length > 0) and (.adopted_files|type=="array") and (.integration_blocks|type=="array")' "$manifest" >/dev/null || {
  echo 'FAIL invalid manifest schema'
  exit 1
}

work=$(mktemp -d "${TMPDIR:-/tmp}/sdd-verify.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM

# Each projection is materialised before it is read. `sh` has no pipefail, so a
# jq that dies mid-stream feeds an empty loop and the pipeline still exits 0 --
# a mistyped or truncated manifest would verify clean, which is the path a bad
# upgrade takes.
project() {
  jq -r "$1" "$manifest" >"$work/rows" || {
    echo 'FAIL unreadable manifest projection'
    exit 1
  }
}

project '.managed_files[] | [.destination,.sha256] | @tsv'
while IFS="$(printf '\t')" read -r destination expected; do
  file="$target/$destination"
  [ -f "$file" ] || {
    echo "FAIL missing managed file: $destination"
    exit 1
  }
  actual=$(sha256sum "$file" | cut -d' ' -f1)
  [ "$actual" = "$expected" ] || {
    echo "FAIL managed drift: $destination"
    exit 1
  }
done <"$work/rows"

project '.adopted_files[] | [.destination,.sha256] | @tsv'
while IFS="$(printf '\t')" read -r destination baseline; do
  file="$target/$destination"
  [ -f "$file" ] || {
    echo "FAIL missing adopted file: $destination"
    exit 1
  }
  actual=$(sha256sum "$file" | cut -d' ' -f1)
  [ "$actual" = "$baseline" ] || echo "DRIFT adopted file requires reconciliation: $destination"
done <"$work/rows"

block="$target/.pre-commit-config.yaml"
[ -f "$block" ] || {
  echo 'FAIL missing .pre-commit-config.yaml'
  exit 1
}
[ "$(grep -c '^# BEGIN spec-driven-docs managed$' "$block" || true)" -eq 1 ] || {
  echo 'FAIL missing managed pre-commit block'
  exit 1
}
[ "$(grep -c '^# END spec-driven-docs managed$' "$block" || true)" -eq 1 ] || {
  echo 'FAIL malformed managed pre-commit block'
  exit 1
}

# The recorded marker hash is what makes the integration self-defending. Storing
# it and never comparing it leaves the one region the canon owns editable with
# no signal, which is the tampering this check exists to see.
recorded=$(jq -r '.integration_blocks[] | select(.path==".pre-commit-config.yaml") | .marker_hash // empty' "$manifest")
[ -n "$recorded" ] || {
  echo 'FAIL manifest records no marker hash for .pre-commit-config.yaml'
  exit 1
}
present=$(sed -n '/^# BEGIN spec-driven-docs managed$/,/^# END spec-driven-docs managed$/p' "$block" | sha256sum | cut -d' ' -f1)
[ "$present" = "$recorded" ] || {
  echo 'FAIL managed block tampered: .pre-commit-config.yaml'
  exit 1
}

# Every entry in the block that names a path names an executable that is there.
# A half-written install records the block and leaves the verifier behind, and
# nothing else in this script would notice. An entry naming a command on PATH --
# `dprint check` -- is the tool's problem, not the instance's, so it is skipped.
sed -n '/^# BEGIN spec-driven-docs managed$/,/^# END spec-driven-docs managed$/p' "$block" |
  sed -n 's/^[[:space:]]*entry:[[:space:]]*//p' | cut -d' ' -f1 >"$work/entries"
entries=0
while IFS= read -r entry; do
  case "$entry" in ./* | /* | .[!/]*) ;; *) continue ;; esac
  entries=$((entries + 1))
  [ -x "$target/$entry" ] || {
    echo "FAIL managed block entry is not executable: $entry"
    exit 1
  }
done <"$work/entries"
[ "$entries" -gt 0 ] || {
  echo 'FAIL managed block names no executable entry'
  exit 1
}

docs_root=$(jq -r .docs_root "$manifest")
[ -d "$target/$docs_root/specs" ] || {
  echo "FAIL missing local specs: $docs_root/specs"
  exit 1
}

# Every rule ID the local specs state is a slug pair and resolves to exactly one
# requirement. A duplicated ID stops being an address, and the citation in a
# commit or a suppression then names two rules at once.
# shellcheck disable=SC2016
grep -rhoE '^### `[a-z0-9-]+:[a-z0-9-]+`' "$target/$docs_root/specs" >"$work/ids" || :
sort "$work/ids" >"$work/sorted-ids" || {
  echo 'FAIL unreadable rule ID list'
  exit 1
}
duplicated=$(uniq -d "$work/sorted-ids")
[ -z "$duplicated" ] || {
  echo 'FAIL duplicate rule ID in local specs'
  echo "$duplicated"
  exit 1
}

if [ -n "$upstream" ]; then
  [ -f "$upstream/VERSION" ] || {
    echo 'FAIL upstream checkout has no VERSION'
    exit 1
  }
  echo "installed $(jq -r .canon_version "$manifest"), upstream $(cat "$upstream/VERSION")"
fi
echo "OK spec-driven-docs $(jq -r .canon_version "$manifest") at $target"
