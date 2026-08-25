#!/usr/bin/env sh
# A record filed upstream carries the body it was filed with.
#
# Once `upstream:` names one issue rather than a tracker, the report exists in
# two places, and only one of them is under review here. A `## Report` section
# holding the filed text in the tracker's own markup keeps the repository
# holding what was actually said upstream, and makes the next filing a paste.
#
# What the gate can see is the pairing: a specific upstream reference and the
# section. That the section is the filed text, in the tracker's markup, is held
# by review — as is the body written ahead of filing, which no frontmatter
# announces.
set -eu

# shellcheck source-path=SCRIPTDIR
# shellcheck disable=SC1091
# shellcheck source=lib-instance-paths.sh
. "$(dirname "$0")/lib-instance-paths.sh"

records=$(ki_records "$@")
[ -n "$records" ] || exit 0

# A specific item: an id introduced by `/`, `#` or `=` anywhere in the value.
# The `=` form is not decoration -- a canonical Bugzilla link is
# show_bug.cgi?id=<digits>, so without it a record filed into Bugzilla by the
# URL a reporter actually pastes carries no report and the gate says nothing.
# The match is unanchored at its right edge because a citation routinely carries
# more than the id: a comment anchor, a status note, or a second link.
filed='^[[:space:]]*[Uu]pstream:.*[/#=][0-9]+'

bad=""
while IFS= read -r f; do
  grep -qE "$filed" "$f" || continue
  grep -qE '^## Report$' "$f" || bad="$bad $f"
done <<RECORDS
$records
RECORDS
[ -z "$bad" ] || {
  echo "FAIL known-issues:a-filed-record-carries-its-report:$bad"
  exit 1
}
