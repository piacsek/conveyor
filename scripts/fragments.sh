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

refuse_strays() {
  local listing stray
  listing="$(git -c core.quotePath=false ls-tree -r --name-only HEAD -- changes/)" || return 1
  stray="$(printf '%s\n' "$listing" | awk '$0 != "" && !/^changes\/[^\/]+\.md$/ { print; exit }')"
  if [ -n "$stray" ]; then
    echo "$stray is not a fragment: a fragment is a file changes/<name>.md" >&2
    return 1
  fi
}

refuse_edits() {
  local edited
  edited="$(git -c core.quotePath=false diff-tree -r --name-only --no-renames --diff-filter=M "$1" HEAD -- changes/)" || return 1
  edited="${edited%%$'\n'*}"
  if [ -n "$edited" ]; then
    echo "$edited was released in $1 and has changed since; a released fragment never changes, so put the change in a new file" >&2
    return 1
  fi
}

pending_fragments() {
  local added
  refuse_strays || return 1
  refuse_edits "$1" || return 1
  added="$(git -c core.quotePath=false diff-tree -r --name-only --no-renames --diff-filter=A "$1" HEAD -- changes/)" || return 1
  [ -z "$added" ] || printf '%s\n' "$added" | LC_ALL=C sort
}

fragment_bullets() {
  git show "HEAD:$1" | awk '
    { sub(/\r$/, "") }
    NR == 1 { next }
    /^- / && $0 !~ /^- [ \t]*$/ { print; seen = 1; next }
    /^[ \t]+[^ \t]/ && seen { print }
  '
}

joined_bullets() {
  awk -v sectioned="$1" '
    function flush() {
      if (bullet != "") print version "\t" bullet
      bullet = ""
    }
    function trimmed(text) {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", text)
      return text
    }
    { sub(/\r$/, "") }
    /^## / {
      flush()
      version = substr($0, 4)
      sub(/^v/, "", version)
      sub(/[[:space:]].*$/, "", version)
      sectioned = 1
      next
    }
    !sectioned { next }
    /^- / && trimmed(substr($0, 3)) != "" {
      flush()
      bullet = trimmed(substr($0, 3))
      next
    }
    /^[[:space:]]/ && trimmed($0) != "" && bullet != "" { bullet = bullet " " trimmed($0) }
    END { flush() }
  '
}

refuse_repeats() {
  local released file
  released="$(git show "$1:CHANGELOG.md" | joined_bullets 0)" || return 1
  while IFS= read -r file; do
    fragment_bullets "$file" | joined_bullets 1 | RELEASED="$released" FILE="$file" awk '
      BEGIN {
        count = split(ENVIRON["RELEASED"], lines, "\n")
        for (i = 1; i <= count; i++) {
          tab = index(lines[i], "\t")
          bullet = substr(lines[i], tab + 1)
          if (!(bullet in version)) version[bullet] = substr(lines[i], 1, tab - 1)
        }
      }
      {
        bullet = substr($0, index($0, "\t") + 1)
        if (bullet in version) {
          print ENVIRON["FILE"] " repeats a bullet of v" version[bullet] ": " bullet
          exit 1
        }
      }
    ' >&2 || return 1
  done <<<"$2"
}

stray_line() {
  git show "HEAD:$1" | awk '
    { sub(/\r$/, "") }
    NR == 1 { next }
    /^- / && $0 !~ /^- [ \t]*$/ { seen = 1; next }
    stray != "" || /^- / { next }
    !seen && !/^[ \t]*$/ { stray = $0 }
    seen && $0 != "" && !/^[ \t]/ { stray = $0 }
    END { if (seen) print stray }
  '
}

fragment_kind() {
  local kind stray
  kind="$(git show "HEAD:$1" | sed -n '1s/^kind: //p' | tr -d '\r')"
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
  stray="$(stray_line "$1")"
  if [ -n "$stray" ]; then
    echo "$1 has a bullet, so every line after its kind must be a \`- \` bullet, an indented continuation or blank: $stray" >&2
    return 1
  fi
  echo "$kind"
}
