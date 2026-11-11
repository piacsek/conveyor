#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
link="$HOME/.local/bin/conveyor-dev"

install() {
  echo "$*"
  exit 1
}

[ -x "$link" ] || install "conveyor-dev is missing or dangling"
[ ! "$link" -ef target/release/conveyor ] || install "conveyor-dev is this worktree's build"
base="$(git merge-base HEAD origin/main 2>/dev/null)" || install "no merge base with origin/main"
changed="$(git diff --name-only --no-renames "$base")"
untracked="$(git ls-files --others --exclude-standard)"
input="$(printf '%s\n%s\n' "$changed" "$untracked" | scripts/feeds-binary.sh | sed -n 1p)"
[ -z "$input" ] || install "$input feeds the binary"
echo "nothing on this branch feeds the binary, and conveyor-dev is another worktree's build"
