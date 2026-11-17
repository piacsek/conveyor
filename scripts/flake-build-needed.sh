#!/usr/bin/env bash
set -uo pipefail
base="${1:-}"

decide() {
  echo "build=$1"
  exit 0
}

[ -n "$base" ] || decide true
changed="$(git diff --name-only --no-renames --end-of-options "$base" HEAD 2>/dev/null)" || decide true
inputs="$(printf '%s\n' "$changed" | "$(dirname "$0")/flake-inputs.sh")" || decide true
[ -n "$inputs" ] && decide true
decide false
