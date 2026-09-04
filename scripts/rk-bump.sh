#!/usr/bin/env bash
# Move the release-kit flake input to a release tag — the argument, or the
# latest GitHub release — transactionally and under one lock.
#
# The operation mutates flake.nix, then updates flake.lock, then builds:
# three steps over two files. An interrupt or failure between them leaves a
# new input URL against an old lock — a pin that evaluates to nothing
# buildable, which the flake watch in .envrc then feeds straight into the
# next shell. Every caller reaches the rewrite through this script, so one
# snapshot discipline and one lock cover all of them — two overlapping
# callers could otherwise snapshot each other's half-written state and
# restore it over a good result.
#
# Invoke it by its real path: it locates the repository from its own, and
# follows no symlink to get there.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
flake="$root/flake.nix"
lock="$root/flake.lock"

if ! command -v flock >/dev/null 2>&1; then
  echo "rk-bump: flock is required to serialize the update; enter the devshell" >&2
  exit 1
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "rk-bump: jq is required to verify the lock's outcome; enter the devshell" >&2
  exit 1
fi

# The lock's answer for the release-kit input: the node the root's input
# map names, followed through the lock's own structure, so no other
# input's ref can stand in for it.
locked_ref() {
  jq -r '.nodes[.root].inputs["release-kit"] as $node | .nodes[$node].original.ref // empty' "$lock" 2>/dev/null || true
}

# Repo-local and gitignored, so the lock is per checkout and never committed.
lockdir="$root/.direnv"
mkdir -p "$lockdir"
exec 9>"$lockdir/rk-bump.lock"
# Contention gets an exit code of its own, so that the one case worth passing
# over quietly — another caller already bumping — is not conflated with a lock
# that cannot be taken at all, on a filesystem that does not carry them for
# instance. The latter has to be reported: the daily stamp is already written
# by then, so a silent skip would repeat every day without ever saying why.
lock_rc=0
if [ -n "${RK_BUMP_NONBLOCK:-}" ]; then
  # The directory-entry caller declines rather than stalling a shell behind
  # another bump's build.
  flock -n -E 100 9 || lock_rc=$?
  if [ "$lock_rc" -eq 100 ]; then exit 0; fi
else
  flock 9 || lock_rc=$?
fi
if [ "$lock_rc" -ne 0 ]; then
  echo "rk-bump: the lock could not be taken; flock exited $lock_rc" >&2
  exit 1
fi

# The named repository decides, never a caller's exported git context: a run
# from inside a git hook must judge $root, not the hook's own repository.
git_here() {
  env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE -u GIT_PREFIX \
    git -C "$root" "$@"
}

# Refuse before mutating when the operator already has edits in the two files
# this run would snapshot and later restore: the envelope must never be
# blamed for losing work it did not create. Porcelain status covers the
# staged and the unstaged form alike — plain diff compares only the working
# tree against the index, so a staged edit would slip through it.
if command -v git >/dev/null 2>&1 &&
  git_here rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  # Captured separately so a status that fails outright refuses too: a
  # check that cannot judge the tree must fail closed, never proceed.
  if ! pending="$(git_here status --porcelain -- flake.nix flake.lock)"; then
    echo "rk-bump: git status failed, so the tree cannot be judged clean; refusing" >&2
    exit 1
  fi
  if [ -n "$pending" ]; then
    echo "rk-bump: flake.nix or flake.lock carries uncommitted changes; commit or stash them first" >&2
    exit 1
  fi
fi

# Normalize the three input shapes into a bare tag, each form on its own: a
# suffix strip written for the URL form would hand the argument form a
# doubled prefix (vv0.2.9), a tag that does not exist.
if [ -n "${1:-}" ]; then
  raw="$1"
else
  # /releases/latest redirects to the newest tag that is neither a draft nor
  # a prerelease, and needs no token.
  if ! raw="$(curl -fsSL --max-time 30 -o /dev/null -w '%{url_effective}' \
    https://github.com/gubasso/release-kit/releases/latest)"; then
    echo "rk-bump: the latest release tag could not be discovered from GitHub" >&2
    exit 1
  fi
fi
case "$raw" in
  https://* | http://*) want="${raw##*/}" ;;
  v*) want="$raw" ;;
  *) want="v$raw" ;;
esac
if ! printf '%s\n' "$want" | grep -qE '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "rk-bump: not a release tag: $want" >&2
  exit 1
fi

# One anchored matcher for counting, reading, rewriting, and verifying:
# the complete assignment line — leading whitespace, `url =`, the quoted
# URL, the terminating semicolon, nothing else. A version named in a
# comment, quoted or not, a suffixed tag like v0.2.8-beta, and an
# assignment sharing a line with anything are all outside it: the first
# two never count, and a pin whose line is not clean counts zero and
# refuses loudly rather than taking a guessed rewrite. Exactly one line
# may match: zero means the flake changed shape, more than one is an
# ambiguity a blind rewrite would resolve wrongly.
pin='^[[:space:]]*url[[:space:]]*=[[:space:]]*"github:gubasso/release-kit/v[0-9]+\.[0-9]+\.[0-9]+";[[:space:]]*$'
matches=$(grep -cE "$pin" "$flake") || matches=0
if [ "$matches" -ne 1 ]; then
  echo "rk-bump: expected 1 pin assignment line in flake.nix, found $matches" >&2
  exit 1
fi

# A no-op bump writes nothing — but only a true no-op: the text and the
# lock's own answer must agree on the tag, or a decoy at the requested
# version could masquerade as done while the live input sits elsewhere.
current="$(grep -E "$pin" "$flake" | head -n 1)"
current="${current##*release-kit/}"
if [ "${current%%\";*}" = "$want" ]; then
  if [ "$(locked_ref)" = "$want" ]; then
    exit 0
  fi
  echo "rk-bump: the pin reads $want but the lock's release-kit node does not reference it; the flake's input shape needs a human look" >&2
  exit 1
fi

backup_flake="$(mktemp)"
backup_lock="$(mktemp)"
cp "$flake" "$backup_flake"
cp "$lock" "$backup_lock"
bump_ok=""

# Restoration is the exit path's responsibility, not a statement after the
# update, so an interrupt lands on it too. Both files restore together:
# atomicity per file is not atomicity per operation.
finish() {
  [ -e "$backup_flake" ] || return 0
  if [ -n "$bump_ok" ]; then
    rm -f "$backup_flake" "$backup_lock"
    return 0
  fi
  restore_failed=""
  cp "$backup_flake" "$flake" || restore_failed=1
  cp "$backup_lock" "$lock" || restore_failed=1
  if [ -z "$restore_failed" ]; then
    rm -f "$backup_flake" "$backup_lock"
    echo "rk-bump: the update failed and the pin is unchanged" >&2
  else
    echo "rk-bump: the pin could not be restored; its previous contents are at $backup_flake and $backup_lock" >&2
  fi
}
# A signal arriving between commands would otherwise kill the shell outright,
# and bash runs no EXIT trap for a signal it has no trap for. Converting each
# to a normal exit is what keeps finish reachable.
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# A temp file and a rename, not `sed -i`: GNU takes -i where BSD requires
# -i '', and the devshell supports Darwin. The rewrite uses the same
# anchored shape as the matcher, and the result is verified before the
# lock moves: the assignment must now carry exactly the wanted tag, once,
# or the run fails inside the envelope, which restores both files.
sed -E "s|^([[:space:]]*url[[:space:]]*=[[:space:]]*\")github:gubasso/release-kit/v[0-9]+\.[0-9]+\.[0-9]+(\";[[:space:]]*)$|\1github:gubasso/release-kit/$want\2|" "$flake" >"$flake.tmp"
mv "$flake.tmp" "$flake"
wanted="^[[:space:]]*url[[:space:]]*=[[:space:]]*\"github:gubasso/release-kit/$want\";[[:space:]]*$"
rewritten=$(grep -cE "$wanted" "$flake") || rewritten=0
remaining=$(grep -cE "$pin" "$flake") || remaining=0
if [ "$rewritten" -ne 1 ] || [ "$remaining" -ne 1 ]; then
  echo "rk-bump: the rewrite did not land on exactly the pin assignment; found $rewritten new and $remaining total" >&2
  exit 1
fi
(cd "$root" && nix flake update release-kit)
# The outcome judged, not only the text: whatever the matcher did, the
# lock's release-kit node — followed through the lock's own structure, so
# no other input's ref can stand in — must now reference the wanted tag.
# This is what catches a decoy the text matcher could not: a live input
# refactored into a shape the anchored line no longer matches while an
# old copy lingers in a block comment, because the lock reflects the
# input Nix actually resolved.
if [ "$(locked_ref)" != "$want" ]; then
  echo "rk-bump: the lock's release-kit node does not reference $want; the pin the rewrite touched is not the live input" >&2
  exit 1
fi
# --no-link: .envrc triggers this on directory entry, and a routine cd must
# not drop a result symlink into the working tree. The build is also the
# proof the follows deal demands: a release that does not evaluate or build
# against this repository's nixpkgs fails inside this envelope rather than
# in the next shell.
(cd "$root" && nix build --no-link \
  ".#devShells.$(nix eval --impure --raw --expr builtins.currentSystem).default")
bump_ok=1
