#!/usr/bin/env sh
set -eu
[ -f AGENTS.md ] || {
  echo 'FAIL no root AGENTS.md'
  exit 1
}
find . \
  -type d \( -name .git -o -name node_modules -o -name .venv -o -name vendor -o -name target -o -name dist \) -prune -o \
  -type f -name AGENTS.md -print |
  while IFS= read -r file; do
    cap=150
    [ "$file" = ./AGENTS.md ] && cap=100
    lines=$(wc -l <"$file")
    [ "$lines" -le "$cap" ] || {
      echo "FAIL docs-format:author-instructions-stay-within-budget $file"
      exit 1
    }
  done
