#!/usr/bin/env sh
# Every known-issue record walks its mechanism step by step.
#
# A record that only names the defect is unfalsifiable to everyone but its
# author, and the reader who arrives from a suppression's case id knows nothing
# about the bug. The heading is what the gate can see; the walkthrough being a
# run rather than a restatement is held by review.
#
# The heading is matched anchored and case-sensitively, so a record that buries
# the phrase in prose does not clear the gate.
set -eu

# shellcheck source-path=SCRIPTDIR
# shellcheck disable=SC1091
# shellcheck source=lib-instance-paths.sh
. "$(dirname "$0")/lib-instance-paths.sh"

records=$(ki_records "$@")
[ -n "$records" ] || exit 0

bad=""
while IFS= read -r f; do
  grep -qE '^## How it works$' "$f" || bad="$bad $f"
done <<RECORDS
$records
RECORDS
[ -z "$bad" ] || {
  echo "FAIL known-issues:a-record-walks-the-mechanism:$bad"
  exit 1
}
