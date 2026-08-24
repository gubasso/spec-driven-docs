#!/usr/bin/env sh
set -eu
for file; do
  tables=$(grep -c '^|.*|$' "$file" || true)
  # shellcheck disable=SC2016
  [ "$tables" -eq 0 ] || grep -qE '^Verified: ([0-9]{4}-[0-9]{2}-[0-9]{2}|`<YYYY-MM-DD>`)' "$file" || {
    echo "FAIL comparison-docs:every-table-is-dated $file"
    exit 1
  }
done
