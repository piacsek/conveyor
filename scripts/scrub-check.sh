#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
list="${CONVEYOR_DENYLIST:-$HOME/.config/conveyor-dev/denylist}"
[ -f "$list" ] || { echo "scrub-check: no denylist at $list, skipped"; exit 0; }
found=0
for where in --untracked --cached; do
  status=0
  git grep "$where" -niIF -f "$list" || status=$?
  case "$status" in
    0) found=1 ;;
    1) ;;
    *) echo "scrub-check: git grep $where failed" >&2; exit 1 ;;
  esac
done
if [ "$found" = 1 ]; then
  echo "scrub-check: denylisted terms found (see above)" >&2
  exit 1
fi
echo "scrub-check: clean"
