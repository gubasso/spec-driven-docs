#!/usr/bin/env sh
# A spec stays within 300 authored lines and carries a TOC above 100.
#
# The generated TOC is excluded from the count: it grows with the requirement
# list and would otherwise spend the author's budget on navigation. Excluding it
# means trusting its delimiters, so the pair is checked first — a spec carrying
# one marker and nothing to close it would have every line after that marker
# deleted from the count, which is the over-budget file the cap exists to
# reject.
set -eu

# shellcheck source-path=SCRIPTDIR
# shellcheck disable=SC1091
# shellcheck source=lib-instance-paths.sh
. "$(dirname "$0")/lib-instance-paths.sh"

set -- "$(sdd_docs_root)"/specs/SPEC-?*.md
[ -e "$1" ] || {
  echo "FAIL no specs matched; the layout moved"
  exit 1
}

for f; do
  markers=$(grep -c '^<!--TOC-->$' "$f") || markers=0
  case "$markers" in
    0 | 2) ;;
    *)
      echo "FAIL docs-specs:spec-stays-within-300-lines $f: $markers TOC markers, expected 0 or 2"
      exit 1
      ;;
  esac

  n=$(sed '/^<!--TOC-->$/,/^<!--TOC-->$/d' "$f" | wc -l)
  [ "$n" -le 300 ] || {
    echo "FAIL docs-specs:spec-stays-within-300-lines $f: $n authored lines, cap is 300"
    exit 1
  }
  if [ "$n" -gt 100 ] && [ "$markers" -eq 0 ]; then
    echo "FAIL docs-specs:spec-stays-within-300-lines $f: over 100 lines with no TOC"
    exit 1
  fi
done
