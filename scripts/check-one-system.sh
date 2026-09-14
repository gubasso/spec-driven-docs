#!/usr/bin/env bash
# Hold the flake to one exposed system.
#
# The evaluated half of release:the-binary-builds-for-every-declared-target.
# The canon test reads flake.nix as text and holds one canonical authored
# form; this asks the evaluator which systems the flake really exposes, so a
# second system declared in any spelling is caught.
#
# nix/systems.nix says why this reads the flake's outputs rather than the
# JSON that `nix flake show` prints.
set -euo pipefail

supported="x86_64-linux"
# getFlake needs an absolute path, so a relative argument is resolved here
# rather than left for the evaluator to refuse.
reference="$(cd "${1:-.}" && pwd)"

found="$(
  nix eval --impure --json \
    --expr "import ./nix/systems.nix \"${reference}\"" |
    jq -r 'unique | join(" ")'
)"

if [ "${found}" != "${supported}" ]; then
  echo "the flake exposes [${found}], and ${supported} is the only supported system" >&2
  echo "see _docs/decisions/ADR-linux-is-the-only-supported-target.md" >&2
  exit 1
fi

echo "the flake exposes ${supported} and nothing else"
