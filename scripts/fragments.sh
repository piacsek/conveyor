#!/usr/bin/env bash

last_tag() {
  local tag
  tag="$(git tag --list 'v*' --sort=-v:refname | awk '/^v[0-9]+\.[0-9]+\.[0-9]+$/ && !found { print; found = 1 }')"
  if [ -z "$tag" ]; then
    echo "no vX.Y.Z tag to count from; fetch the tags (git fetch --tags)" >&2
    return 1
  fi
  echo "$tag"
}

fragments_at() {
  git -c core.quotePath=false ls-tree --name-only "$1" changes/ |
    { grep -E '^changes/[^/]+\.md$' || true; } |
    LC_ALL=C sort
}

refuse_strays() {
  local listing stray
  listing="$(git -c core.quotePath=false ls-tree -r --name-only HEAD -- changes/)" || return 1
  stray="$(printf '%s\n' "$listing" | awk '$0 != "" && !/^changes\/[^\/]+\.md$/ { print; exit }')"
  if [ -n "$stray" ]; then
    echo "$stray is not a fragment: a fragment is a file changes/<name>.md" >&2
    return 1
  fi
}

pending_fragments() {
  refuse_strays || return 1
  LC_ALL=C comm -23 <(fragments_at HEAD) <(fragments_at "$1")
}

fragment_bullets() {
  git show "HEAD:$1" | awk '
    NR == 1 { next }
    /^- / && $0 !~ /^- [ \t]*$/ { print; seen = 1; next }
    /^[ \t]+[^ \t]/ && seen { print }
  '
}

fragment_kind() {
  local kind
  kind="$(git show "HEAD:$1" | sed -n '1s/^kind: //p')"
  case "$kind" in
    internal) ;;
    fix | feat)
      if [ -z "$(fragment_bullets "$1")" ]; then
        echo "$1 is a \`$kind\` without a \`- \` bullet" >&2
        return 1
      fi
      ;;
    *)
      echo "$1 must start with \`kind: fix\`, \`kind: feat\` or \`kind: internal\`" >&2
      return 1
      ;;
  esac
  echo "$kind"
}
