#!/usr/bin/env sh
# A record filename is `ADR-<slug>.md`, and the slug carries no digit.
#
# The slug is the identifier: a counter lets two branches each allocate the next
# number, and the merge leaves one identity claimed twice. The charset is exact
# rather than a prefix test, because `md-adr` is scoped to the slug shape — a
# filename this gate waves through but that hook does not match would carry any
# heading structure at all.
set -u

status=0
for f; do
  b=${f##*/}
  [ "$b" = TEMPLATE-adr.md ] && continue
  case "$b" in
    ADR-*) ;;
    *)
      echo "FAIL decision-records:filename-carries-no-digit $f: no ADR- prefix"
      status=1
      continue
      ;;
  esac
  case "$b" in
    *.md) ;;
    *)
      echo "FAIL decision-records:filename-carries-no-digit $f: not a markdown file"
      status=1
      continue
      ;;
  esac
  s=${b#ADR-}
  s=${s%.md}
  case "$s" in
    '' | *[!a-z-]*)
      echo "FAIL decision-records:filename-carries-no-digit $f: the slug is lowercase and hyphens, with no digit"
      status=1
      ;;
  esac
done
exit "$status"
