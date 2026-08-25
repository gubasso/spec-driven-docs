#!/usr/bin/env sh
# Where an instance keeps the documents these gates read.
#
# An instance names its documentation root in the manifest the installer writes,
# so a codebase instance keeping records under `docs/` is gated exactly like a
# knowledge base keeping them under `_docs/`. Assuming one root is what turns
# these gates into green lights over nothing.
#
# A consumer who installed the published hooks without the payload has no
# manifest, so the root is discovered instead: every root that actually holds
# the zone is searched, and a repository holding none is the only case that
# resolves to nothing.
#
# Arguments win: a repository whose records live outside the documentation root
# passes the directories holding them, which is how this repository gates the
# single record it keeps as a fixture.
#
# Each gate sources this file as `$(dirname "$0")/lib-instance-paths.sh`, so a
# gate runs from the directory it was installed into. Invoking one through a
# symlink placed elsewhere is not supported.

sdd_docs_root() {
  if [ -f .spec-driven-docs/manifest.json ]; then
    sdd_resolved=$(sed -n 's/^[[:space:]]*"docs_root"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
      .spec-driven-docs/manifest.json | head -1)
    if [ -n "$sdd_resolved" ]; then
      printf '%s\n' "$sdd_resolved"
      return 0
    fi
  fi
  for sdd_candidate in _docs docs; do
    if [ -d "$sdd_candidate/specs" ]; then
      printf '%s\n' "$sdd_candidate"
      return 0
    fi
  done
  printf '%s\n' _docs
}

ki_record_roots() {
  if [ "$#" -gt 0 ]; then
    printf '%s\n' "$@"
    return 0
  fi
  if [ -f .spec-driven-docs/manifest.json ]; then
    printf '%s\n' "$(sdd_docs_root)/reference/known-issues"
    return 0
  fi
  for sdd_candidate in _docs docs; do
    if [ -d "$sdd_candidate/reference/known-issues" ]; then
      printf '%s\n' "$sdd_candidate/reference/known-issues"
    fi
  done
  return 0
}

ki_records() {
  ki_record_roots "$@" | while IFS= read -r ki_root; do
    [ -d "$ki_root" ] || continue
    for ki_file in "$ki_root"/KI-?*.md; do
      [ -f "$ki_file" ] || continue
      printf '%s\n' "$ki_file"
    done
  done
}
