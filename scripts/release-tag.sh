#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/fragments.sh

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
errors="$(mktemp)"
trap 'rm -f "$errors"' EXIT
if ! published="$(gh release view "$last" --json assets --jq '.assets[].name' 2>"$errors")"; then
  if ! grep -q 'release not found' "$errors"; then
    echo "the release of $last could not be read: $(cat "$errors")" >&2
    exit 1
  fi
  resume
fi
expected=""
for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu; do
  expected+="conveyor-$target.tar.gz"$'\n'"conveyor-$target.tar.gz.sha256"$'\n'
done
unexpected="$(grep -vxF -e '' -f <(printf '%s' "$expected") <<<"$published" | head -n 1 || true)"
if [ -n "$unexpected" ]; then
  echo "the release of $last holds $unexpected besides the three tarballs and their .sha256; remove it by hand" >&2
  exit 1
fi
while IFS= read -r asset; do
  grep -qxF "$asset" <<<"$published" || resume
done <<<"${expected%$'\n'}"
next="$(scripts/next-version.sh)"
[ -n "$next" ] || exit 0
scripts/release-commit.sh "$next"
git add Cargo.toml Cargo.lock CHANGELOG.md
git -c user.name="github-actions[bot]" -c user.email="github-actions[bot]@users.noreply.github.com" commit -qm "Release v$next"
git tag "v$next"
git push -q origin "refs/tags/v$next"
echo "version=$next"
echo "tag=v$next"
