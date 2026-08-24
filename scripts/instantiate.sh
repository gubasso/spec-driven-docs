#!/usr/bin/env sh
set -eu

usage() {
  echo 'usage: instantiate.sh --target <absolute-path> --profile codebase|knowledge-base [--canon-ref vX.Y.Z] [--apply] [--dry-run]' >&2
  exit 2
}
target=
profile=
canon_ref=pre-release
dry=0
apply=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --target)
      [ "$#" -ge 2 ] || usage
      target=$2
      shift 2
      ;;
    --profile)
      [ "$#" -ge 2 ] || usage
      profile=$2
      shift 2
      ;;
    --canon-ref)
      [ "$#" -ge 2 ] || usage
      canon_ref=$2
      shift 2
      ;;
    --apply)
      apply=1
      shift
      ;;
    --dry-run)
      dry=1
      shift
      ;;
    *) usage ;;
  esac
done
[ -n "$target" ] && [ -n "$profile" ] || usage
case "$target" in /*) ;; *)
  echo 'FAIL target must be absolute' >&2
  exit 1
  ;;
esac
[ -d "$target" ] || {
  echo "FAIL unresolved target: $target" >&2
  exit 1
}
target=$(cd "$target" && pwd -P)
# `pwd -P` is permitted to print `//`, so the guard tests what the path is made
# of rather than comparing it to one spelling of the root.
[ -n "$(printf '%s' "$target" | tr -d /)" ] || {
  echo 'FAIL refusing root target'
  exit 1
}
canon=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd -P)
case "$target/" in "$canon"/*)
  echo 'FAIL target is inside the canon checkout'
  exit 1
  ;;
esac
profile_file="$canon/instance/profiles/$profile.json"
[ -f "$profile_file" ] || {
  echo "FAIL unknown profile: $profile"
  exit 1
}
# Whether the target already holds content, tested with a glob rather than
# `find -mindepth -maxdepth -quit`: none of those three is POSIX, and this
# script is the entry point a consumer runs on whatever userland they have.
target_has_content() {
  for entry in "$target"/* "$target"/.[!.]* "$target"/..?*; do
    # An unmatched glob stays literal and satisfies neither test. A dangling
    # symlink satisfies only `-L`, and it is an entry: a target holding one is
    # not empty, and treating it as empty skips the preview.
    { [ -e "$entry" ] || [ -L "$entry" ]; } || continue
    [ "${entry##*/}" = .git ] && continue
    return 0
  done
  return 1
}
forced_dry=0
if [ "$apply" -eq 0 ] && target_has_content && [ ! -f "$target/.spec-driven-docs/manifest.json" ]; then
  dry=1
  forced_dry=1
fi

docs_root=$(jq -r .docs_root "$profile_file")
stage=$(mktemp -d "${TMPDIR:-/tmp}/sdd-install.XXXXXX")
trap 'rm -rf "$stage"' EXIT HUP INT TERM
mkdir -p "$stage/root/.spec-driven-docs/hooks" "$stage/root/.spec-driven-docs/markdownlint" "$stage/root/$docs_root/specs" "$stage/root/$docs_root/decisions" "$stage/root/$docs_root/reference"

managed="$stage/managed.jsonl"
adopted="$stage/adopted.jsonl"
: >"$managed"
: >"$adopted"
copy_managed() {
  source=$1 destination=$2
  mkdir -p "$stage/root/$(dirname "$destination")"
  cp "$canon/$source" "$stage/root/$destination"
  hash=$(sha256sum "$stage/root/$destination" | cut -d' ' -f1)
  jq -nc --arg source "$source" --arg destination "$destination" --arg sha256 "$hash" '{source:$source,destination:$destination,sha256:$sha256}' >>"$managed"
  echo "$destination"
}
jq -r '.managed[] | @tsv' "$profile_file" | while IFS="$(printf '\t')" read -r source destination; do copy_managed "$source" "$destination"; done
for source in "$canon"/.hooks/*; do
  [ -f "$source" ] || continue
  copy_managed ".hooks/${source##*/}" ".spec-driven-docs/hooks/${source##*/}"
done

# An adopted file is the instance's own copy from the moment it lands, so its
# `sha256` is the installed bytes and `baseline_sha256` is what upstream shipped.
# Recording the upstream hash as the file's own hash makes the baseline
# permanently unmatchable, and turns the reconciliation signal into noise.
jq -r '.adopted[] | @tsv' "$profile_file" | while IFS="$(printf '\t')" read -r source destination; do
  destination=$(printf '%s' "$destination" | sed "s/{docs_root}/$docs_root/g")
  mkdir -p "$stage/root/$(dirname "$destination")"
  if [ -f "$target/$destination" ]; then cp "$target/$destination" "$stage/root/$destination"; else cp "$canon/$source" "$stage/root/$destination"; fi
  hash=$(sha256sum "$stage/root/$destination" | cut -d' ' -f1)
  baseline=$(sha256sum "$canon/$source" | cut -d' ' -f1)
  jq -nc --arg source "$source" --arg destination "$destination" --arg sha256 "$hash" --arg baseline "$baseline" '{source:$source,destination:$destination,sha256:$sha256,baseline_sha256:$baseline}' >>"$adopted"
  echo "$destination"
done

# The managed block is spliced into the `repos:` sequence rather than appended
# at end of file. Appended, it lands after whatever top-level key follows
# `repos:` -- `ci:` and `default_language_version:` are both ordinary -- and the
# consumer's configuration stops parsing as YAML.
precommit="$stage/root/.pre-commit-config.yaml"
if [ -f "$target/.pre-commit-config.yaml" ]; then cp "$target/.pre-commit-config.yaml" "$precommit"; else printf 'repos:\n' >"$precommit"; fi
# The markers are checked before anything is stripped. A lone BEGIN makes the
# strip swallow every line after it, and the install then writes that truncated
# document back as the consumer's configuration.
begins=$(grep -c '^# BEGIN spec-driven-docs managed$' "$precommit" || true)
ends=$(grep -c '^# END spec-driven-docs managed$' "$precommit" || true)
if [ "$begins" -ne "$ends" ] || [ "$begins" -gt 1 ]; then
  echo 'FAIL malformed managed markers in .pre-commit-config.yaml; repair them and re-run'
  exit 1
fi
if [ "$begins" -eq 1 ]; then
  first_begin=$(grep -n '^# BEGIN spec-driven-docs managed$' "$precommit" | head -1 | cut -d: -f1)
  first_end=$(grep -n '^# END spec-driven-docs managed$' "$precommit" | head -1 | cut -d: -f1)
  [ "$first_begin" -lt "$first_end" ] || {
    echo 'FAIL managed markers are out of order in .pre-commit-config.yaml'
    exit 1
  }
fi
awk '/^# BEGIN spec-driven-docs managed$/{skip=1;next}/^# END spec-driven-docs managed$/{skip=0;next}!skip{print}' "$precommit" >"$stage/precommit.base"
base="$stage/precommit.base"
repos_line=$(grep -n '^repos:[[:space:]]*$' "$base" | head -1 | cut -d: -f1)
[ -n "$repos_line" ] || {
  echo 'FAIL .pre-commit-config.yaml has no top-level repos: key; add one and re-run'
  exit 1
}
indent=$(awk -v s="$repos_line" 'NR>s && /^[[:space:]]*- / { match($0, /^[[:space:]]*/); print substr($0, 1, RLENGTH); exit }' "$base")
[ -n "$indent" ] || indent='  '
{
  printf '%s\n' '# BEGIN spec-driven-docs managed'
  printf '%s- repo: local\n' "$indent"
  printf '%s  hooks:\n' "$indent"
  printf '%s    - id: spec-driven-docs-verify\n' "$indent"
  printf '%s      name: verify spec-driven docs instance\n' "$indent"
  printf '%s      entry: .spec-driven-docs/verify.sh --target . --offline\n' "$indent"
  printf '%s      language: system\n' "$indent"
  printf '%s      always_run: true\n' "$indent"
  printf '%s      pass_filenames: false\n' "$indent"
  printf '%s\n' '# END spec-driven-docs managed'
} >"$stage/block"
end_line=$(awk -v s="$repos_line" 'NR>s && /^[A-Za-z_][A-Za-z0-9_-]*:/ { print NR; exit }' "$base")
if [ -n "$end_line" ]; then
  head -n "$((end_line - 1))" "$base" >"$precommit"
  cat "$stage/block" >>"$precommit"
  tail -n +"$end_line" "$base" >>"$precommit"
else
  cat "$base" "$stage/block" >"$precommit"
fi
marker_hash=$(sed -n '/^# BEGIN spec-driven-docs managed$/,/^# END spec-driven-docs managed$/p' "$precommit" | sha256sum | cut -d' ' -f1)

version=$(cat "$canon/VERSION")
if [ -f "$target/.spec-driven-docs/manifest.json" ]; then
  installed=$(jq -r .installed_at "$target/.spec-driven-docs/manifest.json")
else
  installed=$(date -u +%Y-%m-%dT%H:%M:%SZ)
fi
jq -s '.' "$managed" >"$stage/managed.json"
jq -s '.' "$adopted" >"$stage/adopted.json"
jq -n --arg version "$version" --arg ref "$canon_ref" --arg profile "$profile" --arg docs "$docs_root" --arg installed "$installed" --arg marker "$marker_hash" --slurpfile managed "$stage/managed.json" --slurpfile adopted "$stage/adopted.json" '{schema_version:1,canon_version:$version,canon_source:"https://github.com/gubasso/spec-driven-docs",canon_ref:$ref,profile:$profile,docs_root:$docs,installed_at:$installed,managed_files:$managed[0],adopted_files:$adopted[0],integration_blocks:[{path:".pre-commit-config.yaml",marker_hash:$marker}]}' >"$stage/root/.spec-driven-docs/manifest.json"
echo '.pre-commit-config.yaml'
echo '.spec-driven-docs/manifest.json'
[ "$dry" -eq 0 ] || {
  [ "$forced_dry" -eq 0 ] || echo 'DRY RUN: the target is a non-empty repository with no instance; re-run with --apply to write these files'
  echo 'DRY RUN: no files written'
  exit 0
}

# The apply is per file with a rollback, and the manifest lands last. A bulk
# copy that dies part way leaves a manifest listing files that were never
# written, and nothing downstream can tell that state from a good install.
manifest_destination=.spec-driven-docs/manifest.json
(cd "$stage/root" && find . -type f | sed 's|^\./||' | LC_ALL=C sort) >"$stage/destinations"
: >"$stage/backed-up"
: >"$stage/created"

apply_file() {
  d=$1
  mkdir -p "$target/$(dirname "$d")" || return 1
  if [ -f "$target/$d" ]; then
    mkdir -p "$stage/backup/$(dirname "$d")" || return 1
    cp -p "$target/$d" "$stage/backup/$d" || return 1
    printf '%s\n' "$d" >>"$stage/backed-up"
  else
    printf '%s\n' "$d" >>"$stage/created"
  fi
  cp "$stage/root/$d" "$target/$d" || return 1
}

rollback() {
  while IFS= read -r d; do
    [ -f "$stage/backup/$d" ] && cp -p "$stage/backup/$d" "$target/$d"
  done <"$stage/backed-up"
  while IFS= read -r d; do
    rm -f "$target/$d"
  done <"$stage/created"
}

# Every destination is checked before the first write. A symlink anywhere on the
# path writes outside the target the caller named; a non-directory where a
# directory belongs, or anything but a regular file at the destination itself,
# makes `cp` and the rollback mean something other than what they say.
check_destination() {
  d=$1 prefix=$target rest=$1
  while [ "$rest" != "${rest%%/*}" ]; do
    prefix="$prefix/${rest%%/*}"
    rest=${rest#*/}
    [ ! -L "$prefix" ] || {
      echo "FAIL destination escapes the target through a symlink: $d"
      return 1
    }
    if [ -e "$prefix" ] && [ ! -d "$prefix" ]; then
      echo "FAIL a file blocks a directory the install needs: ${prefix#"$target"/}"
      return 1
    fi
  done
  [ ! -L "$target/$d" ] || {
    echo "FAIL destination escapes the target through a symlink: $d"
    return 1
  }
  if [ -e "$target/$d" ] && [ ! -f "$target/$d" ]; then
    echo "FAIL destination exists and is not a regular file: $d"
    return 1
  fi
}

check_destinations() {
  while IFS= read -r d; do
    check_destination "$d" || return 1
  done <"$stage/destinations"
}

apply_all() {
  check_destinations || return 1
  while IFS= read -r d; do
    [ "$d" = "$manifest_destination" ] && continue
    apply_file "$d" || return 1
  done <"$stage/destinations"
  chmod 755 "$target/.spec-driven-docs/verify.sh" || return 1
  for hook in "$target/.spec-driven-docs/hooks/"*; do
    [ -f "$hook" ] || continue
    chmod 755 "$hook" || return 1
  done
  apply_file "$manifest_destination" || return 1
  "$target/.spec-driven-docs/verify.sh" --target "$target" --offline || return 1
}

if ! apply_all; then
  rollback
  echo 'FAIL apply aborted; the target was restored'
  exit 1
fi
