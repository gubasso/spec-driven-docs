default:
    @just --list

fmt:
    cargo fmt
    dprint fmt

# Every check is a hook, so the commit, the push, and CI run one file set
# with one tool version (release:the-local-hooks-run-every-check-of-ci).
# The sweep commits nothing, so the commit-location guards are skipped as
# the worktree convention directs; each still fires at commit time.
# rk-status-check runs here, because the devshell carries the rk binary.
lint:
    prek validate-config .pre-commit-config.yaml
    SKIP=no-commit-to-branch,rk-worktree-location prek run --files $(rg --files --hidden -g '!.git' -g '!.git/**')

# The costly checks of the push stage: cargo deny, the scratch install of
# `build`, the Nix package, the flake checks, and the one-system evaluation.
lint-push:
    SKIP=no-commit-to-branch,rk-worktree-location prek run --hook-stage pre-push --files $(rg --files --hidden -g '!.git' -g '!.git/**')

test:
    cargo nextest run

# Apply the managed pre-commit region this repository runs: the same render
# an instance receives, with this checkout's own entry prefix.
hooks:
    cargo run -q -- hooks --apply --entry 'cargo run -q --'

# The record is generated, so it keeps the form `self-manifest` writes, the
# same form the release branch's alignment job commits.
manifest: hooks
    cargo run -q -- self-manifest

# Install into a scratch repository and verify it, end to end, with the real
# binary.
build:
    set -eu; d=$(mktemp -d); trap 'rm -rf "$d"' EXIT; mkdir -p "$d/.git"; cargo run -q -- init --target "$d" --profile knowledge-base --apply >/dev/null; cargo run -q -- verify --target "$d"

check: lint lint-push

# Install this checkout as the user's sdd, plus the user-scope agent skills.
#
# `CLAUDE_CONFIG_DIR` is dropped for the skill step alone, because these two
# recipes install for the machine and that variable names one session. A
# session wrapper sets it per terminal to an isolated directory it owns, so an
# install inheriting it would write that terminal's directory instead of the
# user's, and would refuse at the wrapper's own declared link on the way. The
# verb still honours the variable everywhere else, which is what an install
# into a relocated configuration directory needs.
install:
    cargo install --path . --locked
    env -u CLAUDE_CONFIG_DIR sdd skill install --apply

# Remove the user-scope skills, then the binary; the binary owns the file
# list, so the skills go first, while it still exists. The variable is dropped
# for the same reason as the install above: the removal must reach the files
# the install wrote.
uninstall:
    env -u CLAUDE_CONFIG_DIR sdd skill uninstall --apply
    cargo uninstall spec-driven-docs

instantiate target profile="knowledge-base":
    cargo run -q -- init --target "{{target}}" --profile "{{profile}}"

verify-instance target:
    cargo run -q -- verify --target "{{target}}"

upgrade-instance target:
    cargo run -q -- upgrade --target "{{target}}"
