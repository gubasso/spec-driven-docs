#!/usr/bin/env sh
# A case id is `KI-<slug>.md`, and the slug is not a counter.
#
# The id a suppression cites has to survive a merge, so it names the bug rather
# than its position in a queue. A digit inside the slug is ordinary — an
# upstream issue number belongs to the story it tells — but a slug that opens
# with one is the counter this rejects.
set -u

status=0
for f; do
  b=${f##*/}
  case "$b" in
    KI-*) ;;
    *)
      echo "FAIL known-issues:case-id-is-a-slug $f: no KI- prefix"
      status=1
      continue
      ;;
  esac
  case "$b" in
    *.md) ;;
    *)
      echo "FAIL known-issues:case-id-is-a-slug $f: not a markdown file"
      status=1
      continue
      ;;
  esac
  s=${b#KI-}
  s=${s%.md}
  case "$s" in
    [0-9]*)
      echo "FAIL known-issues:case-id-is-a-slug $f: a slug, not a counter"
      status=1
      ;;
    '' | *[!a-z0-9-]*)
      echo "FAIL known-issues:case-id-is-a-slug $f: the slug is lowercase, digits and hyphens"
      status=1
      ;;
  esac
done
exit "$status"
