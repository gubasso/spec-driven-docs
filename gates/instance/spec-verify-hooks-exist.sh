#!/usr/bin/env sh
# Every rule whose verification runs a hook names a hook that is still defined.
#
# A rule can be stated in a spec, given a `Verify:` line, and enforced by
# nothing. This reads the hook name out of every `Verify:` that runs pre-commit
# and asserts the hook is defined, so renaming or deleting one fails here
# instead of quietly turning a rule back into a suggestion.
#
# The `$` anchor is load-bearing: without it a hook renamed by suffix still
# matches the id it replaced. Aliases count, because `md-spec` is how the spec
# names its gate.
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

ids=$(grep -ho 'pre-commit run [a-z0-9-]*' "$@" | cut -d' ' -f3 | sort -u)
[ -n "$ids" ] || {
  echo "FAIL no spec names a hook; the Verify shape moved"
  exit 1
}

for h in $ids; do
  grep -qE "^ *(- id|alias): $h\$" .pre-commit-config.yaml || {
    echo "FAIL docs-specs:verification-names-a-live-hook: $h"
    exit 1
  }
done
