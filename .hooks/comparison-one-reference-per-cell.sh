#!/usr/bin/env sh
set -eu
for file; do
  awk -F'|' '/^\|/ { for(i=2;i<NF;i++){ c=$i; n=gsub(/\]\(/,"&",c); if(n>1){print "FAIL comparison-docs:a-cell-carries-one-reference " FILENAME ":" FNR; bad=1}}} END{exit bad}' "$file"
done
