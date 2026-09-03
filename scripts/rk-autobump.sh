#!/usr/bin/env bash
# Keep the devshell's rk pin at release-kit's latest GitHub release.
# Runs from .envrc, so direnv alone triggers it and `nix develop` in CI never
# does. Every failure path exits 0: entering the directory offline must still
# produce a working shell.
#
# Invoke it by its real path: it locates the repository from its own, and
# follows no symlink to get there.
set -euo pipefail

if [ -n "${RK_SKIP_AUTOBUMP:-}" ]; then exit 0; fi
if ! command -v rk >/dev/null 2>&1; then exit 0; fi
if ! command -v curl >/dev/null 2>&1; then exit 0; fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# One check a day, per checkout. A directory change is not the moment to
# spend a network round trip.
stamp="${DIRENV_LAYOUT_DIR:-$root/.direnv}/rk-autobump"
if [ -f "$stamp" ] && [ -z "$(find "$stamp" -mtime +0 2>/dev/null)" ]; then exit 0; fi

have="$(rk --version 2>/dev/null | cut -d' ' -f2)"
if [ -z "$have" ]; then exit 0; fi

# /releases/latest redirects to the newest tag that is neither a draft nor a
# prerelease, and needs no token, so a check never spends the API rate limit.
tag="$(curl -fsSL --max-time 5 -o /dev/null -w '%{url_effective}' \
  https://github.com/gubasso/release-kit/releases/latest 2>/dev/null)" || exit 0
want="${tag##*/}"
want="${want#v}"
if [ -z "$want" ]; then exit 0; fi

# Stamped before the attempt, not after it: a bump that fails costs a vendor
# fetch and a build, and retrying that on every directory entry would be
# worse than waiting a day. The failure path below says how to retry now.
mkdir -p "$(dirname "$stamp")"
: >"$stamp"
if [ "$have" = "$want" ]; then exit 0; fi

echo "rk $have -> $want: bumping the pin; the next prompt rebuilds the shell" >&2

# scripts/rk-bump.sh owns the transaction and the lock, so this caller adds
# neither. RK_BUMP_NONBLOCK makes it decline a bump another shell already
# holds rather than stalling this one behind a build. It reports what became
# of the pin itself, so nothing is claimed about it here.
if RK_BUMP_NONBLOCK=1 "$root/scripts/rk-bump.sh" >&2; then exit 0; fi
echo "rk: the bump to $want did not complete; run 'just rk-bump' to retry" >&2
