#!/usr/bin/env bash
# Vendor the SimpleEnglish dependency surface into third-party/simpleenglish/.
#
# This script is the only path that changes the vendored tree. It fetches one
# resolved Git object from the upstream repository, validates the declared
# surface, shows the diff against the destination, and writes only with
# --apply. The destination is replaced atomically and restored on failure.
#
# Usage:
#   scripts/vendor-simpleenglish.sh --release latest            preview the latest release
#   scripts/vendor-simpleenglish.sh --release v2.0.0            preview one release
#   scripts/vendor-simpleenglish.sh --revision <full-object-id> preview one commit
#   ... --apply                                                 write the result
#
# Exit codes:
#   0  the destination already holds the selected revision (no change)
#   3  preview: changes are pending; apply: changes were written
#   1  usage error
#   2  validation refused: symbolic revision, missing license, unexpected path,
#      incomplete reference set, or a dirty destination

set -euo pipefail

REPOSITORY="https://github.com/AminBlg/SimpleEnglish"
DESTINATION="third-party/simpleenglish"
RECORD="UPSTREAM.json"

# The declared surface. One line per file: <scope> <upstream path>.
# instance: an installed author or an offline command reads it, so the
#           binary projects it into every instance.
# canon:    a canon-side conformance oracle or an unchanged upstream hook
#           source, kept for comparison and never projected or executed.
SURFACE='
instance LICENSE
instance prompts/system-prompt.md
instance skills/simple-english/SKILL.md
instance skills/simple-english/references/checklist.md
instance skills/simple-english/references/strict-vocabulary.md
instance skills/simple-english/references/use-cases.md
instance skills/simple-english/references/word-swaps.md
canon hooks/hooks.json
canon src/hooks/README.md
canon src/hooks/package.json
canon src/hooks/simple-english-activate.js
canon src/hooks/simple-english-activate.test.js
canon src/hooks/lint_hook.py
canon src/hooks/test_lint_hook.py
canon evals/ste_lint.py
canon evals/slop.tsv
'

# Upstream surfaces the vendored tree leaves out, each with the reason.
# A path ending in / excludes a directory. Every upstream file must be
# declared above or excluded here; anything else is an unexpected path.
EXCLUDED='
.agents/plugins/marketplace.json|plugin manifest: the convention arrives through the instance integration, not a plugin install
.claude-plugin/marketplace.json|plugin manifest: the convention arrives through the instance integration, not a plugin install
.claude-plugin/plugin.json|plugin manifest: the convention arrives through the instance integration, not a plugin install
.codex-plugin/plugin.json|plugin manifest: the convention arrives through the instance integration, not a plugin install
.gitignore|repository administration: nothing here is executed or read
README.md|upstream marketing and repository routing: nothing here is executed or read
examples/before-after.md|upstream marketing example: nothing here is executed or read
output-styles/simple-english.md|harness-specific configuration: the instance integration replaces it
evals/pressure-tests.md|benchmark evidence: development corpus, not the runtime dependency surface
evals/reply_scenarios.json|benchmark input: development corpus, not the runtime dependency surface
evals/scenarios.json|benchmark input: development corpus, not the runtime dependency surface
evals/run_bench.py|benchmark runner: development tooling, not the runtime dependency surface
evals/run_pi_bench.py|benchmark runner: development tooling, not the runtime dependency surface
evals/run_reply_bench.py|benchmark runner: development tooling, not the runtime dependency surface
evals/score_text_dir.py|benchmark runner: development tooling, not the runtime dependency surface
evals/test_run_pi_bench.py|benchmark runner test: development tooling, not the runtime dependency surface
evals/results/|recorded benchmark results: development evidence, not the runtime dependency surface
package/|recorded benchmark results: development evidence, not the runtime dependency surface
'

usage() {
  sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
  exit 1
}

fail() {
  echo "vendor-simpleenglish: $*" >&2
  exit 2
}

need() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 is required and not on PATH"
}

release=""
revision=""
apply=0
while [ $# -gt 0 ]; do
  case "$1" in
    --release)
      [ $# -ge 2 ] || usage
      release="$2"
      shift 2
      ;;
    --revision)
      [ $# -ge 2 ] || usage
      revision="$2"
      shift 2
      ;;
    --apply)
      apply=1
      shift
      ;;
    -h | --help) usage ;;
    *) usage ;;
  esac
done
if [ -n "$release" ] && [ -n "$revision" ]; then usage; fi
if [ -z "$release" ] && [ -z "$revision" ]; then usage; fi

need git
need sha256sum
need jq
need diff

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

work="$(mktemp -d "${TMPDIR:-/tmp}/vendor-simpleenglish.XXXXXX")"
# shellcheck disable=SC2329  # invoked via trap
cleanup() { rm -rf "$work"; }
trap cleanup EXIT

is_object_id() {
  case "$1" in
    *[!0-9a-f]*) return 1 ;;
  esac
  [ "${#1}" -eq 40 ] || [ "${#1}" -eq 64 ]
}

# Resolve --release to a reference and a full object id.
reference=""
if [ -n "$release" ]; then
  tags="$(GIT_TERMINAL_PROMPT=0 git ls-remote --tags --refs "$REPOSITORY" 'refs/tags/v*')" ||
    fail "cannot list tags at $REPOSITORY"
  if [ "$release" = "latest" ]; then
    release="$(printf '%s\n' "$tags" | awk '{print $2}' | sed 's|refs/tags/||' | sort -V | tail -n 1)"
    [ -n "$release" ] || fail "no v* tag found at $REPOSITORY"
  fi
  reference="refs/tags/$release"
  tag_object="$(printf '%s\n' "$tags" | awk -v r="$reference" '$2 == r {print $1}')"
  [ -n "$tag_object" ] || fail "no tag $release at $REPOSITORY"
  # An annotated tag names a tag object; the pinned revision is the commit.
  peeled="$(GIT_TERMINAL_PROMPT=0 git ls-remote --tags "$REPOSITORY" "$reference^{}" | awk '{print $1}' | head -n 1)"
  revision="${peeled:-$tag_object}"
fi

is_object_id "$revision" || fail "the revision must be a full 40- or 64-character hexadecimal object id, not a branch or tag: $revision"

# Fetch exactly that object into a scratch repository.
git -C "$work" init -q
git -C "$work" remote add origin "$REPOSITORY"
GIT_TERMINAL_PROMPT=0 git -C "$work" fetch -q --depth 1 origin "$revision" ||
  fail "cannot fetch $revision from $REPOSITORY"
git -C "$work" checkout -q --detach FETCH_HEAD
fetched="$(git -C "$work" rev-parse HEAD)"
[ "$fetched" = "$revision" ] || fail "fetched $fetched, expected $revision"

# Validate the declared surface against the fetched tree.
staged="$work/staged"
mkdir -p "$staged"
declared_paths=""
while read -r scope path; do
  [ -n "$scope" ] || continue
  [ -f "$work/$path" ] || fail "declared path is missing upstream at $revision: $path"
  declared_paths="$declared_paths$path"$'\n'
  mkdir -p "$staged/$(dirname "$path")"
  cp "$work/$path" "$staged/$path"
done <<<"$SURFACE"

grep -q '^MIT License$' "$staged/LICENSE" || fail "upstream LICENSE is not the MIT license text"
grep -q '^Copyright (c)' "$staged/LICENSE" || fail "upstream LICENSE carries no copyright line"

# Every upstream file is declared or excluded; anything else is a new surface
# a person classifies before the next vendor run.
unexpected=0
while IFS= read -r file; do
  if printf '%s' "$declared_paths" | grep -qxF "$file"; then
    continue
  fi
  excluded=0
  while IFS='|' read -r pattern _reason; do
    [ -n "$pattern" ] || continue
    case "$pattern" in
      */) case "$file" in "$pattern"*) excluded=1 ;; esac ;;
      *) [ "$file" = "$pattern" ] && excluded=1 ;;
    esac
  done <<<"$EXCLUDED"
  if [ "$excluded" -eq 0 ]; then
    echo "vendor-simpleenglish: unexpected upstream path, neither declared nor excluded: $file" >&2
    unexpected=1
  fi
done < <(git -C "$work" ls-tree -r --name-only HEAD)
[ "$unexpected" -eq 0 ] || exit 2

# Every reference the skill names must be part of the surface.
while IFS= read -r ref; do
  [ -n "$ref" ] || continue
  [ -f "$staged/skills/simple-english/$ref" ] ||
    fail "the skill names $ref and the surface does not carry it"
done < <(grep -o 'references/[a-z-]*\.md' "$staged/skills/simple-english/SKILL.md" | sort -u)

# Write the record.
release_name="${release:-}"
{
  echo '{'
  echo '  "project": "SimpleEnglish",'
  echo "  \"repository\": \"$REPOSITORY\","
  if [ -n "$release_name" ]; then
    echo "  \"release\": \"$release_name\","
    echo "  \"reference\": \"$reference\","
  else
    echo '  "release": null,'
    echo '  "reference": null,'
  fi
  echo "  \"revision\": \"$revision\","
  echo '  "license": "MIT",'
  echo '  "digest_algorithm": "sha256",'
  echo '  "files": ['
  first=1
  while read -r scope path; do
    [ -n "$scope" ] || continue
    digest="$(sha256sum "$staged/$path" | awk '{print $1}')"
    [ "$first" -eq 1 ] || echo ','
    first=0
    printf '    { "path": "%s", "scope": "%s", "sha256": "%s" }' "$path" "$scope" "$digest"
  done <<<"$SURFACE"
  echo
  echo '  ],'
  echo '  "excluded": ['
  first=1
  while IFS='|' read -r pattern reason; do
    [ -n "$pattern" ] || continue
    [ "$first" -eq 1 ] || echo ','
    first=0
    printf '    { "path": "%s", "reason": "%s" }' "$pattern" "$reason"
  done <<<"$EXCLUDED"
  echo
  echo '  ]'
  echo '}'
} | jq . >"$staged/$RECORD"

# Dirty-destination check: the destination must hold exactly what its own
# record says, so a hand edit is refused rather than overwritten.
if [ -f "$DESTINATION/$RECORD" ]; then
  while IFS=$'\t' read -r path digest; do
    if [ ! -f "$DESTINATION/$path" ]; then
      fail "dirty destination: $DESTINATION/$path is recorded and missing; restore the tree with git before vendoring"
    fi
    actual="$(sha256sum "$DESTINATION/$path" | awk '{print $1}')"
    [ "$actual" = "$digest" ] ||
      fail "dirty destination: $DESTINATION/$path differs from its record; restore the tree with git before vendoring"
  done < <(jq -r '.files[] | [.path, .sha256] | @tsv' "$DESTINATION/$RECORD")
  while IFS= read -r file; do
    rel="${file#"$DESTINATION"/}"
    [ "$rel" = "$RECORD" ] && continue
    jq -e --arg p "$rel" '.files[] | select(.path == $p)' "$DESTINATION/$RECORD" >/dev/null ||
      fail "dirty destination: $file is not in the record; remove it before vendoring"
  done < <(find "$DESTINATION" -type f | sort)
elif [ -d "$DESTINATION" ] && [ -n "$(ls -A "$DESTINATION")" ]; then
  fail "dirty destination: $DESTINATION exists with no $RECORD"
fi

# Compare and report.
if [ -d "$DESTINATION" ]; then
  if diff -r -q "$DESTINATION" "$staged" >/dev/null 2>&1; then
    echo "OK $DESTINATION already holds $REPOSITORY at $revision${release_name:+ ($release_name)}"
    exit 0
  fi
  echo "upstream-to-vendored diff ($DESTINATION against $revision):"
  diff -r -u "$DESTINATION" "$staged" || true
else
  echo "new vendored tree from $REPOSITORY at $revision:"
  (cd "$staged" && find . -type f | sort | sed 's|^\./|  |')
fi

if [ "$apply" -eq 0 ]; then
  echo "PREVIEW: no files written; re-run with --apply to replace $DESTINATION"
  exit 3
fi

# Apply atomically on the destination's filesystem: stage beside it, swap,
# and put the old tree back if anything fails.
parent="$(dirname "$DESTINATION")"
mkdir -p "$parent"
incoming="$(mktemp -d "$parent/.simpleenglish.incoming.XXXXXX")"
backup="$parent/.simpleenglish.previous.$$"
# shellcheck disable=SC2329  # invoked via trap
restore() {
  rm -rf "$incoming"
  if [ -d "$backup" ]; then
    rm -rf "$DESTINATION"
    mv "$backup" "$DESTINATION"
  fi
}
trap 'restore; cleanup' EXIT
cp -R "$staged/." "$incoming/"
if [ -d "$DESTINATION" ]; then
  mv "$DESTINATION" "$backup"
fi
mv "$incoming" "$DESTINATION"
rm -rf "$backup"
trap cleanup EXIT
echo "OK wrote $DESTINATION from $REPOSITORY at $revision${release_name:+ ($release_name)}"
exit 3
