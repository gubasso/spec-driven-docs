#!/usr/bin/env sh
# The instance manifest is readable and the instance it describes verifies.
#
# The manifest is what every other ownership check reads, so a shape error in it
# disables them all at once. The schema assertion runs first and names the rule
# it enforces, because the verifier it hands off to reads the same file and
# would otherwise report a confusing downstream failure.
set -eu
jq -e '.schema_version == 1 and (.canon_version|type=="string") and (.managed_files|type=="array") and (.adopted_files|type=="array") and (.integration_blocks|type=="array")' .spec-driven-docs/manifest.json >/dev/null || {
  echo 'FAIL distribution:manifest-identifies-every-owned-file .spec-driven-docs/manifest.json: invalid manifest shape'
  exit 1
}
if [ -x .spec-driven-docs/verify.sh ]; then
  .spec-driven-docs/verify.sh --target . --offline
else
  scripts/verify.sh --target . --offline
fi
