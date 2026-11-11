#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/fragments.sh
version="${1:-}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "usage: scripts/release-commit.sh X.Y.Z" >&2
  exit 1
fi
last="$(last_tag)"
pending="$(pending_fragments "$last")"
if [ -z "$pending" ]; then
  echo "no fragment in changes/ is pending since $last; nothing to release" >&2
  exit 1
fi
bullets="$(
  while IFS= read -r file; do
    fragment_kind "$file" >/dev/null || exit 1
    fragment_bullets "$file"
  done <<<"$pending"
)"
released="$(git show "$last:CHANGELOG.md" | awk '/^## / { found = 1 } found')"
manifest="$(awk -v version="$version" '
  /^\[/ { package = ($0 == "[package]") }
  package && /^version = / { $0 = "version = \"" version "\"" }
  { print }
' Cargo.toml)"
lock="$(awk -v version="$version" '
  entry && /^version = / { $0 = "version = \"" version "\"" }
  { entry = ($0 == "name = \"conveyor\""); print }
' Cargo.lock)"
printf '%s\n' "$manifest" >Cargo.toml
printf '%s\n' "$lock" >Cargo.lock
printf '# Changelog\n\n## v%s (%s)\n%s\n\n%s\n' "$version" "$(date -u +%Y-%m-%d)" "${bullets:-- Internal changes only.}" "$released" >CHANGELOG.md
