#!/usr/bin/env bash

RELEASE_ASSETS="conveyor-aarch64-apple-darwin.tar.gz
conveyor-aarch64-apple-darwin.tar.gz.sha256
conveyor-x86_64-apple-darwin.tar.gz
conveyor-x86_64-apple-darwin.tar.gz.sha256
conveyor-x86_64-unknown-linux-gnu.tar.gz
conveyor-x86_64-unknown-linux-gnu.tar.gz.sha256"

release_state() {
  local tag="$1" errors view draft published unexpected asset
  errors="$(mktemp)"
  if ! view="$(gh release view "$tag" --json isDraft,assets --jq '.isDraft, .assets[].name' 2>"$errors")"; then
    if grep -q 'release not found' "$errors"; then
      rm -f "$errors"
      echo missing
      return 0
    fi
    echo "the release of $tag could not be read: $(cat "$errors")" >&2
    rm -f "$errors"
    return 1
  fi
  rm -f "$errors"
  draft="$(head -n 1 <<<"$view")"
  published="$(tail -n +2 <<<"$view")"
  unexpected="$(grep -vxF -e '' -f <(printf '%s\n' "$RELEASE_ASSETS") <<<"$published" | head -n 1 || true)"
  if [ -n "$unexpected" ]; then
    echo "the release of $tag holds $unexpected besides the three tarballs and their .sha256; remove it by hand" >&2
    return 1
  fi
  if [ "$draft" != false ]; then
    echo draft
    return 0
  fi
  while IFS= read -r asset; do
    if ! grep -qxF "$asset" <<<"$published"; then
      echo short
      return 0
    fi
  done <<<"$RELEASE_ASSETS"
  echo complete
}

newer() {
  local a1 a2 a3 b1 b2 b3
  IFS=. read -r a1 a2 a3 <<<"$1"
  IFS=. read -r b1 b2 b3 <<<"$2"
  [ "$a1" -ne "$b1" ] && { [ "$a1" -gt "$b1" ]; return; }
  [ "$a2" -ne "$b2" ] && { [ "$a2" -gt "$b2" ]; return; }
  [ "$a3" -gt "$b3" ]
}

newest_complete_release_above() {
  local current="$1" tags versions version state
  tags="$(gh release list --exclude-drafts --limit 100 --json tagName --jq '.[].tagName')"
  versions="$(sed -n 's/^v\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$/\1/p' <<<"$tags" | sort -t. -k1,1nr -k2,2nr -k3,3nr)"
  while IFS= read -r version; do
    [ -n "$version" ] || continue
    if [ -n "$current" ] && ! newer "$version" "$current"; then
      return 0
    fi
    state="$(release_state "v$version")"
    if [ "$state" = complete ]; then
      echo "version=$version"
      return 0
    fi
  done <<<"$versions"
}
