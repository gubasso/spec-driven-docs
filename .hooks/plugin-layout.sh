#!/usr/bin/env sh
# The plugin declares this project's version and routes to the same scripts.
#
# Each assertion names the rule it enforces, because a bare `jq -e` under
# `set -e` exits non-zero with nothing on stdout, and a control that reads only
# the status cannot tell that from a crash.
set -eu
version=$(cat VERSION)
jq -e --arg version "$version" '.name == "spec-driven-docs" and .version == $version and .license == "CC-BY-4.0"' .claude-plugin/plugin.json >/dev/null || {
  echo "FAIL distribution:versions-are-semantic-and-aligned .claude-plugin/plugin.json: expected version $version"
  exit 1
}
jq -e '.plugins | length == 1 and .[0].source == "./"' .claude-plugin/marketplace.json >/dev/null || {
  echo 'FAIL distribution:plugin-and-scripts-have-parity .claude-plugin/marketplace.json: expected one plugin sourced at ./'
  exit 1
}
for skill in instantiate verify upgrade; do
  file="skills/$skill/SKILL.md"
  command_name=$skill
  [ "$skill" = instantiate ] && command_name=init
  if ! { [ -f "$file" ] && grep -q '^name:' "$file" && grep -q "skills/$skill/SKILL.md" "commands/$command_name.md"; }; then
    echo "FAIL distribution:plugin-and-scripts-have-parity $file: skill and command are not wired"
    exit 1
  fi
done
