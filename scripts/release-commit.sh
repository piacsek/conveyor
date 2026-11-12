#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/fragments.sh
version="${1:-}"
number='(0|[1-9][0-9]*)'
if ! [[ "$version" =~ ^$number\.$number\.$number$ ]]; then
  echo "usage: scripts/release-commit.sh X.Y.Z" >&2
  exit 1
fi
last="$(last_tag)"
if ! awk -v next_version="$version" -v last="${last#v}" 'BEGIN {
  split(next_version, a, ".")
  split(last, b, ".")
  for (i = 1; i <= 3; i++) {
    if (a[i] + 0 > b[i] + 0) exit 0
    if (a[i] + 0 < b[i] + 0) exit 1
  }
  exit 1
}'; then
  echo "$version is not above the last release $last" >&2
  exit 1
fi
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
  /^[ \t]*\[/ { package = ($0 == "[package]") }
  package && /^version = / { $0 = "version = \"" version "\"" }
  { print }
' Cargo.toml)"
lock="$(awk -v version="$version" '
  entry && /^version = / { $0 = "version = \"" version "\"" }
  { entry = ($0 == "name = \"conveyor\""); print }
' Cargo.lock)"
if ! printf '%s\n' "$manifest" | awk -v line="version = \"$version\"" '
  /^[ \t]*\[/ { package = ($0 == "[package]") }
  package && $0 == line { found = 1 }
  END { exit !found }
'; then
  echo "Cargo.toml: the version of [package] does not read $version after the rewrite" >&2
  exit 1
fi
if ! printf '%s\n' "$lock" | awk -v line="version = \"$version\"" '
  entry && $0 == line { found = 1 }
  { entry = ($0 == "name = \"conveyor\"") }
  END { exit !found }
'; then
  echo "Cargo.lock: the version of the conveyor entry does not read $version after the rewrite" >&2
  exit 1
fi
printf '%s\n' "$manifest" >Cargo.toml
printf '%s\n' "$lock" >Cargo.lock
printf '# Changelog\n\n## v%s (%s)\n%s\n\n%s\n' "$version" "$(date -u +%Y-%m-%d)" "${bullets:-- v$version: internal changes only.}" "$released" >CHANGELOG.md
