#!/usr/bin/env bash
set -euo pipefail

tag="${1:?usage: scripts/previous-tag.sh vX.Y.Z}"
errors="$(mktemp)"
if ! names="$(gh api --paginate 'repos/{owner}/{repo}/tags' --jq '.[].name' 2>"$errors")"; then
  echo "the tags below $tag could not be read: $(cat "$errors")" >&2
  rm -f "$errors"
  exit 1
fi
rm -f "$errors"
previous="$(
  { grep -xE 'v[0-9]+\.[0-9]+\.[0-9]+' <<<"$names" || true; echo "$tag"; } \
    | sort -u -t. -k1.2,1n -k2,2n -k3,3n \
    | awk -v tag="$tag" '$0 == tag { print prev; exit } { prev = $0 }'
)"
echo "previous_tag=$previous"
