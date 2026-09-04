#!/usr/bin/env bats
# scripts/rk-bump.sh is the transactional envelope and the lock every caller
# reaches the pin rewrite through. The operation it guards spans two files —
# flake.nix takes the new input URL, flake.lock takes the content pin — so
# what is proven here is the pair's guarantee: however the run dies, both
# files are either fully advanced or byte-identical to what they were, and
# two callers cannot overlap.

setup() {
  REPO="$(mktemp -d)"
  mkdir -p "$REPO/scripts" "$REPO/stub" "$REPO/badlock"
  cp "$BATS_TEST_DIRNAME/../scripts/rk-bump.sh" "$REPO/scripts/rk-bump.sh"

  FLAKE="$REPO/flake.nix"
  LOCKFILE="$REPO/flake.lock"
  cat >"$FLAKE" <<'FLAKE'
{
  inputs = {
    release-kit = {
      url = "github:gubasso/release-kit/v0.2.8";
    };
  };
}
FLAKE
  printf 'lock for v0.2.8\n' >"$LOCKFILE"
  ORIG_FLAKE="$REPO/flake.nix.orig"
  ORIG_LOCK="$REPO/flake.lock.orig"
  cp "$FLAKE" "$ORIG_FLAKE"
  cp "$LOCKFILE" "$ORIG_LOCK"

  LOCK="$REPO/.direnv/rk-bump.lock"
  STUB_PID="$REPO/stub.pid"
  STUB_RELEASE="$REPO/stub.release"
  STUB_CALLED="$REPO/stub.called"
  CURL_CALLED="$REPO/curl.called"

  # Stands in for nix: `flake update` writes the lock from the flake's pin,
  # `eval` answers the system, and `build` is where the failure and hang
  # modes live — after the lock moved, which is the window the envelope
  # exists for.
  cat >"$REPO/stub/nix" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$@" >>"$RK_STUB_CALLED"
case "$1" in
  eval)
    printf 'x86_64-linux'
    ;;
  flake)
    tag="$(grep -oE 'release-kit/v[0-9][0-9.]*' flake.nix | head -n 1 | sed 's|release-kit/||')"
    printf '"ref": "%s"\n' "${RK_STUB_LOCK_REF:-$tag}" >flake.lock
    ;;
  build)
    if [ "${NIX_STUB_MODE:-ok}" = fail ]; then exit 1; fi
    if [ "${NIX_STUB_MODE:-ok}" = hang ]; then
      trap 'exit 143' TERM
      echo $$ >"$RK_STUB_PID"
      while [ ! -e "$RK_STUB_RELEASE" ]; do
        sleep 0.1 >/dev/null 2>&1
      done
    fi
    ;;
esac
exit 0
STUB

  # Stands in for the redirect discovery; records every call so the
  # explicit-argument path can prove it made no request.
  cat >"$REPO/stub/curl" <<'STUB'
#!/usr/bin/env bash
printf 'called\n' >>"$RK_CURL_CALLED"
if [ "${CURL_STUB_MODE:-ok}" = fail ]; then exit 6; fi
printf 'https://github.com/gubasso/release-kit/releases/tag/v0.2.9'
STUB

  # A stubbed flock, used only by the case that needs a non-contention failure.
  cat >"$REPO/badlock/flock" <<'STUB'
#!/usr/bin/env bash
exit 5
STUB
  chmod +x "$REPO/stub/nix" "$REPO/stub/curl" "$REPO/badlock/flock" "$REPO/scripts/rk-bump.sh"

  RK_STUB_PID="$STUB_PID"
  RK_STUB_RELEASE="$STUB_RELEASE"
  RK_STUB_CALLED="$STUB_CALLED"
  RK_CURL_CALLED="$CURL_CALLED"
  export RK_STUB_PID RK_STUB_RELEASE RK_STUB_CALLED RK_CURL_CALLED
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

# Assert the flake moved to exactly the given tag and nothing else changed:
# rewriting the pin back must reproduce the original bytes.
flake_moved_only_to() {
  grep -q "release-kit/$1" "$FLAKE"
  sed "s|release-kit/$1|release-kit/v0.2.8|" "$FLAKE" | cmp -s - "$ORIG_FLAKE"
}

both_files_unchanged() {
  cmp -s "$ORIG_FLAKE" "$FLAKE" && cmp -s "$ORIG_LOCK" "$LOCKFILE"
}

@test "a discovered bump rewrites exactly the pin line and the lock" {
  run bash "$REPO/scripts/rk-bump.sh"
  [ "$status" -eq 0 ]
  flake_moved_only_to v0.2.9
  grep -q '"ref": "v0.2.9"' "$LOCKFILE"
}

@test "a same-version bump writes nothing" {
  run bash "$REPO/scripts/rk-bump.sh" v0.2.8
  [ "$status" -eq 0 ]
  both_files_unchanged
  [ ! -e "$STUB_CALLED" ]
}

@test "a failed tag discovery leaves both files unchanged and names the failure" {
  CURL_STUB_MODE=fail run bash "$REPO/scripts/rk-bump.sh"
  [ "$status" -eq 1 ]
  [[ "$output" == *"could not be discovered"* ]]
  both_files_unchanged
  [ ! -e "$STUB_CALLED" ]
}

@test "a build failure after the lock updated restores both files" {
  NIX_STUB_MODE=fail run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -ne 0 ]
  both_files_unchanged
  [[ "$output" == *"the pin is unchanged"* ]]
}

# The wrapper is started under job control (`set -m`) so it gets its own
# process group and a default SIGINT disposition: without that, a
# non-interactive shell starts an asynchronous child with SIGINT ignored, and
# a shell cannot trap a signal that was ignored when it started. The build is
# then released to succeed, which is what isolates the trap: the pending
# signal runs before `bump_ok=1`, so both files must come back.
@test "an interrupt mid-run restores both files even when the build then succeeds" {
  set -m
  NIX_STUB_MODE=hang bash "$REPO/scripts/rk-bump.sh" v0.2.9 2>"$REPO/err" &
  wrapper=$!
  set +m
  await "$STUB_PID"
  kill -INT "$wrapper"
  : >"$STUB_RELEASE"
  status=0
  wait "$wrapper" || status=$?
  [ "$status" -eq 130 ]
  both_files_unchanged
  grep -q 'the pin is unchanged' "$REPO/err"
}

@test "a terminated build leaves both files byte-identical" {
  NIX_STUB_MODE=hang bash "$REPO/scripts/rk-bump.sh" v0.2.9 2>"$REPO/err" &
  wrapper=$!
  await "$STUB_PID"
  kill -TERM "$wrapper"
  kill -TERM "$(cat "$STUB_PID")"
  status=0
  wait "$wrapper" || status=$?
  [ "$status" -ne 0 ]
  both_files_unchanged
  grep -q 'the pin is unchanged' "$REPO/err"
}

@test "pre-existing uncommitted edits refuse before any mutation" {
  scrubbed_git() {
    env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE -u GIT_PREFIX \
      git -C "$REPO" "$@"
  }
  scrubbed_git init -q
  scrubbed_git add flake.nix flake.lock
  scrubbed_git -c user.email=t@t -c user.name=t commit -q -m seed --no-verify
  printf '# the operator was here\n' >>"$FLAKE"
  run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 1 ]
  [[ "$output" == *"uncommitted changes"* ]]
  grep -q 'the operator was here' "$FLAKE"
  grep -q 'release-kit/v0.2.8' "$FLAKE"
  cmp -s "$ORIG_LOCK" "$LOCKFILE"
  [ ! -e "$STUB_CALLED" ]
}

@test "an explicit tag argument makes no GitHub request" {
  run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 0 ]
  [ ! -e "$CURL_CALLED" ]
  flake_moved_only_to v0.2.9
}

@test "every input shape resolves to the same tag and never a doubled prefix" {
  for form in v0.2.9 0.2.9 https://github.com/gubasso/release-kit/releases/tag/v0.2.9; do
    cp "$ORIG_FLAKE" "$FLAKE"
    cp "$ORIG_LOCK" "$LOCKFILE"
    run bash "$REPO/scripts/rk-bump.sh" "$form"
    [ "$status" -eq 0 ]
    flake_moved_only_to v0.2.9
    ! grep -q 'vv0' "$FLAKE"
  done
}

@test "a malformed argument is refused before any edit" {
  run bash "$REPO/scripts/rk-bump.sh" not-a-version
  [ "$status" -eq 1 ]
  [[ "$output" == *"not a release tag"* ]]
  both_files_unchanged
  [ ! -e "$STUB_CALLED" ]
}

@test "zero or multiple pin lines refuse with the count" {
  printf '{ }\n' >"$FLAKE"
  run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 1 ]
  [[ "$output" == *"found 0"* ]]
  cp "$ORIG_FLAKE" "$FLAKE"
  printf '      url = "github:gubasso/release-kit/v0.2.7";\n' >>"$FLAKE"
  run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 1 ]
  [[ "$output" == *"found 2"* ]]
  cmp -s "$ORIG_LOCK" "$LOCKFILE"
  [ ! -e "$STUB_CALLED" ]
}

@test "the rewrite goes through a temp file, never sed -i" {
  ! grep -qE 'sed +-i' "$REPO/scripts/rk-bump.sh"
  grep -q 'flake.nix.tmp\|"$flake.tmp"' "$REPO/scripts/rk-bump.sh"
}

@test "staged edits refuse exactly as unstaged ones" {
  scrubbed_git() {
    env -u GIT_DIR -u GIT_WORK_TREE -u GIT_INDEX_FILE -u GIT_PREFIX \
      git -C "$REPO" "$@"
  }
  scrubbed_git init -q
  scrubbed_git add flake.nix flake.lock
  scrubbed_git -c user.email=t@t -c user.name=t commit -q -m seed --no-verify
  printf '# staged, not committed\n' >>"$FLAKE"
  scrubbed_git add flake.nix
  run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 1 ]
  [[ "$output" == *"uncommitted changes"* ]]
  grep -q 'staged, not committed' "$FLAKE"
  grep -q 'release-kit/v0.2.8' "$FLAKE"
  [ ! -e "$STUB_CALLED" ]
}

@test "a version named outside the pin assignment neither counts nor takes the rewrite" {
  printf '  # the previous pin was release-kit/v0.2.7\n' >>"$FLAKE"
  cp "$FLAKE" "$ORIG_FLAKE"
  run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 0 ]
  flake_moved_only_to v0.2.9
  grep -q 'release-kit/v0.2.7' "$FLAKE"
}

@test "a pin line that is not a clean assignment refuses rather than guessing" {
  sed 's|url = "github:gubasso/release-kit/v0.2.8";|/* was "github:gubasso/release-kit/v0.2.7" */ url = "github:gubasso/release-kit/v0.2.8";|' "$FLAKE" >"$FLAKE.tmp"
  mv "$FLAKE.tmp" "$FLAKE"
  cp "$FLAKE" "$ORIG_FLAKE"
  run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 1 ]
  [[ "$output" == *"found 0"* ]]
  both_files_unchanged
  [ ! -e "$STUB_CALLED" ]
}

@test "a commented copy of the assignment neither counts nor takes the rewrite" {
  printf '  # url = "github:gubasso/release-kit/v0.2.7";\n' >>"$FLAKE"
  cp "$FLAKE" "$ORIG_FLAKE"
  run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 0 ]
  flake_moved_only_to v0.2.9
  grep -q '# url = "github:gubasso/release-kit/v0.2.7";' "$FLAKE"
}

@test "a decoy the text matcher hits is caught by the lock outcome" {
  cat >"$FLAKE" <<'FLAKE'
{
  inputs = {
    /* the old shape:
    url = "github:gubasso/release-kit/v0.2.8";
    */
    release-kit.url = "github:gubasso/release-kit/v0.2.8";
  };
}
FLAKE
  cp "$FLAKE" "$ORIG_FLAKE"
  # The override stands in for Nix resolving the live, unchanged input
  # while the text rewrite landed inside the block comment.
  RK_STUB_LOCK_REF=v0.2.8 run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -ne 0 ]
  [[ "$output" == *"not the live input"* ]]
  both_files_unchanged
}

@test "a git status that fails refuses before any mutation" {
  mkdir -p "$REPO/gitstub"
  cat >"$REPO/gitstub/git" <<'STUB'
#!/usr/bin/env bash
case "$*" in
  *rev-parse*) exit 0 ;;
  *status*) echo boom >&2; exit 128 ;;
  *) exit 0 ;;
esac
STUB
  chmod +x "$REPO/gitstub/git"
  PATH="$REPO/gitstub:$PATH" run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 1 ]
  [[ "$output" == *"cannot be judged"* ]]
  both_files_unchanged
  [ ! -e "$STUB_CALLED" ]
}

@test "the nix invocations carry the exact load-bearing vectors" {
  run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 0 ]
  cat >"$REPO/expected" <<'ARGS'
flake
update
release-kit
eval
--impure
--raw
--expr
builtins.currentSystem
build
--no-link
.#devShells.x86_64-linux.default
ARGS
  # The whole vectors, so dropping --no-link, updating the wrong input, or
  # building a different attribute cannot drift unnoticed.
  diff "$REPO/expected" "$STUB_CALLED"
}

@test "a lock that cannot be taken is reported rather than passed over" {
  PATH="$REPO/badlock:$PATH" RK_BUMP_NONBLOCK=1 run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 1 ]
  [[ "$output" == *"the lock could not be taken"* ]]
  [ ! -e "$STUB_CALLED" ]
  both_files_unchanged
}

@test "a non-blocking caller declines a lock another caller holds" {
  mkdir -p "$(dirname "$LOCK")"
  exec 8>"$LOCK"
  flock -n 8
  # 8>&- so the child does not inherit the descriptor that owns the lock:
  # flock follows the open file description, and an inherited copy would make
  # the child a holder of the very lock it is meant to contend for.
  RK_BUMP_NONBLOCK=1 run bash "$REPO/scripts/rk-bump.sh" v0.2.9 8>&-
  exec 8>&-
  [ "$status" -eq 0 ]
  [ ! -e "$STUB_CALLED" ]
  both_files_unchanged
}

@test "a blocking caller waits for the lock rather than overlapping" {
  mkdir -p "$(dirname "$LOCK")"
  exec 8>"$LOCK"
  flock -n 8
  bash "$REPO/scripts/rk-bump.sh" v0.2.9 8>&- &
  wrapper=$!
  # It must not have touched the rewrite while the lock is held elsewhere.
  sleep 1
  [ ! -e "$STUB_CALLED" ]
  cmp -s "$ORIG_FLAKE" "$FLAKE"
  exec 8>&-
  status=0
  wait "$wrapper" || status=$?
  [ "$status" -eq 0 ]
  [ -e "$STUB_CALLED" ]
  flake_moved_only_to v0.2.9
}

@test "a missing flock refuses rather than updating unserialized" {
  local path guard=0
  while path="$(command -v flock 2>/dev/null)"; do
    guard=$((guard + 1))
    [ "$guard" -gt 20 ] && break
    PATH="$(printf '%s' "$PATH" | tr ':' '\n' | grep -vx "$(dirname "$path")" | paste -sd: -)"
    export PATH
  done
  run bash "$REPO/scripts/rk-bump.sh" v0.2.9
  [ "$status" -eq 1 ]
  [ ! -e "$STUB_CALLED" ]
  both_files_unchanged
}
