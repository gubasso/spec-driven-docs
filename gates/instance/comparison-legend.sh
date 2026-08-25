#!/usr/bin/env sh
set -eu
for file; do
  grep -q '^|.*|$' "$file" || continue
  grep -q '^Legend: ' "$file" || {
    echo "FAIL comparison-docs:a-comparison-carries-a-legend $file"
    exit 1
  }
done
