default:
    @just --list

fmt:
    dprint fmt
    shfmt -w -i 2 -ci scripts/*.sh .hooks/*.sh tests/*.sh

lint:
    dprint check
    shfmt -d -i 2 -ci scripts/*.sh .hooks/*.sh tests/*.sh
    shellcheck scripts/*.sh .hooks/*.sh tests/*.sh
    editorconfig-checker -disable-insert-final-newline
    typos
    markdownlint-cli2 "**/*.md" "#tests/fixtures/**"
    check-jsonschema --schemafile instance/manifest.schema.json .spec-driven-docs/manifest.json
    pre-commit validate-config .pre-commit-config.yaml
    pre-commit validate-manifest .pre-commit-hooks.yaml
    pre-commit run --files $(rg --files --hidden -g '!.git/**')

test-gates:
    .hooks/test-gates.sh

test-instantiation:
    tests/test-instantiation.sh

test-upgrade:
    tests/test-upgrade.sh

test-plugin:
    .hooks/plugin-layout.sh

test: test-gates test-instantiation test-upgrade test-plugin

build:
    set -eu; d=$(mktemp -d "../sdd-build.XXXXXX"); d=$(cd "$d" && pwd -P); trap 'rm -rf "$d"' EXIT; mkdir -p "$d/.git"; scripts/instantiate.sh --target "$d" --profile knowledge-base >/dev/null; "$d/.spec-driven-docs/verify.sh" --target "$d" --offline

check: lint test build

instantiate target profile="knowledge-base":
    scripts/instantiate.sh --target "{{target}}" --profile "{{profile}}"

verify-instance target:
    scripts/verify.sh --target "{{target}}" --offline

upgrade-instance target from:
    scripts/upgrade.sh --target "{{target}}" --from "{{from}}"
