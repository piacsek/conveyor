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
read_log() {
  "$@" || { echo "scrub-check: git log failed" >&2; exit 1; }
}
added_lines() {
  awk '
    /^diff / { hunk = 0; next }
    /^@@/ { match($0, /^@+/); width = RLENGTH - 1; hunk = 1; next }
    hunk && substr($0, 1, width) == substr("++++++++++++++++", 1, width) { print substr($0, width + 1) }
  '
}
matches() {
  local label="$1" text="$2"
  printf '%s\n' "$text" | grep -iF -f "$list" | sed "s|^|scrub-check: $label: |"
}
if [ "${1:-}" = --outgoing ]; then
  commits="$(read_log git rev-list HEAD --not --remotes)"
  head="$(git rev-parse HEAD)"
  for commit in $commits; do
    label="commit $(git rev-parse --short "$commit")"
    [ "$commit" = "$head" ] && label="$label (HEAD)"
    message="$(read_log git log -1 --format=%B "$commit")"
    paths="$(read_log git diff-tree -r --root --cc --no-commit-id --name-only --no-renames --diff-filter=d "$commit")"
    diff="$(read_log git diff-tree -r --root --cc -p -U0 --no-color --no-ext-diff --no-commit-id "$commit")"
    search "the outgoing check" matches "$label message" "$message"
    search "the outgoing check" matches "$label path" "$paths"
    search "the outgoing check" matches "$label added line" "$(printf '%s\n' "$diff" | added_lines)"
  done
  search "the outgoing check" matches "branch name" "$(git rev-parse --abbrev-ref HEAD)"
else
  search "git grep --untracked" git grep --untracked -niIF -f "$list"
  search "git grep --cached" git grep --cached -niIF -f "$list"
  search "the path check" listed_paths
fi
if [ "$found" = 1 ]; then
  echo "scrub-check: denylisted terms found (see above)" >&2
  exit 1
fi
echo "scrub-check: clean"
