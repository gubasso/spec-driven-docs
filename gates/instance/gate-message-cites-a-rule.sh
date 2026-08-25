#!/usr/bin/env sh
# Every rule id a gate prints resolves to a requirement in a spec.
#
# The id in a failure message is an address: the reader follows it to the
# sentence that binds, and a message naming an id no spec defines sends them
# nowhere. Drift is quiet -- a rule is reworded, its id is renamed in the spec,
# and the gate keeps printing the old one -- so the check runs over the gate
# bodies themselves rather than over the documents they read.
set -eu

# shellcheck source-path=SCRIPTDIR
# shellcheck disable=SC1091
# shellcheck source=lib-instance-paths.sh
. "$(dirname "$0")/lib-instance-paths.sh"

# The gate reads its own directory, and any further directory named as an
# argument. An instance receives the delivered set and needs no argument; this
# repository also keeps gates under `gates/canon/`, and their messages cite
# rules from the same specs, so they are passed in rather than left unchecked.
hooks=$(dirname "$0")
set -- "$hooks" "$@"
specs="$(sdd_docs_root)/specs"
[ -d "$specs" ] || {
  echo "FAIL no specs directory at $specs; the layout moved"
  exit 1
}

cited=$(grep -rhoE 'FAIL [a-z0-9-]+:[a-z0-9-]+' "$@" | cut -d' ' -f2 | sort -u)
[ -n "$cited" ] || {
  echo 'FAIL no gate names a rule id; the message shape moved'
  exit 1
}
# shellcheck disable=SC2016
defined=$(grep -rhoE '^### `[a-z0-9-]+:[a-z0-9-]+`' "$specs" | tr -d '`' | cut -d' ' -f2 | sort -u)

status=0
for id in $cited; do
  printf '%s\n' "$defined" | grep -Fxq "$id" || {
    echo "FAIL spec-to-code:a-gate-message-cites-the-rule: $id resolves to no requirement"
    status=1
  }
done
exit "$status"
