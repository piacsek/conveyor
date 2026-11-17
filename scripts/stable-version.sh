#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/releases.sh

errors="$(mktemp)"
trap 'rm -f "$errors"' EXIT
current=""
if manifest="$(gh api 'repos/{owner}/{repo}/contents/Cargo.toml?ref=stable' -H 'Accept: application/vnd.github.raw+json' 2>"$errors")"; then
  current="$(grep -m1 '^version = ' <<<"$manifest" | cut -d'"' -f2 || true)"
  if ! grep -qxE '[0-9]+\.[0-9]+\.[0-9]+' <<<"$current"; then
    echo "Cargo.toml on the stable branch names no version; delete the stable branch and the next release run recreates it at the newest release" >&2
    exit 1
  fi
elif ! grep -q 'HTTP 404' "$errors"; then
  echo "the stable branch could not be read: $(cat "$errors")" >&2
  exit 1
fi

newest_complete_release_above "$current"
