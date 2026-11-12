#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/releases.sh

newer() {
  local a1 a2 a3 b1 b2 b3
  IFS=. read -r a1 a2 a3 <<<"$1"
  IFS=. read -r b1 b2 b3 <<<"$2"
  [ "$a1" -ne "$b1" ] && { [ "$a1" -gt "$b1" ]; return; }
  [ "$a2" -ne "$b2" ] && { [ "$a2" -gt "$b2" ]; return; }
  [ "$a3" -gt "$b3" ]
}

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

tags="$(gh release list --exclude-drafts --limit 100 --json tagName --jq '.[].tagName')"
versions="$(sed -n 's/^v\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$/\1/p' <<<"$tags" | sort -t. -k1,1nr -k2,2nr -k3,3nr)"
while IFS= read -r version; do
  [ -n "$version" ] || continue
  if [ -n "$current" ] && ! newer "$version" "$current"; then
    exit 0
  fi
  state="$(release_state "v$version")"
  if [ "$state" = complete ]; then
    echo "version=$version"
    exit 0
  fi
done <<<"$versions"
