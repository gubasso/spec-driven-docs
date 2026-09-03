#!/usr/bin/env bats
# scripts/rk-bump.sh is the transactional envelope and the lock every caller
# reaches the updater through, so what is proven here is the guarantee that
# motivated it: whatever nix-update leaves behind, and however the wrapper
# dies, the pin on disk is either fully advanced or byte-identical to what it
# was, and two callers cannot overlap.

setup() {
  REPO="$(mktemp -d)"
  mkdir -p "$REPO/scripts" "$REPO/nix" "$REPO/stub" "$REPO/badlock"
  cp "$BATS_TEST_DIRNAME/../scripts/rk-bump.sh" "$REPO/scripts/rk-bump.sh"

  PIN="$REPO/nix/rk.nix"
  ORIG="$REPO/nix/rk.nix.orig"
  printf 'version = "0.2.8";\nhash = "sha256-old";\n' >"$PIN"
  cp "$PIN" "$ORIG"

  LOCK="$REPO/.direnv/rk-bump.lock"
  STUB_PID="$REPO/stub.pid"
  STUB_RELEASE="$REPO/stub.release"
  STUB_CALLED="$REPO/stub.called"

  # Stands in for `nix run nixpkgs#nix-update`, reproducing the ordering that
  # makes the envelope necessary: the version is written before either hash is
  # resolved, so a run that dies in between leaves the file inconsistent.
  cat >"$REPO/stub/nix" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$@" >"$RK_STUB_CALLED"
printf 'version = "0.2.9";\nhash = "sha256-old";\n' >"$RK_PIN"
if [ "${NIX_STUB_MODE:-ok}" = fail ]; then exit 1; fi
if [ "${NIX_STUB_MODE:-ok}" = hang ]; then
  # The sleeps get their own stdio: an orphan holding the harness's output
  # pipe would keep the test alive long after the processes it watches exit.
  trap 'exit 143' TERM
  echo $$ >"$RK_STUB_PID"
  # Held until the test either terminates this stub or releases it to finish
  # successfully. The second is what isolates the wrapper's pending trap.
  while [ ! -e "$RK_STUB_RELEASE" ]; do
    sleep 0.1 >/dev/null 2>&1
  done
fi
printf 'version = "0.2.9";\nhash = "sha256-new";\n' >"$RK_PIN"
STUB
  chmod +x "$REPO/stub/nix"

  # A stubbed flock, used only by the case that needs a non-contention failure.
  cat >"$REPO/badlock/flock" <<'STUB'
#!/usr/bin/env bash
exit 5
STUB
  chmod +x "$REPO/badlock/flock"

  RK_PIN="$PIN"
  RK_STUB_PID="$STUB_PID"
  RK_STUB_RELEASE="$STUB_RELEASE"
  RK_STUB_CALLED="$STUB_CALLED"
  export RK_PIN RK_STUB_PID RK_STUB_RELEASE RK_STUB_CALLED
  PATH="$REPO/stub:$PATH"
  export PATH
}

teardown() {
  [ -e "$STUB_PID" ] && kill "$(cat "$STUB_PID")" 2>/dev/null
  rm -rf "$REPO"
  return 0
}

await() {
  local waited=0
  while [ ! -e "$1" ]; do
    sleep 0.1
    waited=$((waited + 1))
    [ "$waited" -lt 150 ] || return 1
  done
}

# Interrupt the wrapper while it waits on the updater. Bash defers a trapped
# signal until the foreground command returns, so the updater is released too,
# exactly as a terminal interrupt would release the whole process group.
signal_wrapper() {
  local wrapper=$1 signal=$2
  await "$STUB_PID"
  kill -"$signal" "$wrapper"
  kill -TERM "$(cat "$STUB_PID")"
}

@test "a successful update keeps the advanced pin and leaves no snapshot" {
  run bash "$REPO/scripts/rk-bump.sh"
  [ "$status" -eq 0 ]
  grep -q 'sha256-new' "$PIN"
  grep -q '0.2.9' "$PIN"
}

@test "a failed update restores the pin byte-identically" {
  NIX_STUB_MODE=fail run bash "$REPO/scripts/rk-bump.sh"
  [ "$status" -ne 0 ]
  cmp -s "$ORIG" "$PIN"
}

@test "the half-written state the envelope guards against is reachable" {
  NIX_STUB_MODE=fail run env RK_PIN="$PIN" RK_STUB_CALLED="$STUB_CALLED" bash -c 'nix || true'
  grep -q '0.2.9' "$PIN"
  grep -q 'sha256-old' "$PIN"
}

# The wrapper is started under job control (`set -m`) so it gets its own
# process group and a default SIGINT disposition: without that, a
# non-interactive shell starts an asynchronous child with SIGINT ignored, and
# a shell cannot trap a signal that was ignored when it started.
#
# The updater is then released to succeed. That is what isolates the trap:
# with `trap 'exit 130' INT` the pending signal runs before `bump_ok=1` and
# the pin comes back; without it the successful update would stand.
@test "an interrupt restores the pin even when the updater then succeeds" {
  set -m
  NIX_STUB_MODE=hang bash "$REPO/scripts/rk-bump.sh" 2>"$REPO/err" &
  wrapper=$!
  set +m
  await "$STUB_PID"
  kill -INT "$wrapper"
  : >"$STUB_RELEASE"
  status=0
  wait "$wrapper" || status=$?
  [ "$status" -eq 130 ]
  cmp -s "$ORIG" "$PIN"
  grep -q 'the pin is unchanged' "$REPO/err"
}

# What this one holds is restoration, not an exit code: when the updater dies
# first, `set -e` can exit the wrapper before a pending trap runs.
@test "a terminated updater leaves the pin byte-identical" {
  NIX_STUB_MODE=hang bash "$REPO/scripts/rk-bump.sh" 2>"$REPO/err" &
  wrapper=$!
  signal_wrapper "$wrapper" TERM
  status=0
  wait "$wrapper" || status=$?
  [ "$status" -ne 0 ]
  cmp -s "$ORIG" "$PIN"
  grep -q 'the pin is unchanged' "$REPO/err"
}

@test "the updater is invoked as the exact command the pin depends on" {
  run bash "$REPO/scripts/rk-bump.sh"
  [ "$status" -eq 0 ]
  cat >"$REPO/expected" <<'ARGS'
run
nixpkgs#nix-update
--
--flake
--build
--url
https://github.com/gubasso/release-kit
--use-github-releases
--version=stable
release-kit
ARGS
  # The whole vector, so neither the application nor the `--` boundary that
  # decides which side a flag lands on can drift unnoticed.
  diff "$REPO/expected" "$STUB_CALLED"
}

@test "a lock that cannot be taken is reported rather than passed over" {
  PATH="$REPO/badlock:$PATH" RK_BUMP_NONBLOCK=1 run bash "$REPO/scripts/rk-bump.sh"
  [ "$status" -eq 1 ]
  [[ "$output" == *"the lock could not be taken"* ]]
  [ ! -e "$STUB_CALLED" ]
  cmp -s "$ORIG" "$PIN"
}

@test "a non-blocking caller declines a lock another caller holds" {
  mkdir -p "$(dirname "$LOCK")"
  exec 8>"$LOCK"
  flock -n 8
  # 8>&- so the child does not inherit the descriptor that owns the lock:
  # flock follows the open file description, and an inherited copy would make
  # the child a holder of the very lock it is meant to contend for.
  RK_BUMP_NONBLOCK=1 run bash "$REPO/scripts/rk-bump.sh" 8>&-
  exec 8>&-
  [ "$status" -eq 0 ]
  [ ! -e "$STUB_CALLED" ]
  cmp -s "$ORIG" "$PIN"
}

@test "a blocking caller waits for the lock rather than overlapping" {
  mkdir -p "$(dirname "$LOCK")"
  exec 8>"$LOCK"
  flock -n 8
  bash "$REPO/scripts/rk-bump.sh" 8>&- &
  wrapper=$!
  # It must not have touched the updater while the lock is held elsewhere.
  sleep 1
  [ ! -e "$STUB_CALLED" ]
  exec 8>&-
  status=0
  wait "$wrapper" || status=$?
  [ "$status" -eq 0 ]
  [ -e "$STUB_CALLED" ]
  grep -q 'sha256-new' "$PIN"
}

@test "a missing flock refuses rather than updating unserialized" {
  local path guard=0
  while path="$(command -v flock 2>/dev/null)"; do
    guard=$((guard + 1))
    [ "$guard" -gt 20 ] && break
    PATH="$(printf '%s' "$PATH" | tr ':' '\n' | grep -vx "$(dirname "$path")" | paste -sd: -)"
    export PATH
  done
  run bash "$REPO/scripts/rk-bump.sh"
  [ "$status" -eq 1 ]
  [ ! -e "$STUB_CALLED" ]
  cmp -s "$ORIG" "$PIN"
}
