#!/usr/bin/env sh
# Regenerate this repository's own instance manifest.
#
# The canon is an instance of itself, but not one `instantiate.sh` can produce:
# its managed files sit where it authors them rather than under
# `.spec-driven-docs/`. Recording thirty hashes by hand is how the manifest
# drifts from the payload, so it is generated from the payload instead.
set -eu
root=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd -P)
cd "$root"
manifest=.spec-driven-docs/manifest.json
[ -f "$manifest" ] || {
  echo "FAIL missing $manifest"
  exit 1
}
work=$(mktemp -d "${TMPDIR:-/tmp}/sdd-self.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
: >"$work/managed"
: >"$work/adopted"

# shellcheck disable=SC2016  # the jq filter's $s and $h are jq variables
record() {
  hash=$(sha256sum "$1" | cut -d' ' -f1)
  jq -nc --arg s "$1" --arg h "$hash" "$2" >>"$3"
}
# shellcheck disable=SC2016
for file in scripts/verify.sh .markdownlint/*.markdownlint-cli2.jsonc gates/instance/* gates/canon/*; do
  [ -f "$file" ] || continue
  record "$file" '{source:$s,destination:$s,sha256:$h}' "$work/managed"
done
# shellcheck disable=SC2016
for file in _docs/specs/SPEC-*.md _docs/decisions/TEMPLATE-adr.md _docs/reference/TEMPLATE-agents-digest.md; do
  [ -f "$file" ] || continue
  record "$file" '{source:$s,destination:$s,sha256:$h,baseline_sha256:$h}' "$work/adopted"
done

marker=$(sed -n '/^# BEGIN spec-driven-docs managed$/,/^# END spec-driven-docs managed$/p' .pre-commit-config.yaml | sha256sum | cut -d' ' -f1)
jq -s '.' "$work/managed" >"$work/managed.json"
jq -s '.' "$work/adopted" >"$work/adopted.json"
jq -n --arg version "$(cat VERSION)" --arg installed "$(jq -r .installed_at "$manifest")" --arg marker "$marker" \
  --slurpfile managed "$work/managed.json" --slurpfile adopted "$work/adopted.json" \
  '{schema_version:1,canon_version:$version,canon_source:"https://github.com/gubasso/spec-driven-docs",canon_ref:"pre-release",profile:"knowledge-base",docs_root:"_docs",installed_at:$installed,managed_files:$managed[0],adopted_files:$adopted[0],integration_blocks:[{path:".pre-commit-config.yaml",marker_hash:$marker}]}' \
  >"$work/manifest.json"
mv "$work/manifest.json" "$manifest"
dprint fmt "$manifest" >/dev/null
# The formatter changes the manifest's own bytes, never a recorded file's, so
# one more pass is not needed -- but verify.sh is recorded and this script does
# not touch it, so the result is stable on a second run.
echo "OK regenerated $manifest"
