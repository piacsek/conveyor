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
