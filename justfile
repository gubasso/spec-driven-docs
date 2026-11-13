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

# Apply the managed pre-commit region with the pinned sdd of the devshell,
# the previous release, as every instance does
# (release:the-canon-runs-its-previous-release).
hooks:
    sdd hooks --apply

# Write the payload record, an artifact of this tree like its binary, so the
# sdd built from the tree writes it. The release branch's alignment job
# commits the same form.
manifest:
    cargo run -q -- self-manifest

# Install into a scratch repository and verify it, end to end, with the real
# binary.
build:
    set -eu; d=$(mktemp -d); trap 'rm -rf "$d"' EXIT; mkdir -p "$d/.git"; cargo run -q -- init --target "$d" --profile knowledge-base --apply >/dev/null; cargo run -q -- verify --target "$d"

check: lint lint-push

# Remove the agent skills an earlier release installed under the home, then
# the binary an earlier `cargo install` left. The skills land into each
# instance now, and a project tool is never installed on the host.
#
# `CLAUDE_CONFIG_DIR` is dropped for the skill step alone, because the
# cleanup is for the machine and that variable names one session. A session
# wrapper sets it per terminal to an isolated directory it owns, so a cleanup
# inheriting it would miss the copies under the user's own directory. The
# verb still honours the variable everywhere else.
uninstall:
    env -u CLAUDE_CONFIG_DIR sdd skill uninstall --apply
    cargo uninstall spec-driven-docs

# The instance recipes operate another repository, so they run the pinned
# sdd, as that repository would.
instantiate target profile="knowledge-base":
    sdd init --target "{{target}}" --profile "{{profile}}"

verify-instance target:
    sdd verify --target "{{target}}"

upgrade-instance target:
    sdd upgrade --target "{{target}}"
