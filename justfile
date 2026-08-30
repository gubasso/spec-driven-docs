default:
    @just --list

fmt:
    cargo fmt
    dprint fmt

lint:
    cargo fmt --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo deny check
    dprint check
    editorconfig-checker -disable-insert-final-newline
    typos
    markdownlint-cli2 "**/*.md" "#tests/fixtures/**" "#target/**"
    check-jsonschema --schemafile instance/manifest.schema.json .spec-driven-docs/manifest.json
    pre-commit validate-config .pre-commit-config.yaml
    pre-commit run --files $(rg --files --hidden -g '!.git/**')

test:
    cargo nextest run

manifest:
    cargo run -q -- self-manifest
    dprint fmt .spec-driven-docs/manifest.json

# Install into a scratch repository and verify it, end to end, with the real
# binary.
build:
    set -eu; d=$(mktemp -d); trap 'rm -rf "$d"' EXIT; mkdir -p "$d/.git"; cargo run -q -- init --target "$d" --profile knowledge-base --apply >/dev/null; cargo run -q -- verify --target "$d"

check: lint test build

# Install this checkout as the user's sdd, plus the user-scope agent skills.
install:
    cargo install --path . --locked
    sdd skill install --apply

# Remove the user-scope skills, then the binary; the binary owns the file
# list, so the skills go first, while it still exists.
uninstall:
    sdd skill uninstall --apply
    cargo uninstall spec-driven-docs

instantiate target profile="knowledge-base":
    cargo run -q -- init --target "{{target}}" --profile "{{profile}}"

verify-instance target:
    cargo run -q -- verify --target "{{target}}"

upgrade-instance target:
    cargo run -q -- upgrade --target "{{target}}"
