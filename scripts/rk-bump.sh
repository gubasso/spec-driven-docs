#!/usr/bin/env bash
# Move the rk pin to release-kit's latest release, transactionally and under
# one lock.
#
# nix-update rewrites the pin in place and writes the version before it
# resolves either hash, so an interrupted or failing run leaves a version
# whose hashes no longer match: a pin that evaluates to nothing buildable,
# which the watch in .envrc then feeds straight into the next shell. Every
# caller reaches the updater through this script, so one snapshot discipline
# and one lock cover all of them — two overlapping callers could otherwise
# snapshot each other's half-written state and restore it over a good result.
#
# Invoke it by its real path: it locates the repository from its own, and
# follows no symlink to get there.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pin="$root/nix/rk.nix"

if ! command -v flock >/dev/null 2>&1; then
  echo "rk-bump: flock is required to serialize the update; enter the devshell" >&2
  exit 1
fi

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

backup="$(mktemp)"
cp "$pin" "$backup"
bump_ok=""

# Restoration is the exit path's responsibility, not a statement after the
# updater, so an interrupt lands on it too.
finish() {
  [ -e "$backup" ] || return 0
  if [ -n "$bump_ok" ]; then
    rm -f "$backup"
    return 0
  fi
  if cp "$backup" "$pin"; then
    rm -f "$backup"
    echo "rk-bump: the update failed and the pin is unchanged" >&2
  else
    echo "rk-bump: the pin could not be restored; its previous contents are at $backup" >&2
  fi
}
# A signal arriving between commands would otherwise kill the shell outright,
# and bash runs no EXIT trap for a signal it has no trap for. Converting each
# to a normal exit is what keeps finish reachable.
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# --url overrides the version oracle without touching src, so release-kit's
# GitHub releases decide the version while crates.io serves the bytes.
# --build then builds what was selected, so a release that does not compile
# fails inside this envelope rather than in the next shell.
(cd "$root" && nix run nixpkgs#nix-update -- --flake --build \
  --url https://github.com/gubasso/release-kit \
  --use-github-releases --version=stable release-kit)
bump_ok=1
