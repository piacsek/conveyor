#!/usr/bin/env bash
# Gates, dev install, commit and push in one step. Any failure aborts before the commit.
# Usage: scripts/ship.sh "type(scope): message"
set -euo pipefail
cd "$(dirname "$0")/.."
message="${1:?commit message}"
branch="$(git rev-parse --abbrev-ref HEAD)"
if [ "$branch" = "main" ]; then
  echo "refusing to ship on main" >&2
  exit 1
fi
scripts/gates.sh
scripts/dev-install.sh
git add -A
git commit -q -m "$message"
git push -q -u origin "$branch"
git log --oneline -1
