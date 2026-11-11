#!/usr/bin/env bash
# Usage: scripts/release.sh 0.24.0
set -euo pipefail
cd "$(dirname "$0")/.."
version="${1:?version, with or without the leading v}"
version="${version#v}"
tag="v$version"

refuse() {
  echo "$*" >&2
  exit 1
}

[ -z "$(git status --porcelain)" ] || refuse "the working tree is dirty; commit or stash everything before releasing"
branch="$(git rev-parse --abbrev-ref HEAD)"
[ "$branch" = "main" ] || refuse "on $branch, not main; switch to main, the release is tagged from what was merged"
git fetch -q origin
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || refuse "local main is not origin/main; run git pull --ff-only, and land local commits through a pull request"
manifest="$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)"
[ "$manifest" = "$version" ] || refuse "Cargo.toml is at $manifest, not $version; bump it, add the CHANGELOG.md entry and merge that to main first"
on_origin="$(git ls-remote --tags origin "refs/tags/$tag")"
[ -z "$on_origin" ] || refuse "$tag already exists on origin; a released tag never moves, bump the version instead"
local_tag="$(git tag --list "$tag")"
[ -z "$local_tag" ] || refuse "$tag already exists locally; if it was never pushed, delete it with git tag -d $tag"
in_flight="$(gh run list --workflow release.yml --json status,databaseId --jq '.[] | select(.status != "completed") | .databaseId')"
[ -z "$in_flight" ] || refuse "a release run is queued or in progress; wait for it (gh run list --workflow release.yml), then run this again"

git tag "$tag"
git push -q origin "refs/tags/$tag"

run=""
for _ in $(seq "${RELEASE_POLL_ATTEMPTS:-30}"); do
  run="$(gh run list --workflow release.yml --branch "$tag" --limit 1 --json databaseId --jq '.[].databaseId')"
  [ -z "$run" ] || break
  sleep "${RELEASE_POLL_INTERVAL:-2}"
done
[ -n "$run" ] || refuse "no release run appeared for $tag; the tag is pushed, so do not run this again: look at gh run list --workflow release.yml"
gh run watch "$run" --exit-status

assets="$(gh release view "$tag" --json assets --jq '.assets[].name')"
for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu; do
  for asset in "conveyor-$target.tar.gz" "conveyor-$target.tar.gz.sha256"; do
    grep -qxF "$asset" <<<"$assets" || refuse "release $tag lacks $asset; rerun what failed with gh run rerun $run --failed"
  done
done
[ "$(grep -c . <<<"$assets")" -eq 6 ] || refuse "release $tag holds more than the three tarballs and their .sha256; look at gh release view $tag"
formula="$(gh api repos/piacsek/homebrew-tap/contents/Formula/conveyor.rb -H 'Accept: application/vnd.github.raw+json')"
grep -qF "/releases/download/$tag/" <<<"$formula" || refuse "the tap formula does not name $tag; run scripts/homebrew-formula.sh $version > ~/projects/homebrew-tap/Formula/conveyor.rb and push the tap"
gh release view "$tag" --json url --jq .url
