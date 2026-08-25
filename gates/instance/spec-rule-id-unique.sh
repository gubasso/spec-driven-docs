#!/usr/bin/env sh
# A rule ID resolves to exactly one requirement across the whole project.
#
# A commit citing a duplicated ID names two rules at once, so the citation stops
# being an address. Corpus-wide by construction: the duplicate is only visible
# when every spec is read together.
set -eu

# shellcheck source-path=SCRIPTDIR
# shellcheck disable=SC1091
# shellcheck source=lib-instance-paths.sh
. "$(dirname "$0")/lib-instance-paths.sh"

set -- "$(sdd_docs_root)"/specs/SPEC-?*.md
[ -e "$1" ] || {
  echo "FAIL no specs matched; the layout moved"
  exit 1
}

# shellcheck disable=SC2016
dupes=$(grep -hoE '^### `[a-z0-9-]+:[a-z0-9-]+`' "$@" | sort | uniq -d)
[ -z "$dupes" ] || {
  echo "FAIL docs-specs:rule-id-is-unique-and-slugged"
  echo "$dupes"
  exit 1
}
