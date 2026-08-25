#!/usr/bin/env sh
# The delivered gate set is one set, declared once, and reaches both deliveries.
#
# A gate leaves this repository by two routes: projected into an instance by
# `scripts/instantiate.sh`, and published to a consumer who installs this
# repository as a pre-commit repo through `.pre-commit-hooks.yaml`. Those routes
# drifted apart silently -- a gate can sit in the payload, hashed and
# executable, wired into neither -- and a gate that never runs is
# indistinguishable from a gate that passes.
#
# So `instance/gates.json` is the declaration and this check holds three things
# against it: the scripts on disk, the published manifest, and the boundary with
# `gates/canon/`. A gate here checks an invariant only this repository has -- its
# VERSION against its own tag, its dual licence, its own negative controls --
# and delivering one would gate an instance on a release process it does not run.
set -eu

decl=instance/gates.json
published=.pre-commit-hooks.yaml
[ -f "$decl" ] || {
  echo "FAIL release:the-delivered-gate-set-is-declared-once $decl: missing"
  exit 1
}
[ -f "$published" ] || {
  echo "FAIL release:the-delivered-gate-set-is-declared-once $published: missing"
  exit 1
}

work=$(mktemp -d "${TMPDIR:-/tmp}/sdd-domain.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
status=0

script_root=$(jq -r '.script_root' "$decl")
jq -r '.gates[].script' "$decl" | sort >"$work/declared-scripts"
jq -r '.gates[].id' "$decl" | sort >"$work/declared-ids"
jq -r '.support[]' "$decl" | sort >"$work/declared-support"
sort "$work/declared-scripts" "$work/declared-support" >"$work/declared-files"

# Every declared gate is a script that is there and can run. A declaration
# naming a file the payload does not hold projects a manifest entry that no
# install can satisfy.
while IFS= read -r script; do
  [ -f "$script_root/$script" ] || {
    echo "FAIL release:the-delivered-gate-set-is-declared-once $script_root/$script: declared but absent"
    status=1
    continue
  }
  [ -x "$script_root/$script" ] || {
    echo "FAIL release:the-delivered-gate-set-is-declared-once $script_root/$script: not executable"
    status=1
  }
done <"$work/declared-scripts"

# And the reverse: a script sitting in the delivered directory that nothing
# declares is the shape of the original defect -- payload with no wiring.
for file in "$script_root"/*; do
  [ -f "$file" ] || continue
  name=${file##*/}
  grep -Fxq "$name" "$work/declared-files" || {
    echo "FAIL release:the-delivered-gate-set-is-declared-once $file: present but undeclared"
    status=1
  }
done

# The published manifest names the same ids, and names them at the delivered
# path. An entry pointing into `gates/canon/` would ship a canon-only gate to
# every consumer who installs this repository by rev.
grep -E '^- id: ' "$published" | sed 's/^- id: //' | sort >"$work/published-ids"
missing=$(comm -23 "$work/declared-ids" "$work/published-ids")
[ -z "$missing" ] || {
  echo 'FAIL release:the-delivered-gate-set-is-declared-once .pre-commit-hooks.yaml: declared but not published'
  printf '%s\n' "$missing" | sed 's/^/  /'
  status=1
}
extra=$(comm -13 "$work/declared-ids" "$work/published-ids")
[ -z "$extra" ] || {
  echo 'FAIL release:the-delivered-gate-set-is-declared-once .pre-commit-hooks.yaml: published but not declared'
  printf '%s\n' "$extra" | sed 's/^/  /'
  status=1
}

sed -n 's/^  entry: //p' "$published" | cut -d' ' -f1 >"$work/published-entries"
while IFS= read -r entry; do
  case "$entry" in
    "$script_root"/*) ;;
    *)
      echo "FAIL release:a-canon-gate-is-not-delivered $published: $entry is published from outside $script_root"
      status=1
      ;;
  esac
done <"$work/published-entries"

# The boundary, stated as its own assertion rather than inferred from the two
# above: nothing under `gates/canon/` is declared or published.
for file in gates/canon/*; do
  [ -f "$file" ] || continue
  name=${file##*/}
  if grep -Fxq "$name" "$work/declared-files"; then
    echo "FAIL release:a-canon-gate-is-not-delivered $file: a canon gate is in the delivered declaration"
    status=1
  fi
  grep -Fq "gates/canon/$name" "$published" && {
    echo "FAIL release:a-canon-gate-is-not-delivered $file: a canon gate is published to consumers"
    status=1
  }
done

exit "$status"
