#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/fragments.sh
. scripts/release-rewrite.sh
say() {
  printf 'version check: %s\n' "$*" >&2
}
if [ "$#" -ne 2 ]; then
  say "usage: scripts/version-check.sh <base> <head>"
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  say "cargo is not on PATH: the check asks cargo for the version in Cargo.toml, so install Rust and run it again"
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
scratch="$(mktemp -d "${TMPDIR:-/tmp}/version-check.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
scratch="$(cd "$scratch" && pwd -P)"
listing="$scratch/listing"
errors="$scratch/errors"
tree="$scratch/head"
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

refuse_pending_repeats() {
  local pending file known=""
  pending="$(git -c core.quotePath=false diff-tree -r --name-only --no-renames --diff-filter=A "$2" "$1" -- changes/)" || return 1
  while IFS= read -r file; do
    [ -n "$file" ] || continue
    known="$known$(fragments_at="$1" fragment_bullets "$file" | joined_bullets 1 | FILE="$file" awk '{ print ENVIRON["FILE"] "\t" substr($0, index($0, "\t") + 1) }')"$'\n'
  done <<<"$pending"
  while IFS= read -r file; do
    fragment_bullets "$file" | joined_bullets 1 | KNOWN="$known" FILE="$file" awk '
      BEGIN {
        count = split(ENVIRON["KNOWN"], lines, "\n")
        for (i = 1; i <= count; i++) {
          tab = index(lines[i], "\t")
          bullet = substr(lines[i], tab + 1)
          if (tab && !(bullet in owner)) owner[bullet] = substr(lines[i], 1, tab - 1)
        }
      }
      {
        bullet = substr($0, index($0, "\t") + 1)
        if (bullet in owner) {
          print ENVIRON["FILE"] " repeats a bullet of " owner[bullet] ", which main holds and no release has shipped yet: " bullet
          exit 1
        }
      }
    ' >&2 || return 1
  done <<<"$3"
}

materialise() {
  local entry meta path made=""
  while IFS= read -r -d '' entry; do
    meta="${entry%%$'\t'*}"
    path="${entry#*$'\t'}"
    case "$meta" in
      "100644 blob "* | "100755 blob "*) ;;
      *) continue ;;
    esac
    case "/$path/" in
      */../* | */./* | *//* | */[.][cC][aA][rR][gG][oO]/* | */[rR][uU][sS][tT]-[tT][oO][oO][lL][cC][hH][aA][iI][nN]/* | */[rR][uU][sS][tT]-[tT][oO][oO][lL][cC][hH][aA][iI][nN].[tT][oO][mM][lL]/*) continue ;;
    esac
    case "$path" in
      */*)
        if [ "${path%/*}" != "$made" ]; then
          mkdir -p "$tree/${path%/*}" 2>/dev/null || continue
          made="${path%/*}"
        fi
        ;;
    esac
    case "/$path" in
      */Cargo.toml) git cat-file blob "${meta##* }" >"$tree/$path" 2>/dev/null || continue ;;
      *) [ -e "$tree/$path" ] || : >"$tree/$path" 2>/dev/null || continue ;;
    esac
  done < <(git ls-tree -r -z "$head")
}

metadata() {
  (
    cd "$scratch/cwd"
    env -i PATH="$PATH" HOME="$scratch/home" CARGO_HOME="$scratch/home" RUSTUP_HOME="${RUSTUP_HOME:-${HOME:-}/.rustup}" RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}" \
      cargo metadata --no-deps --offline --format-version 1 --manifest-path "$tree/Cargo.toml" >"$scratch/metadata" 2>"$errors"
  )
}

lock_version() {
  awk '
    entry && /^version = / { sub(/^version = "/, ""); sub(/".*$/, ""); print; exit }
    { entry = ($0 == "name = \"conveyor\"") }
  '
}

manifest_rehearsed() {
  local rewritten
  rewritten="$(manifest_with_version "$probe" "$tree/Cargo.toml")" || return 1
  manifest_reads "$probe" "$rewritten" || return 1
  printf '%s\n' "$rewritten" >"$tree/Cargo.toml"
  [ "$(cd "$tree" && grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)" = "$probe" ] || return 1
  metadata || return 1
  [ "$(root_package "$tree" <"$scratch/metadata")" = "conveyor $probe" ]
}

lock_rehearsed() {
  local rewritten
  rewritten="$(printf '%s\n' "$lockfile" | lock_with_version "$probe" -)" || return 1
  lock_reads "$probe" "$rewritten" || return 1
  [ "$(printf '%s\n' "$rewritten" | lock_version)" = "$probe" ]
}

read -r high minor patch < <(od -An -N6 -tu2 /dev/urandom)
probe="$((high + 100000)).$minor.$patch"
mkdir "$tree" "$scratch/home" "$scratch/cwd"
materialise
if metadata; then
  package="$(root_package "$tree" <"$scratch/metadata")"
  case "$package" in
    unreadable)
      refuse "cargo metadata answered in a shape the check does not know: the check reads its output only when it starts the way cargo writes it, so run it again with the cargo this check was written for"
      ;;
    "")
      refuse "Cargo.toml has no package of its own (cargo reads a virtual manifest): the package conveyor belongs at the root, where the release workflow writes its version"
      ;;
    "conveyor 0.0.0")
      if ! manifest_rehearsed; then
        refuse "the release workflow could not set the version in Cargo.toml: rehearsed with a random probe version, cargo, the release build and the release script did not all read it back, so after the merge the release run would push its tag and then fail to build it; keep the plain form: \`[package]\` alone on its line, \`version = \"0.0.0\"\` as its own line under it, and no other \`version = \` line above it"
      fi
      ;;
    "conveyor "*)
      refuse "Cargo.toml sets the package version to ${package#conveyor }: put back \`version = \"0.0.0\"\`, only the release workflow picks a version (from the kinds in changes/)"
      ;;
    *)
      refuse "Cargo.toml names its package ${package%% *}: the package at the root must be conveyor, the one the release workflow versions"
      ;;
  esac
else
  answer="$(awk -v tree="$tree/" '
    /^error/ {
      while ((at = index($0, tree)) > 0) $0 = substr($0, 1, at - 1) substr($0, at + length(tree))
      print
      found = 1
      exit
    }
    END { if (!found) print "nothing" }
  ' "$errors" | tr '[:cntrl:]' ' ')"
  answer="${answer% }"
  refuse "Cargo.toml does not load with cargo, which answers \`$answer\`: the manifest must load with cargo (cargo metadata), fix it there"
fi

lockfile="$(git show "$head:Cargo.lock" 2>/dev/null)" || true
entries="$(printf '%s\n' "$lockfile" | awk '/^[ \t]*name[ \t]*=[ \t]*"conveyor"[ \t]*$/ { count++ } END { print count + 0 }')"
lock="$(printf '%s\n' "$lockfile" | lock_version)"
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
elif ! lock_rehearsed; then
  refuse "the release workflow could not set the version in Cargo.lock: rehearsed with a random probe version, the conveyor entry did not read it back, so after the merge the release run would push its tag and then fail to build it; regenerate it with cargo (\`cargo update --workspace\`) instead of editing it by hand"
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
    said refuse_pending_repeats "$1" "$last" "${added%$'\n'}" || refused=1
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
