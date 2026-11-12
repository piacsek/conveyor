#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$#" -ne 2 ]; then
  echo "usage: scripts/version-check.sh <base> <head>" >&2
  exit 1
fi
if ! head="$(git rev-parse --quiet --verify "$2^{commit}")"; then
  echo "$2 is not a commit here: fetch the pull request first (git fetch origin pull/<number>/head)" >&2
  exit 1
fi
refused=0

refuse() {
  echo "$*" >&2
  refused=1
}

package="$(git show "$head:Cargo.toml" 2>/dev/null | awk '
  /^[ \t]*\[/ { package = ($0 == "[package]") }
  package && /^version = / { sub(/^version = "/, ""); sub(/".*$/, ""); print; exit }
')" || true
[ "$package" = 0.0.0 ] || refuse "Cargo.toml sets the package version to $package: put back \`version = \"0.0.0\"\`, only the release workflow picks a version (from the kinds in changes/)"

lock="$(git show "$head:Cargo.lock" 2>/dev/null | awk '
  entry && /^version = / { sub(/^version = "/, ""); sub(/".*$/, ""); print; exit }
  { entry = ($0 == "name = \"conveyor\"") }
')" || true
[ "$lock" = 0.0.0 ] || refuse "Cargo.lock sets the conveyor entry to $lock: put back \`version = \"0.0.0\"\` under \`name = \"conveyor\"\`, only the release workflow picks a version (from the kinds in changes/)"

[ "$refused" -eq 0 ] || exit 1
echo "the pull request leaves the version to the release workflow"
