#!/usr/bin/env sh
set -eu
# shellcheck source-path=SCRIPTDIR
# shellcheck disable=SC1091
# shellcheck source=lib-instance-paths.sh
. "$(dirname "$0")/lib-instance-paths.sh"

set -- "$(sdd_docs_root)"/decisions/ADR-?*.md
[ -e "$1" ] || {
  echo 'FAIL no decision records matched'
  exit 1
}
for f; do
  words=$(wc -w <"$f")
  [ "$words" -le 350 ] || {
    echo "FAIL decision-records:body-stays-within-350-words $f: $words"
    exit 1
  }
done
