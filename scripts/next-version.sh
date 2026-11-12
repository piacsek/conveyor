#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/fragments.sh
last="$(last_tag)"
pending="$(pending_fragments "$last")"
[ -n "$pending" ] || exit 0
IFS=. read -r major minor patch <<<"${last#v}"
bump=patch
while IFS= read -r file; do
  kind="$(fragment_kind "$file")"
  [ "$kind" != feat ] || bump=minor
done <<<"$pending"
refuse_repeats "$last" "$pending"
if [ "$bump" = minor ]; then
  echo "$major.$((minor + 1)).0"
else
  echo "$major.$minor.$((patch + 1))"
fi
