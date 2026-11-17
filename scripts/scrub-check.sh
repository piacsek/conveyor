#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
list="${CONVEYOR_DENYLIST:-$HOME/.config/conveyor-dev/denylist}"
if [ -z "${CONVEYOR_DENYLIST:-}" ] && [ ! -e "$list" ] && [ ! -L "$list" ]; then
  echo "scrub-check: no denylist at $list, skipped"
  exit 0
fi
if [ ! -f "$list" ] || [ ! -r "$list" ]; then
  echo "scrub-check: $list is not a readable file" >&2
  exit 1
fi
found=0
search() {
  local what="$1" status=0
  shift
  "$@" || status=$?
  case "$status" in
    0) found=1 ;;
    1) ;;
    *) echo "scrub-check: $what failed" >&2; exit 1 ;;
  esac
}
listed_paths() {
  git ls-files -z --cached --others --exclude-standard | tr '\0' '\n' | grep -iF -f "$list"
}
search "git grep --untracked" git grep --untracked -niIF -f "$list"
search "git grep --cached" git grep --cached -niIF -f "$list"
search "the path check" listed_paths
if [ "$found" = 1 ]; then
  echo "scrub-check: denylisted terms found (see above)" >&2
  exit 1
fi
echo "scrub-check: clean"
