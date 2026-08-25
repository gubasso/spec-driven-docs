#!/usr/bin/env sh
# A chapter stays within its line cap, and the debt list shrinks by itself.
#
# The debt list is read with `|| [ -n "$file" ]` because a file whose last line
# carries no newline would otherwise keep its exemption -- pass 2's `grep -Fxq`
# sees that line -- while skipping both checks that expire it, making the entry
# permanently un-expirable.
#
# Both passes compare `./`-prefixed paths, because the walk emits `./README.md`
# and a debt list is written repo-relative; comparing the two shapes directly
# exempts nothing and caps a root file at two different numbers.
#
# The walk prunes the directories a consumer vendors rather than authors. A gate
# a consumer cannot satisfy short of deleting `node_modules/` is a gate they
# turn off, and it takes the rest of the payload with it.
set -eu
debt=.spec-driven-docs/chapter-size-debt.txt

cap_for() {
  case "$1" in
    *-gates.md | *-checklist.md | *glossary.md | *README.md) echo 300 ;;
    *) echo 200 ;;
  esac
}

if [ -f "$debt" ]; then
  while IFS= read -r file || [ -n "$file" ]; do
    case "$file" in '' | '#'*) continue ;; esac
    case "$file" in ./*) ;; *) file="./$file" ;; esac
    [ -f "$file" ] || {
      echo "FAIL docs-format:chapter-stays-within-200-lines delist $file: deleted"
      exit 1
    }
    cap=$(cap_for "$file")
    lines=$(wc -l <"$file")
    [ "$lines" -gt "$cap" ] || {
      echo "FAIL docs-format:chapter-stays-within-200-lines delist $file: now fits"
      exit 1
    }
  done <"$debt"
fi
find . \
  -type d \( -name .git -o -name node_modules -o -name .venv -o -name vendor -o -name target -o -name dist \) -prune -o \
  -type f \( -name '[0-9][0-9]-*.md' -o -name glossary.md -o -name README.md \) -print |
  while IFS= read -r file; do
    if [ -f "$debt" ] && { grep -Fxq "$file" "$debt" || grep -Fxq "${file#./}" "$debt"; }; then
      continue
    fi
    cap=$(cap_for "$file")
    lines=$(wc -l <"$file")
    [ "$lines" -le "$cap" ] || {
      echo "FAIL docs-format:chapter-stays-within-200-lines $file"
      exit 1
    }
  done
