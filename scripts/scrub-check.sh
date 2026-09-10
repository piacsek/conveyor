#!/usr/bin/env bash
# Fail if any tracked file mentions a term from the local denylist. The list lives outside the
# repository on purpose (this repository is public); the check is skipped when it is absent.
set -euo pipefail
cd "$(dirname "$0")/.."
list="${CONVEYOR_DENYLIST:-$HOME/.config/conveyor-dev/denylist}"
[ -f "$list" ] || { echo "scrub-check: no denylist at $list, skipped"; exit 0; }
if git ls-files -z | xargs -0 grep -niIF -f "$list" -- 2>/dev/null; then
  echo "scrub-check: denylisted terms found (see above)" >&2
  exit 1
fi
echo "scrub-check: clean"
