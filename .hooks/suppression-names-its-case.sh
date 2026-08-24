#!/usr/bin/env sh
# Every markdown suppression names the case that justifies it, and that case
# resolves to a record.
#
# A suppression with no case behind it becomes permanent by default: the next
# reader takes it for a design choice and nothing says what would retire it.
#
# Scoped to the HTML-comment form, which is the suppression a document can
# carry. Prose about a suppression -- a chapter teaching `# shellcheck disable`
# -- is content, not a suppression, and a shell script carries its own gate.
#
# The search is `grep -rI`, not a faster tool, because every hook here runs on a
# consumer's machine: a gate that silently reports success when its search tool
# is absent is worse than one that is not installed at all.
set -eu

# shellcheck source-path=SCRIPTDIR
# shellcheck disable=SC1091
# shellcheck source=lib-instance-paths.sh
. "$(dirname "$0")/lib-instance-paths.sh"

status=0
raw=$(grep -rIn --exclude-dir=.git --exclude-dir=known-issues \
  -e '<!-- *dprint-ignore' -e '<!-- *markdownlint-disable' .) || status=$?
[ "$status" -le 1 ] || {
  echo "FAIL spec-to-code:a-suppression-names-its-case: grep exited $status"
  exit 1
}
sup=$(printf '%s\n' "$raw" | grep -v -e dprint-ignore-end -e markdownlint-enable) || true

bad=$(printf '%s\n' "$sup" | grep '<!--' | grep -v 'KI-[a-z0-9-]') || true
[ -z "$bad" ] || {
  echo "FAIL spec-to-code:a-suppression-names-its-case"
  echo "$bad"
  exit 1
}

known=$(ki_records "$@" | sed -e 's|.*/||' -e 's|\.md$||' | sort -u)
for c in $(printf '%s\n' "$sup" | grep -oE 'KI-[a-z0-9-]+' | sort -u); do
  printf '%s\n' "$known" | grep -Fxq "$c" || {
    echo "FAIL spec-to-code:a-suppression-names-its-case: $c resolves to no record"
    exit 1
  }
done
