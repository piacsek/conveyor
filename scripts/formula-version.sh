#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/releases.sh

errors="$(mktemp)"
trap 'rm -f "$errors"' EXIT
current=""
if text="$(gh api repos/piacsek/homebrew-tap/contents/Formula/conveyor.rb -H 'Accept: application/vnd.github.raw+json' 2>"$errors")"; then
  current="$(sed -n 's|.*releases/download/v\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)/.*|\1|p' <<<"$text" | head -n 1)"
  if [ -z "$current" ]; then
    echo "Formula/conveyor.rb in piacsek/homebrew-tap names no release version; fix it by hand" >&2
    exit 1
  fi
elif ! grep -q 'HTTP 404' "$errors"; then
  echo "the tap formula could not be read: $(cat "$errors")" >&2
  exit 1
fi

newest_complete_release_above "$current"
