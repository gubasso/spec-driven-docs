#!/usr/bin/env sh
set -eu
canon=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/sdd-instantiation.XXXXXX")
scratch=$(cd "$scratch" && pwd -P)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

fail() {
  echo "FAIL $1"
  exit 1
}

install_profile() {
  profile=$1 target=$2
  mkdir -p "$target/.git"
  "$canon/scripts/instantiate.sh" --target "$target" --profile "$profile" >/dev/null
  "$target/.spec-driven-docs/verify.sh" --target "$target" --offline >/dev/null
}

code="$scratch/codebase with spaces"
kb="$scratch/knowledge"
install_profile codebase "$code"
install_profile knowledge-base "$kb"
[ -d "$code/docs/specs" ] && [ -d "$kb/_docs/specs" ]
[ -f "$code/docs/specs/SPEC-distribution.md" ] || fail 'codebase living specs were not seeded'

before=$(find "$code" -type f -not -path '*/.git/*' -exec sha256sum {} + | sort | sha256sum)
"$canon/scripts/instantiate.sh" --target "$code" --profile codebase >/dev/null
after=$(find "$code" -type f -not -path '*/.git/*' -exec sha256sum {} + | sort | sha256sum)
[ "$before" = "$after" ] || fail 'reinstall changed bytes'

printf '%s\n' '# local specification' >>"$code/docs/specs/TEMPLATE-spec.md"
printf '%s\n' '<!-- local adopted rule -->' >>"$code/docs/specs/SPEC-distribution.md"
"$canon/scripts/instantiate.sh" --target "$code" --profile codebase >/dev/null
tail -1 "$code/docs/specs/TEMPLATE-spec.md" | grep -q 'local specification'
tail -1 "$code/docs/specs/SPEC-distribution.md" | grep -q 'local adopted rule'

# An adopted file's recorded hash is its installed bytes, so a reconciled copy
# stops reporting DRIFT rather than reporting it forever.
"$code/.spec-driven-docs/verify.sh" --target "$code" --offline | grep -q '^DRIFT' &&
  fail 'a reconciled adopted file still reports drift'

nonempty="$scratch/nonempty"
mkdir -p "$nonempty"
printf '%s\n' keep >"$nonempty/owned.txt"
"$canon/scripts/instantiate.sh" --target "$nonempty" --profile codebase | grep -q 'DRY RUN'
[ "$(find "$nonempty" -type f | wc -l)" -eq 1 ] || fail 'the forced dry run wrote files'

# The forced dry run has an override, or the canon cannot be adopted into the
# repository it exists to serve.
"$canon/scripts/instantiate.sh" --target "$nonempty" --profile codebase --apply >/dev/null
[ -f "$nonempty/.spec-driven-docs/manifest.json" ] || fail '--apply did not install'
grep -q '^keep$' "$nonempty/owned.txt" || fail '--apply disturbed the project content'
"$nonempty/.spec-driven-docs/verify.sh" --target "$nonempty" --offline >/dev/null

dry="$scratch/dry"
mkdir -p "$dry"
"$canon/scripts/instantiate.sh" --target "$dry" --profile codebase --dry-run >/dev/null
[ -z "$(find "$dry" -mindepth 1 -print -quit)" ] || fail 'an explicit dry run wrote files'

# Every rejection guard, pinned. Each was reachable only by inspection before.
"$canon/scripts/instantiate.sh" --target / --profile codebase >/dev/null 2>&1 && fail 'root target accepted'
"$canon/scripts/instantiate.sh" --target // --profile codebase >/dev/null 2>&1 && fail 'double-slash root target accepted'
"$canon/scripts/instantiate.sh" --target "$canon" --profile codebase >/dev/null 2>&1 && fail 'canon checkout accepted'
"$canon/scripts/instantiate.sh" --target "$canon/method" --profile codebase >/dev/null 2>&1 && fail 'path inside the canon accepted'
"$canon/scripts/instantiate.sh" --target relative --profile codebase >/dev/null 2>&1 && fail 'relative target accepted'
"$canon/scripts/instantiate.sh" --target "$dry" --profile nonexistent >/dev/null 2>&1 && fail 'unknown profile accepted'

# The managed block is spliced into the `repos:` sequence. Appended at end of
# file it lands after any later top-level key and the result is not YAML.
printf '%s\n' '# comment before' 'repos:' '# comment inside' '# BEGIN spec-driven-docs managed' old '# END spec-driven-docs managed' '# comment after' >"$kb/.pre-commit-config.yaml"
"$canon/scripts/instantiate.sh" --target "$kb" --profile knowledge-base >/dev/null
grep -q '^# comment before$' "$kb/.pre-commit-config.yaml"
grep -q '^# comment inside$' "$kb/.pre-commit-config.yaml"
grep -q '^# comment after$' "$kb/.pre-commit-config.yaml"

trailing="$scratch/trailing"
mkdir -p "$trailing/.git"
printf '%s\n' 'repos:' '  - repo: https://github.com/example/hooks' '    rev: v1.0.0' '    hooks:' '      - id: example' 'ci:' '  autofix_prs: false' 'default_language_version:' '  python: python3' >"$trailing/.pre-commit-config.yaml"
"$canon/scripts/instantiate.sh" --target "$trailing" --profile codebase --apply >/dev/null
pre-commit validate-config "$trailing/.pre-commit-config.yaml" >/dev/null ||
  fail 'the installed configuration does not parse'
grep -q '^ci:$' "$trailing/.pre-commit-config.yaml" || fail 'a later top-level key was lost'
awk '/^# END spec-driven-docs managed$/{seen=1} /^ci:$/{if(!seen) bad=1} END{exit bad}' \
  "$trailing/.pre-commit-config.yaml" || fail 'the block landed after a later top-level key'
"$canon/scripts/instantiate.sh" --target "$trailing" --profile codebase >/dev/null
pre-commit validate-config "$trailing/.pre-commit-config.yaml" >/dev/null ||
  fail 'a second install broke the configuration'

# A configuration with no `repos:` key is refused rather than corrupted.
norepos="$scratch/norepos"
mkdir -p "$norepos/.git"
printf '%s\n' 'ci:' '  autofix_prs: false' >"$norepos/.pre-commit-config.yaml"
"$canon/scripts/instantiate.sh" --target "$norepos" --profile codebase --apply >/dev/null 2>&1 &&
  fail 'a configuration with no repos: key was accepted'
grep -q 'BEGIN spec-driven-docs managed' "$norepos/.pre-commit-config.yaml" &&
  fail 'the refused install still edited the configuration'

# Malformed markers are refused before anything is stripped. A lone BEGIN makes
# the strip swallow every line after it.
for shape in lone-begin lone-end reversed duplicate; do
  broken="$scratch/broken-$shape"
  mkdir -p "$broken/.git"
  case "$shape" in
    lone-begin) printf '%s\n' 'repos:' '# BEGIN spec-driven-docs managed' '  - repo: local' 'ci:' '  autofix_prs: false' >"$broken/.pre-commit-config.yaml" ;;
    lone-end) printf '%s\n' 'repos:' '# END spec-driven-docs managed' 'ci:' '  autofix_prs: false' >"$broken/.pre-commit-config.yaml" ;;
    reversed) printf '%s\n' 'repos:' '# END spec-driven-docs managed' '# BEGIN spec-driven-docs managed' >"$broken/.pre-commit-config.yaml" ;;
    duplicate) printf '%s\n' 'repos:' '# BEGIN spec-driven-docs managed' '# END spec-driven-docs managed' '# BEGIN spec-driven-docs managed' '# END spec-driven-docs managed' >"$broken/.pre-commit-config.yaml" ;;
  esac
  expected=$(cat "$broken/.pre-commit-config.yaml")
  "$canon/scripts/instantiate.sh" --target "$broken" --profile codebase --apply >/dev/null 2>&1 &&
    fail "$shape markers accepted"
  [ "$expected" = "$(cat "$broken/.pre-commit-config.yaml")" ] ||
    fail "$shape markers were rewritten by the refused install"
done

# The verifier's own checks, each pinned by tampering it must catch. Reverting
# any one of them leaves a control failing rather than a suite still green.
verify_rejects() {
  label=$1 expected=$2 instance=$3
  # The canon's copy is byte-identical to the installed one, and running it
  # keeps the control usable when the tampering is on the installed verifier's
  # own permissions.
  out=$("$canon/scripts/verify.sh" --target "$instance" --offline 2>&1) &&
    fail "verifier accepted $label"
  case "$out" in
    *"$expected"*) ;;
    *) fail "verifier did not report '$expected' for $label: $out" ;;
  esac
}
fresh_instance() {
  rm -rf "$1"
  mkdir -p "$1/.git"
  "$canon/scripts/instantiate.sh" --target "$1" --profile codebase >/dev/null
}
probe="$scratch/probe"

fresh_instance "$probe"
sed -i 's#entry: .spec-driven-docs/verify.sh --target . --offline#entry: true#' "$probe/.pre-commit-config.yaml"
verify_rejects 'a tampered managed block' 'FAIL managed block tampered' "$probe"

fresh_instance "$probe"
chmod 644 "$probe/.spec-driven-docs/verify.sh"
verify_rejects 'a non-executable entry' 'FAIL managed block entry is not executable' "$probe"
chmod 755 "$probe/.spec-driven-docs/verify.sh"

fresh_instance "$probe"
jq '.managed_files = [1,2,3]' "$probe/.spec-driven-docs/manifest.json" >"$scratch/m.json"
cp "$scratch/m.json" "$probe/.spec-driven-docs/manifest.json"
verify_rejects 'a corrupt managed projection' 'FAIL unreadable manifest projection' "$probe"

fresh_instance "$probe"
jq '.managed_files = []' "$probe/.spec-driven-docs/manifest.json" >"$scratch/m.json"
cp "$scratch/m.json" "$probe/.spec-driven-docs/manifest.json"
verify_rejects 'an empty managed set' 'FAIL invalid manifest schema' "$probe"

fresh_instance "$probe"
jq '.adopted_files = [1,2,3]' "$probe/.spec-driven-docs/manifest.json" >"$scratch/m.json"
cp "$scratch/m.json" "$probe/.spec-driven-docs/manifest.json"
verify_rejects 'a corrupt adopted projection' 'FAIL unreadable manifest projection' "$probe"

fresh_instance "$probe"
cp "$probe/docs/specs/SPEC-docs-specs.md" "$probe/docs/specs/SPEC-duplicate.md"
verify_rejects 'a duplicated rule ID' 'FAIL duplicate rule ID in local specs' "$probe"

fresh_instance "$probe"
jq 'del(.integration_blocks[0].marker_hash) | .integration_blocks[0].path = ".pre-commit-config.yaml"' \
  "$probe/.spec-driven-docs/manifest.json" >"$scratch/m.json"
cp "$scratch/m.json" "$probe/.spec-driven-docs/manifest.json"
verify_rejects 'a manifest recording no marker hash' 'FAIL manifest records no marker hash' "$probe"

# A destination reached through a symlink is outside the target the caller
# named, and following one writes the referent instead.
outside="$scratch/outside"
mkdir -p "$outside/payload"
printf '%s\n' 'repos:' '  - repo: https://github.com/example/hooks' '    rev: v1.0.0' '    hooks:' '      - id: example' >"$outside/config.yaml"
printf '%s\n' 'external payload' >"$outside/payload/keep.txt"
outside_before=$(find "$outside" -type f -exec sha256sum {} + | sort | sha256sum)

symlinked_file="$scratch/symlinked-file"
mkdir -p "$symlinked_file/.git"
ln -s "$outside/config.yaml" "$symlinked_file/.pre-commit-config.yaml"
symlink_out=$("$canon/scripts/instantiate.sh" --target "$symlinked_file" --profile codebase --apply 2>&1) &&
  fail 'a symlinked destination file was accepted'
case "$symlink_out" in
  *'escapes the target through a symlink'*) ;;
  *) fail "a symlinked destination file was refused for the wrong reason: $symlink_out" ;;
esac

symlinked_dir="$scratch/symlinked-dir"
mkdir -p "$symlinked_dir/.git"
ln -s "$outside/payload" "$symlinked_dir/.spec-driven-docs"
symlink_out=$("$canon/scripts/instantiate.sh" --target "$symlinked_dir" --profile codebase --apply 2>&1) &&
  fail 'a symlinked managed directory was accepted'
case "$symlink_out" in
  *'escapes the target through a symlink'*) ;;
  *) fail "a symlinked managed directory was refused for the wrong reason: $symlink_out" ;;
esac

[ "$outside_before" = "$(find "$outside" -type f -exec sha256sum {} + | sort | sha256sum)" ] ||
  fail 'the refused install wrote through a symlink'

# An existing directory at a file destination is refused, not copied into.
blocked="$scratch/blocked"
mkdir -p "$blocked/.git" "$blocked/.pre-commit-config.yaml"
blocked_before=$(find "$blocked" -not -path '*/.git/*' | sort)
blocked_out=$("$canon/scripts/instantiate.sh" --target "$blocked" --profile codebase --apply 2>&1) &&
  fail 'a directory at a file destination was accepted'
case "$blocked_out" in
  *'destination exists and is not a regular file'*) ;;
  *) fail "a directory at a file destination was refused for the wrong reason: $blocked_out" ;;
esac
[ "$blocked_before" = "$(find "$blocked" -not -path '*/.git/*' | sort)" ] ||
  fail 'the refused install changed the target'

# A dangling symlink is an entry, so the target is not empty and the preview is
# forced rather than skipped.
dangling="$scratch/dangling"
mkdir -p "$dangling/.git"
ln -s "$dangling/absent" "$dangling/broken-link"
"$canon/scripts/instantiate.sh" --target "$dangling" --profile codebase | grep -q 'DRY RUN' ||
  fail 'a target holding a dangling symlink was treated as empty'
[ ! -e "$dangling/.spec-driven-docs" ] || fail 'the forced dry run wrote into the target'

# The verifier names a missing tool rather than letting a pipeline swallow it.
fresh_instance "$probe"
tool_missing_rejects() {
  missing=$1
  bin="$scratch/bin-$missing"
  rm -rf "$bin"
  mkdir -p "$bin"
  for t in sh env awk cat cut dirname find grep head jq mktemp printf rm sed sha256sum sort tr uniq wc; do
    [ "$t" = "$missing" ] && continue
    resolved=$(command -v "$t" 2>/dev/null) || continue
    ln -s "$resolved" "$bin/$t"
  done
  out=$(PATH="$bin" "$canon/scripts/verify.sh" --target "$probe" --offline 2>&1) &&
    fail "verifier accepted a missing $missing"
  case "$out" in
    *"FAIL missing required tool: $missing"*) ;;
    *) fail "verifier did not name the missing $missing: $out" ;;
  esac
}
tool_missing_rejects sort
tool_missing_rejects uniq
tool_missing_rejects jq
tool_missing_rejects wc
tool_missing_rejects head
tool_missing_rejects mktemp

if grep -q "${canon}" "$kb/.spec-driven-docs/verify.sh"; then
  fail 'verifier records the canon checkout path'
fi
"$kb/.spec-driven-docs/verify.sh" --target "$kb" --offline >/dev/null
# A projected gate is wired, not merely present. The defect this guards against
# is silent in every other check: the payload lands, the manifest hashes it, the
# verifier passes, and no gate runs, because the managed block named the
# verifier alone. So the block is held against the declaration that defines the
# delivered set, and then a real violation is planted and the hooks are run.
gate_ids=$(jq -r '.gates[].id' "$canon/instance/gates.json")
for id in $gate_ids; do
  grep -q "^ *- id: $id\$" "$kb/.pre-commit-config.yaml" ||
    fail "projected gate is wired to nothing: $id"
done
for id in $gate_ids; do
  script=$(jq -r --arg i "$id" '.gates[] | select(.id==$i) | .script' "$canon/instance/gates.json")
  [ -x "$kb/.spec-driven-docs/hooks/$script" ] ||
    fail "wired gate is not executable in the instance: $script"
done

# The boundary holds in the other direction too: nothing from `gates/canon/`
# reaches the instance.
for file in "$canon"/gates/canon/*; do
  [ -f "$file" ] || continue
  [ ! -e "$kb/.spec-driven-docs/hooks/${file##*/}" ] ||
    fail "a canon-only gate was projected: ${file##*/}"
done

# End to end: the instance rejects a document its gates forbid. `pre-commit` is
# run against the one hook, so the assertion is that this gate ran and failed,
# not that something somewhere in the config did.
if command -v pre-commit >/dev/null 2>&1 && command -v git >/dev/null 2>&1; then
  live="$scratch/live"
  mkdir -p "$live"
  cp -R "$kb/." "$live/"
  rm -rf "$live/.git"
  git -C "$live" init -q
  git -C "$live" add -A
  printf '# Choice\n' >"$live/_docs/decisions/ADR-use-v2.md"
  git -C "$live" add -A
  out=$(cd "$live" && pre-commit run adr-filename-shape --all-files 2>&1) && {
    printf '%s\n' "$out"
    fail 'the projected filename gate accepted a record carrying a digit'
  }
  case "$out" in
    *decision-records:filename-carries-no-digit*) ;;
    *)
      printf '%s\n' "$out"
      fail 'the projected gate failed without reporting its rule'
      ;;
  esac
fi

echo 'OK instantiation controls'
