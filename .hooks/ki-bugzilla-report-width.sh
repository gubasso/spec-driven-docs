#!/usr/bin/env sh
# A Bugzilla report body fits in 79 columns.
#
# Bugzilla renders a comment as preformatted plain text in a fixed-width box and
# reflows nothing, so a line past that width wraps where Bugzilla chooses. Prose
# survives that; the aligned table and the annotated source excerpt a report is
# made of do not, and those are the parts carrying the argument. 79 is the width
# terminals, diff views, and quoted replies already agree on, and it leaves room
# for a `> `.
#
# The rule is Bugzilla's, not every tracker's, so the gate reads the record's
# `upstream:` and checks nothing else. A tracker that reflows -- GitHub, Jira --
# renders a hard width invisible, and a width no renderer honours is a rule
# nobody can justify keeping.
#
# The tracker is recognised from the whole `upstream:` value, case-insensitively
# and past leading whitespace, and `show_bug.cgi` counts alongside the product
# name: a Bugzilla instance is routinely branded something else -- bugs.kde.org,
# bugs.gentoo.org -- and matching the product name alone turns the gate off for
# exactly the trackers it was written for.
#
# The body sits in a fence, which is what stops a markdown formatter reflowing
# it and what makes the width checkable here rather than after the paste. Body
# text outside the fence fails for the same reason: dprint owns the wrapping of
# everything unfenced, at a width it chooses rather than the tracker's.
#
# The heading match is guarded by the fence state. A body in a tracker markup
# that spells a heading `## ` -- or quotes one -- would otherwise close the
# section from inside itself and take the rest of the body out of the check.
#
# A report kept as a standalone file rather than a fenced section is the same
# rule through .editorconfig: `max_line_length = 79` scoped to `*.bugzilla.txt`,
# read by the editorconfig-checker hook. The tracker lives in the suffix there
# for the same reason it lives in `upstream:` here -- the rule is per-tracker,
# so a sibling `*.jira.txt` is deliberately not matched.
set -eu

# shellcheck source-path=SCRIPTDIR
# shellcheck disable=SC1091
# shellcheck source=lib-instance-paths.sh
. "$(dirname "$0")/lib-instance-paths.sh"

records=$(ki_records "$@")
[ -n "$records" ] || exit 0

status=0
while IFS= read -r record; do
  awk '
  FNR == 1         { report = 0; fence = 0; bugzilla = 0 }
  tolower($0) ~ /^[ \t]*upstream:/ {
    if (tolower($0) ~ /(bugzilla|show_bug\.cgi|bsc#|boo#|bnc#)/) bugzilla = 1
  }
  /^## Report$/    { report = 1; next }
  !fence && /^## / { report = 0 }
  report && /^```/ { fence = !fence; next }
  !bugzilla        { next }
  report && fence && length($0) > 79 {
    printf "FAIL known-issues:a-bugzilla-report-body-fits-in-79-columns %s:%d: %d columns\n",
      FILENAME, FNR, length($0); bad = 1 }
  report && !fence && $0 != "" {
    printf "FAIL known-issues:a-bugzilla-report-body-fits-in-79-columns %s:%d: body outside a fence\n",
      FILENAME, FNR; bad = 1 }
  END { exit bad }
  ' "$record" || status=1
done <<RECORDS
$records
RECORDS
exit "$status"
