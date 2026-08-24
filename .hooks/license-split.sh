#!/usr/bin/env sh
# Both licenses stay named where a reader and a tool look for them.
#
# A dual-licensed repository fails quietly: one file is renamed, or `LICENSE`
# stops naming the half it does not carry, and whoever opens it is told the
# wrong terms for half the tree. Each
# assertion names the rule it enforces, because a bare test under `set -e`
# exits non-zero with nothing on stdout, and a control that reads only the
# status cannot tell that from a crash.
set -eu

for f in LICENSE LICENSE-MIT LICENSE-CC-BY-4.0; do
  [ -s "$f" ] || {
    echo "FAIL distribution:license-declares-both-halves $f: missing or empty"
    exit 1
  }
done

for f in LICENSE-MIT LICENSE-CC-BY-4.0; do
  grep -Fq "$f" LICENSE || {
    echo "FAIL distribution:license-declares-both-halves LICENSE: does not name $f"
    exit 1
  }
done
