#!/usr/bin/env bash
set -uo pipefail
base="${1:-}"

decide() {
  echo "build=$1"
  exit 0
}

undecided() {
  echo "flake-build-needed: $1, so the flake builds" >&2
  decide true
}

[ -n "$base" ] || undecided "no base commit was given"
inputs="$(git diff -z --name-only --no-renames --end-of-options "$base" HEAD 2>/dev/null |
  "$(dirname "$0")/flake-inputs.sh" | tr '\0' '\n')" ||
  undecided "could not list the paths changed since $base"
[ -n "$inputs" ] && decide true
decide false
