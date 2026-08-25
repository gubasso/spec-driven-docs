#!/usr/bin/env sh
# Every known-issue record carries the condition under which it is removed.
#
# A record whose workaround has no exit becomes permanent by default, and the
# next reader takes it for a design choice. The key must carry a value: an empty
# `retire_when:` states no condition and would otherwise report success.
set -eu

# shellcheck source-path=SCRIPTDIR
# shellcheck disable=SC1091
# shellcheck source=lib-instance-paths.sh
. "$(dirname "$0")/lib-instance-paths.sh"

records=$(ki_records "$@")
[ -n "$records" ] || exit 0

bad=""
while IFS= read -r f; do
  grep -qE '^retire_when:[[:space:]]*[^[:space:]]' "$f" || bad="$bad $f"
done <<RECORDS
$records
RECORDS
[ -z "$bad" ] || {
  echo "FAIL known-issues:a-record-carries-its-retirement-condition:$bad"
  exit 1
}
