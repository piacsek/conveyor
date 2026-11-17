#!/usr/bin/env bash
set -euo pipefail

every=5
last="$(
  for file in plans/phase-*.md; do
    name="${file##*/phase-}"
    number="${name%%-*}"
    case "$number" in
      '' | *[!0-9]*) continue ;;
    esac
    echo "$number"
  done | sort -n | tail -n1
)"
next=$((last + 1))
if [ $((next % every)) -ne 0 ]; then
  echo "no sweep is due: the next phase is $next"
  exit 0
fi

title="Sweep due before phase $next"
errors="$(mktemp)"
if ! titles="$(gh issue list --state all --search "\"$title\" in:title" --json title --jq '.[].title' 2>"$errors")"; then
  echo "the sweep issues could not be read: $(cat "$errors")" >&2
  rm -f "$errors"
  exit 1
fi
rm -f "$errors"
if grep -qxF "$title" <<<"$titles"; then
  echo "the issue already exists: $title"
  exit 0
fi

body="$(mktemp)"
{
  echo "Phase $next opens with a sweep (see \"The sweep\" in \`AGENTS.md\`). Run the \`sweep\` skill, or read the report below and write the findings into the \`### Sweep\` section of phase $next's \`## Outcome\`."
  echo
  scripts/sweep-report.sh | sed '1s/^# /## /; 2,$s/^\(#\{1,5\}\) /#\1 /'
} >"$body"
gh issue create --title "$title" --body-file "$body"
rm -f "$body"
echo "opened the issue: $title"
