#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/fragments.sh
say() {
  printf 'version check: %s\n' "$*" >&2
}
if [ "$#" -ne 2 ]; then
  say "usage: scripts/version-check.sh <base> <head>"
  exit 1
fi
if ! head="$(git rev-parse --quiet --verify "$2^{commit}")"; then
  say "$2 is not a commit here: fetch the pull request first (git fetch origin pull/<number>/head)"
  exit 1
fi
if ! fork="$(git merge-base "$1" "$head")"; then
  say "$2 shares no history with $1: cut the branch from $1 and open the pull request again"
  exit 1
fi
export LC_ALL=C
listing="$(mktemp)"
errors="$(mktemp)"
trap 'rm -f "$listing" "$errors"' EXIT
git diff --name-status -z -M "$fork" "$head" >"$listing"
hostile=0
while IFS= read -r -d '' field; do
  case "$field" in
    *[[:cntrl:]]*) hostile=$((hostile + 1)) ;;
  esac
done <"$listing"
if [ "$hostile" -gt 0 ]; then
  say "changed paths whose name holds a control character (a newline, a tab, another byte below 0x20 or 0x7f): $hostile; rename them, the check never prints such a name"
  exit 1
fi
fragments_at="$head"
added=""
touched=""
refused=0

refuse() {
  say "$*"
  refused=1
}

said() {
  local status=0
  "$@" 2>"$errors" || status=$?
  tr '\r' ' ' <"$errors" | sed 's/^/version check: /' >&2
  return "$status"
}

under_changes() {
  case "$1" in
    changes/*) return 0 ;;
    *) return 1 ;;
  esac
}

manifest="$(git show "$head:Cargo.toml" 2>/dev/null)" || true
versions="$(printf '%s\n' "$manifest" | awk '
  /^[ \t]*\[/ { package = ($0 == "[package]") }
  package && /^[ \t]*["'\'']?version["'\'']?[ \t]*[=.]/ { count++ }
  END { print count + 0 }
')"
package="$(printf '%s\n' "$manifest" | awk '
  /^[ \t]*\[/ { package = ($0 == "[package]") }
  package && /^version = / { sub(/^version = "/, ""); sub(/".*$/, ""); print; exit }
')"
case "$manifest" in
  *'"""'* | *"'''"*)
    refuse "Cargo.toml holds a multi-line string (\"\"\" or '''): the check reads the version line by line, so write every string in Cargo.toml on one line"
    ;;
  *)
    if [ "$versions" -gt 1 ]; then
      refuse "Cargo.toml holds more than one \`version\` line in [package]: the check reads the version line by line, so keep the one \`version = \"0.0.0\"\`"
    elif [ -z "$package" ]; then
      refuse "Cargo.toml has no \`version = \"...\"\` line under a plain \`[package]\` header: the check reads the version line by line, so write the header as \`[package]\` alone on its line and the version as \`version = \"0.0.0\"\`"
    elif [ "$package" != 0.0.0 ]; then
      refuse "Cargo.toml sets the package version to $package: put back \`version = \"0.0.0\"\`, only the release workflow picks a version (from the kinds in changes/)"
    fi
    ;;
esac

lockfile="$(git show "$head:Cargo.lock" 2>/dev/null)" || true
entries="$(printf '%s\n' "$lockfile" | awk '/^[ \t]*name[ \t]*=[ \t]*"conveyor"[ \t]*$/ { count++ } END { print count + 0 }')"
lock="$(printf '%s\n' "$lockfile" | awk '
  entry && /^version = / { sub(/^version = "/, ""); sub(/".*$/, ""); print; exit }
  { entry = ($0 == "name = \"conveyor\"") }
')"
case "$lockfile" in
  *'"""'* | *"'''"*)
    shape="holds a multi-line string (\"\"\" or ''')"
    ;;
  *)
    shape="$(printf '%s\n' "$lockfile" | awk -v q="'" '
      function outside(line) { return "line " line " is a `name = \"conveyor\"` that is not right after a `[[package]]` line and right before its `version = \"...\"` line" }
      entry && !/^version = "[^"]*"$/ { print outside(NR - 1); done = 1; exit }
      $0 ~ "^[ \t]*(name|\"[^=]*\"|" q "[^=]*" q ")[ \t]*[=.]" && !/^name = "[A-Za-z0-9_-]+"$/ { print "line " NR " is a `name` or quoted key other than `name = \"<crate>\"` (letters, digits, `_` and `-`)"; done = 1; exit }
      $0 == "name = \"conveyor\"" && previous != "[[package]]" { print outside(NR); done = 1; exit }
      { entry = ($0 == "name = \"conveyor\""); previous = $0 }
      END { if (entry && !done) print outside(NR) }
    ')"
    ;;
esac
if [ -n "$shape" ]; then
  refuse "Cargo.lock $shape: cargo writes Cargo.lock in one shape, so regenerate it with cargo (\`cargo update --workspace\`) instead of editing it by hand"
elif [ "$entries" -gt 1 ]; then
  refuse "Cargo.lock holds more than one package named \`conveyor\`: the check reads the version line by line, so keep the one entry with \`version = \"0.0.0\"\`"
elif [ -z "$lock" ]; then
  refuse "Cargo.lock has no \`name = \"conveyor\"\` line with a \`version = \"...\"\` line right after it: cargo writes Cargo.lock in one shape, so regenerate it with cargo (\`cargo update --workspace\`) instead of editing it by hand"
elif [ "$lock" != 0.0.0 ]; then
  refuse "Cargo.lock sets the conveyor entry to $lock: put back \`version = \"0.0.0\"\` under \`name = \"conveyor\"\`, only the release workflow picks a version (from the kinds in changes/)"
fi

while IFS= read -r -d '' status && IFS= read -r -d '' path; do
  renamed=""
  case "$status" in
    R* | C*) IFS= read -r -d '' renamed ;;
  esac
  if [ "$path" = CHANGELOG.md ] || [ "$renamed" = CHANGELOG.md ]; then
    refuse "CHANGELOG.md is changed: leave it as main has it, the release workflow writes it from the fragments in changes/ (add one there instead)"
  else
    touched="$touched$path"$'\n'"$renamed"$'\n'
  fi
  case "$status" in
    A)
      case "$path" in
        changes/*/*) ;;
        changes/*.md)
          if git cat-file -e "$1:$path" 2>/dev/null; then
            refuse "$path is taken on main: the pull request needs its own file changes/<name>.md"
          else
            added="$added$path"$'\n'
          fi
          ;;
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

said refuse_strays || refused=1
while IFS= read -r file; do
  [ -n "$file" ] || continue
  said fragment_kind "$file" >/dev/null || refused=1
done <<<"$added"
if [ -n "$added" ]; then
  if last="$(said last_tag)"; then
    said refuse_repeats "$last" "${added%$'\n'}" || refused=1
  else
    refused=1
  fi
fi

binary="$(printf '%s' "$touched" | scripts/feeds-binary.sh | sed -n 1p)"
if [ -n "$binary" ] && [ -z "$added" ]; then
  refuse "$binary feeds the binary, so the pull request needs a fragment: add changes/<branch>.md starting with \`kind: fix\`, \`kind: feat\` or \`kind: internal\` (see Releasing in AGENTS.md)"
fi

[ "$refused" -eq 0 ] || exit 1
echo "the pull request leaves the version to the release workflow"
