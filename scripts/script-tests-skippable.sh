#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
read_by_the_script_tests='^(scripts|tests/scripts|tests/fixtures|\.github|changes)/|^(Cargo\.toml|Cargo\.lock|CHANGELOG\.md)$'

run() {
  echo "$*"
  exit 1
}

base="$(git merge-base HEAD origin/main 2>/dev/null)" || run "no merge base with origin/main"
changed="$(git diff --name-only --no-renames "$base")"
untracked="$(git ls-files --others --exclude-standard)"
needed="$(printf '%s\n%s\n' "$changed" "$untracked" | { grep -E "$read_by_the_script_tests" || true; } | sed -n 1p)"
[ -z "$needed" ] || run "$needed needs the script tests"
echo "nothing on this branch reaches the script tests"
