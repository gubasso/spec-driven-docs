#!/usr/bin/env sh
# Every verdict symbol carries the word the legend gives it.
#
# The match is per occurrence, not per line. Filtering whole lines exonerates a
# row for all of its cells as soon as one cell is annotated, and a row with
# several subject columns -- the shape a comparison table is made of -- is
# exactly where the bare verdict hides.
set -eu
for file; do
  awk '
    {
      line = $0
      while (match(line, /(✅|⚠️|❌|➖|🧪|❓)( [a-z\/]+)?/)) {
        hit = substr(line, RSTART, RLENGTH)
        line = substr(line, RSTART + RLENGTH)
        if (hit == "✅ yes" || hit == "⚠️ partial" || hit == "❌ no" ||
            hit == "➖ n/a" || hit == "🧪 unstable" || hit == "❓ untested") continue
        printf "FAIL comparison-docs:a-verdict-carries-its-word %s:%d: %s\n", FILENAME, FNR, hit
        bad = 1
      }
    }
    END { exit bad }
  ' "$file"
done
