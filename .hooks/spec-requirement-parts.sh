#!/usr/bin/env sh
# Every requirement in a spec is a rule ID heading followed by its verification.
#
# The check is per block, not per file. Counting rule IDs and `Verify:` lines
# over a whole spec and comparing the totals passes a spec whose second
# requirement has no verification and whose first has two — the relationship the
# rule states is ownership, so a count cannot see it. Each `###` heading opens a
# block that runs to the next one or to end of file, and each block owes exactly
# one rule ID in its heading and exactly one verification in its body.
#
# The separator after the rule ID is matched as `[^ ]+` rather than `.`, because
# it is an em-dash: one `.` is one byte under `LC_ALL=C` and in mawk, so a
# conforming heading is rejected on every machine whose awk is byte-oriented.
set -u

status=0
for f; do
  awk -v file="$f" '
    function close_block() {
      if (heading == "") return
      if (heading !~ /^### `[a-z0-9-]+:[a-z0-9-]+` [^ ]+ /) {
        printf "FAIL docs-specs:requirement-carries-five-parts %s:%d: heading is not a rule id\n", file, hline
        bad = 1
      }
      if (verifies != 1) {
        printf "FAIL docs-specs:requirement-carries-five-parts %s:%d: %d verifications, expected 1\n", file, hline, verifies
        bad = 1
      }
    }
    /^### / { close_block(); heading = $0; hline = FNR; verifies = 0; next }
    /^Verify: / { if (heading != "") verifies++; next }
    END { close_block(); exit bad }
  ' "$f" || status=1
done
exit "$status"
