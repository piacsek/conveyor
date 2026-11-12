#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/fragments.sh
if [ "$#" -ne 2 ]; then
  echo "usage: scripts/version-check.sh <base> <head>" >&2
  exit 1
fi
if ! head="$(git rev-parse --quiet --verify "$2^{commit}")"; then
  echo "$2 is not a commit here: fetch the pull request first (git fetch origin pull/<number>/head)" >&2
  exit 1
fi
fragments_at="$head"
added=""
refused=0

refuse() {
  echo "$*" >&2
  refused=1
}

under_changes() {
  case "$1" in
    changes/*) return 0 ;;
    *) return 1 ;;
  esac
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

fork="$(git merge-base "$1" "$head")"
listing="$(mktemp)"
trap 'rm -f "$listing"' EXIT
git diff --name-status -z -M "$fork" "$head" >"$listing"
while IFS= read -r -d '' status && IFS= read -r -d '' path; do
  renamed=""
  case "$status" in
    R* | C*) IFS= read -r -d '' renamed ;;
  esac
  if [ "$path" = CHANGELOG.md ] || [ "$renamed" = CHANGELOG.md ]; then
    refuse "CHANGELOG.md is changed: leave it as main has it, the release workflow writes it from the fragments in changes/ (add one there instead)"
  fi
  case "$status" in
    A)
      case "$path" in
        changes/*/*) ;;
        changes/*.md) added="$added$path"$'\n' ;;
      esac
      ;;
    R*)
      if under_changes "$path"; then
        refuse "$path is renamed to $renamed: a fragment on main is never renamed, so restore $path and add new text as a new file changes/<name>.md"
      elif under_changes "$renamed"; then
        refuse "$path is renamed to $renamed: a fragment is only ever added, so restore $path and add $renamed as a new file"
      fi
      ;;
    D)
      if under_changes "$path"; then
        refuse "$path is deleted: a fragment on main is never deleted, so restore it as main has it"
      fi
      ;;
    *)
      if under_changes "$path"; then
        refuse "$path is edited: a fragment on main never changes, so leave it as it is and put the correction in a new file changes/<name>.md"
      fi
      ;;
  esac
done <"$listing"

refuse_strays || refused=1
while IFS= read -r file; do
  [ -n "$file" ] || continue
  fragment_kind "$file" >/dev/null || refused=1
done <<<"$added"
if [ -n "$added" ]; then
  if last="$(last_tag)"; then
    refuse_repeats "$last" "${added%$'\n'}" || refused=1
  else
    refused=1
  fi
fi

[ "$refused" -eq 0 ] || exit 1
echo "the pull request leaves the version to the release workflow"
