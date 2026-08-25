#!/usr/bin/env sh
set -eu
canon=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/sdd-upgrade.XXXXXX")
scratch=$(cd "$scratch" && pwd -P)
trap 'chmod -R u+w "$scratch" 2>/dev/null || true; rm -rf "$scratch"' EXIT HUP INT TERM

fail() {
  echo "FAIL $1"
  exit 1
}

# A real repository's `.git` holds loose objects at mode 444. Copying the whole
# target through a staging directory dies on the first of them and leaves the
# upgrade half applied, so the fixture carries one.
seed_git() {
  mkdir -p "$1/.git/objects/ab" "$1/.git/refs/heads"
  printf 'ref: refs/heads/develop\n' >"$1/.git/HEAD"
  printf '[remote "origin"]\n\turl = git@example.invalid:owner/repo.git\n' >"$1/.git/config"
  printf 'loose object bytes\n' >"$1/.git/objects/ab/cdef"
  chmod 444 "$1/.git/objects/ab/cdef"
}
git_state() {
  find "$1/.git" -type f -exec sha256sum {} + | sort | sha256sum
}

target="$scratch/target"
mkdir -p "$target"
seed_git "$target"
"$canon/scripts/instantiate.sh" --target "$target" --profile codebase >/dev/null

# The step out of the installed version is whatever the canon currently
# releases, so the control reads it rather than restating it and going stale on
# the next bump.
from_version=$(cat "$canon/VERSION")
next="$scratch/canon next"
cp -R "$canon" "$next"
printf '%s\n' 0.2.0 >"$next/VERSION"
printf '%s\n' "# Migration from $from_version to 0.2.0" \
  >"$next/migrations/$from_version-to-0.2.0.md"
printf '%s\n' '# managed change' >>"$next/gates/instance/adr-word-cap.sh"

before=$(find "$target" -type f -not -path '*/.git/*' -exec sha256sum {} + | sort | sha256sum)
"$next/scripts/upgrade.sh" --target "$target" --from "$next" --dry-run >/dev/null
after=$(find "$target" -type f -not -path '*/.git/*' -exec sha256sum {} + | sort | sha256sum)
[ "$before" = "$after" ] || fail 'upgrade dry-run changed bytes'

# A `--from` path carrying a space names its guides correctly, or the operator
# is told to read migrations that do not exist.
"$next/scripts/upgrade.sh" --target "$target" --from "$next" --dry-run |
  grep -q "^consult migration: $from_version-to-0.2.0.md\$" || fail 'the migration list word-split'
[ "$("$next/scripts/upgrade.sh" --target "$target" --from "$next" --dry-run | grep -c '^consult migration:')" -eq 1 ] ||
  fail 'the migration list named more guides than exist'

printf '%s\n' '# local edit' >>"$target/.spec-driven-docs/hooks/adr-word-cap.sh"
printf '%s\n' '# second local edit' >>"$target/.spec-driven-docs/hooks/agents-digest-size.sh"
conflict_before=$(find "$target" -type f -not -path '*/.git/*' -exec sha256sum {} + | sort | sha256sum)
conflicts=$("$next/scripts/upgrade.sh" --target "$target" --from "$next" 2>&1) && fail 'conflict accepted'
[ "$(printf '%s\n' "$conflicts" | grep -c '^CONFLICT')" -eq 2 ] ||
  fail 'the upgrade reported one conflict at a time'
conflict_after=$(find "$target" -type f -not -path '*/.git/*' -exec sha256sum {} + | sort | sha256sum)
[ "$conflict_before" = "$conflict_after" ] || fail 'conflict changed target'

rm -rf "$target"
mkdir -p "$target"
seed_git "$target"
"$canon/scripts/instantiate.sh" --target "$target" --profile codebase >/dev/null
printf '%s\n' '# local living rule' >>"$target/docs/specs/TEMPLATE-spec.md"
git_before=$(git_state "$target")
"$next/scripts/upgrade.sh" --target "$target" --from "$next" >/dev/null
tail -1 "$target/docs/specs/TEMPLATE-spec.md" | grep -q 'local living rule'
jq -e '.canon_version == "0.2.0"' "$target/.spec-driven-docs/manifest.json" >/dev/null
[ "$git_before" = "$(git_state "$target")" ] || fail 'the upgrade rewrote .git'

# A reinstall that cannot proceed reports why. Discarding the installer's output
# leaves the operator with an exit code and nothing else.
collide="$scratch/collide"
mkdir -p "$collide"
seed_git "$collide"
"$canon/scripts/instantiate.sh" --target "$collide" --profile codebase >/dev/null
# A destination the new release introduces, blocked by a directory the target
# already holds. The conflict scan cannot see it: the installed manifest does
# not list a file that did not exist when the instance was installed.
printf '#!/usr/bin/env sh\nexit 0\n' >"$next/gates/instance/new-gate.sh"
chmod 755 "$next/gates/instance/new-gate.sh"
mkdir -p "$collide/.spec-driven-docs/hooks/new-gate.sh"
collide_out=$("$next/scripts/upgrade.sh" --target "$collide" --from "$next" 2>&1) &&
  fail 'a colliding destination was accepted'
case "$collide_out" in
  *'destination exists and is not a regular file'*) ;;
  *) fail "the reinstall failure was reported without its reason: $collide_out" ;;
esac
[ -d "$collide/.spec-driven-docs/hooks/new-gate.sh" ] ||
  fail 'the refused upgrade changed the colliding destination'
rm -f "$next/gates/instance/new-gate.sh"

final="$scratch/canon-final"
cp -R "$next" "$final"
printf '%s\n' 0.3.0 >"$final/VERSION"
printf '%s\n' '# Migration from 0.2.0 to 0.3.0' >"$final/migrations/0.2.0-to-0.3.0.md"
"$final/scripts/upgrade.sh" --target "$target" --from "$final" | grep -q '0.2.0-to-0.3.0'
echo 'OK upgrade controls'
