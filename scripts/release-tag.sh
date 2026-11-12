#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/fragments.sh
. scripts/releases.sh

if [ "${GITHUB_ACTIONS:-}" != true ]; then
  echo "scripts/release-tag.sh runs in the release workflow only; merge to main and watch the release run" >&2
  exit 1
fi

resume() {
  echo "version=${last#v}"
  echo "tag=$last"
  exit 0
}

last="$(last_tag)"
state="$(release_state "$last")"
[ "$state" = complete ] || resume
next="$(scripts/next-version.sh)"
[ -n "$next" ] || exit 0
scripts/release-commit.sh "$next"
git add Cargo.toml Cargo.lock CHANGELOG.md
git -c user.name="github-actions[bot]" -c user.email="github-actions[bot]@users.noreply.github.com" commit -qm "Release v$next"
git tag "v$next"
git push -q origin "refs/tags/v$next"
echo "version=$next"
echo "tag=v$next"
