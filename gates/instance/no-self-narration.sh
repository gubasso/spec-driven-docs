#!/usr/bin/env sh
# Run the narration matcher against the files pre-commit passes.
#
# The awk program is the gate; this wrapper exists so the hook can be published
# as `language: script`, which resolves the entry against the hook repository's
# own checkout. A `language: system` entry naming `gates/instance/no-self-narration.awk`
# resolves against the consumer's working directory instead, where that path
# does not exist.
set -eu
exec awk -f "$(dirname "$0")/no-self-narration.awk" "$@"
