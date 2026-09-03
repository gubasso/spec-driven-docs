#!/usr/bin/env bats
# The auto-bump helper mutates a tracked Nix expression on directory entry and
# coordinates concurrent shells, so every branch is exercised here against
# stubs rather than the network. Each stub lives in its own PATH directory and
# records its own call, so a case cannot pass because a different guard fired.

setup() {
  REPO="$(mktemp -d)"
  mkdir -p "$REPO/scripts" "$REPO/nix" "$REPO/stub/rk" "$REPO/stub/curl" "$REPO/calls"
  cp "$BATS_TEST_DIRNAME/../scripts/rk-autobump.sh" "$REPO/scripts/rk-autobump.sh"

  PIN="$REPO/nix/rk.nix"
  ORIG="$REPO/nix/rk.nix.orig"
  printf 'version = "0.2.8";\n' >"$PIN"
  cp "$PIN" "$ORIG"

  DIRENV_LAYOUT_DIR="$REPO/.direnv"
  STAMP="$DIRENV_LAYOUT_DIR/rk-autobump"
  CALLS="$REPO/calls"
  export DIRENV_LAYOUT_DIR CALLS

  cat >"$REPO/stub/rk/rk" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$@" >"$CALLS/rk"
echo "rk ${RK_STUB_VERSION:-0.2.8}"
STUB

  # The helper reads only the effective URL, so the stub prints the redirect
  # target the real /releases/latest would land on.
  cat >"$REPO/stub/curl/curl" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$@" >"$CALLS/curl"
if [ -n "${CURL_STUB_FAIL:-}" ]; then exit 22; fi
echo "https://github.com/gubasso/release-kit/releases/tag/v${CURL_STUB_LATEST:-0.2.9}"
STUB

  # Stands in for the transactional envelope the helper delegates to.
  cat >"$REPO/scripts/rk-bump.sh" <<'STUB'
#!/usr/bin/env bash
: >"$CALLS/bump"
printf '%s\n' "${RK_BUMP_NONBLOCK:-}" >"$CALLS/bump-nonblock"
if [ -n "${BUMP_STUB_FAIL:-}" ]; then exit 1; fi
printf 'version = "%s";\n' "${CURL_STUB_LATEST:-0.2.9}" >"$RK_PIN"
STUB

  chmod +x "$REPO/stub/rk/rk" "$REPO/stub/curl/curl" "$REPO/scripts/rk-bump.sh"
  RK_PIN="$PIN"
  export RK_PIN
  PATH="$REPO/stub/rk:$REPO/stub/curl:$PATH"
  export PATH
}

teardown() {
  rm -rf "$REPO"
}

# Drop every directory that provides $1 from PATH: the command can sit in more
# than one, and a host cargo install of rk alongside the devshell's is exactly
# the case this helper has to tolerate. Nix gives each package its own bin
# directory, so removing one takes that command and nothing else.
hide_command() {
  local path guard=0
  while path="$(command -v "$1" 2>/dev/null)"; do
    guard=$((guard + 1))
    [ "$guard" -gt 20 ] && break
    PATH="$(printf '%s' "$PATH" | tr ':' '\n' | grep -vx "$(dirname "$path")" | paste -sd: -)"
    export PATH
  done
}

pin_unchanged() {
  cmp -s "$ORIG" "$PIN"
}

@test "the opt-out exits zero before reading any version" {
  RK_SKIP_AUTOBUMP=1 run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  [ ! -e "$STAMP" ]
  [ ! -e "$CALLS/rk" ]
}

@test "a missing rk exits zero without reaching curl" {
  hide_command rk
  run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  [ ! -e "$STAMP" ]
  [ ! -e "$CALLS/curl" ]
}

@test "a missing curl exits zero after rk is still resolvable" {
  hide_command curl
  [ -n "$(command -v rk)" ]
  run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  [ ! -e "$STAMP" ]
  [ ! -e "$CALLS/bump" ]
}

@test "a fresh stamp short-circuits before curl is called" {
  mkdir -p "$DIRENV_LAYOUT_DIR"
  : >"$STAMP"
  run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  [ ! -e "$CALLS/curl" ]
  [ ! -e "$CALLS/bump" ]
}

@test "an unreachable release check exits zero and leaves the pin alone" {
  CURL_STUB_FAIL=1 run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  [ -e "$CALLS/curl" ]
  [ ! -e "$CALLS/bump" ]
  pin_unchanged
}

@test "a current pin stamps and bumps nothing" {
  RK_STUB_VERSION=0.2.9 CURL_STUB_LATEST=0.2.9 run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  [ -e "$STAMP" ]
  [ -e "$CALLS/curl" ]
  [ ! -e "$CALLS/bump" ]
}

@test "a behind pin bumps and reports the move" {
  RK_STUB_VERSION=0.2.8 CURL_STUB_LATEST=0.2.9 run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  [ -e "$CALLS/bump" ]
  grep -q '0.2.9' "$PIN"
  [[ "$output" == *"rk 0.2.8 -> 0.2.9"* ]]
}

@test "a failed bump reports the retry without claiming the pin's state" {
  RK_STUB_VERSION=0.2.8 CURL_STUB_LATEST=0.2.9 BUMP_STUB_FAIL=1 \
    run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  [ -e "$CALLS/bump" ]
  pin_unchanged
  [[ "$output" == *"did not complete"* ]]
  [[ "$output" == *"run 'just rk-bump' to retry"* ]]
  # The envelope owns the pin's fate, so this caller asserts nothing about it.
  [[ "$output" != *"the pin stays at"* ]]
}

@test "the bump is delegated non-blocking so a busy checkout is skipped" {
  # The lock lives in the envelope, so what this caller owes is the flag that
  # makes the envelope decline instead of stalling a shell behind a build.
  RK_STUB_VERSION=0.2.8 CURL_STUB_LATEST=0.2.9 run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  [ -e "$CALLS/bump" ]
  grep -q '^1$' "$CALLS/bump-nonblock"
}

@test "the version and the release check are asked for as exact commands" {
  # Whole vectors, because each part is load-bearing: dropping -w leaves the
  # tag empty and silently disables every bump, dropping -o prints the body
  # instead of the effective URL, and dropping --max-time unbounds a check
  # that runs on directory entry.
  RK_STUB_VERSION=0.2.9 CURL_STUB_LATEST=0.2.9 run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  printf -- '--version\n' >"$REPO/expected-rk"
  diff "$REPO/expected-rk" "$CALLS/rk"
  cat >"$REPO/expected-curl" <<'ARGS'
-fsSL
--max-time
5
-o
/dev/null
-w
%{url_effective}
https://github.com/gubasso/release-kit/releases/latest
ARGS
  diff "$REPO/expected-curl" "$CALLS/curl"
}

@test "an envelope that refuses is reported as an incomplete bump" {
  cat >"$REPO/scripts/rk-bump.sh" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "${RK_BUMP_NONBLOCK:-}" >"$CALLS/bump-nonblock"
echo "rk-bump: the lock could not be taken; flock exited 5" >&2
exit 1
STUB
  chmod +x "$REPO/scripts/rk-bump.sh"
  RK_STUB_VERSION=0.2.8 CURL_STUB_LATEST=0.2.9 run bash "$REPO/scripts/rk-autobump.sh"
  [ "$status" -eq 0 ]
  [[ "$output" == *"the lock could not be taken"* ]]
  [[ "$output" == *"did not complete"* ]]
  pin_unchanged
}
