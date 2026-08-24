#!/usr/bin/env sh
set -eu
for file; do
  awk '
    /^\|/ {
      code = 0
      for (i = 1; i <= length($0); i++) {
        char = substr($0, i, 1)
        if (char == "`") code = !code
        if (code && char == "|" && substr($0, i - 1, 1) != "\\") {
          print "FAIL comparison-docs:table-pipes-are-escaped " FILENAME ":" FNR
          bad = 1
        }
      }
    }
    END { exit bad }
  ' "$file"
done
