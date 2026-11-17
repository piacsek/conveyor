#!/usr/bin/env bash
set -euo pipefail

plans=plans
first_with_hurdles=40

numbered() {
  for file in "$plans"/phase-*.md; do
    [ -f "$file" ] || continue
    name="${file##*/phase-}"
    number="${name%%-*}"
    case "$number" in
      '' | *[!0-9]*) continue ;;
    esac
    printf '%s\t%s\n' "$number" "$file"
  done | sort -n -k1,1
}

phases="$(numbered)"
if [ -z "$phases" ]; then
  echo "no phase plans under $plans/" >&2
  exit 1
fi
last="$(tail -n1 <<<"$phases" | cut -f1)"

if [ $# -gt 0 ]; then
  first="$1"
else
  first="$first_with_hurdles"
  while IFS=$'\t' read -r number file; do
    if grep -qx '### Sweep' "$file"; then
      first=$((number + 1))
    fi
  done <<<"$phases"
fi

in_range="$(awk -F'\t' -v first="$first" '$1 + 0 >= first + 0' <<<"$phases")"

records="$(
  while IFS=$'\t' read -r number file; do
    [ -n "$number" ] || continue
    awk -v phase="$number" '
      function flush() {
        if (item == "") return
        if (item != "none recorded") {
          rest = substr(item, 2)
          tick = index(rest, "`")
          tag = substr(rest, 1, tick - 1)
          body = substr(rest, tick + 1)
          sub(/^ +/, "", body)
          what = body; where = ""
          for (at = length(body); at > 0; at--) {
            if (substr(body, at, 5) == " \342\206\222 ") {
              what = substr(body, 1, at - 1)
              where = substr(body, at + 5)
              break
            }
          }
          printf "%s\t%s\t%s\t%s\n", phase, tag, what, where
        }
        item = ""
      }
      /^## / { flush(); inside = ($0 == "## Hurdles"); if (inside) seen = 1; next }
      !inside { next }
      /^- / { flush(); item = substr($0, 3); next }
      /^  / && item != "" { line = $0; sub(/^ +/, "", line); item = item " " line; next }
      END { flush(); if (!seen) printf "missing\t%s\n", phase }
    ' "$file"
  done <<<"$in_range"
)"

hurdles="$(grep -v '^missing' <<<"$records" || true)"
missing="$(grep '^missing' <<<"$records" | cut -f2 || true)"

list() {
  if [ -n "$1" ]; then printf '%s\n' "$1"; else echo "- none"; fi
}

echo "# Sweep report: phases $first to $last"
echo
echo "## Hurdles by tag"
echo
echo "| Tag | Hurdles |"
echo "| --- | --- |"
if [ -n "$hurdles" ]; then
  cut -f2 <<<"$hurdles" | sort | uniq -c | sort -k1,1nr -k2,2 \
    | awk '{ printf "| `%s` | %s |\n", $2, $1 }'
fi
echo
echo "## Went nowhere (→ none)"
echo
list "$(awk -F'\t' '$4 == "none" { printf "- %s `%s` %s\n", $1, $2, $3 }' <<<"$hurdles")"
echo
echo "## By tag"
if [ -n "$hurdles" ]; then
  for tag in $(cut -f2 <<<"$hurdles" | sort -u); do
    echo
    echo "### \`$tag\`"
    echo
    awk -F'\t' -v tag="$tag" '$2 == tag { printf "- %s %s \342\206\222 %s\n", $1, $3, $4 }' <<<"$hurdles"
  done
else
  echo
  echo "- none"
fi
echo
echo "## No Hurdles section"
echo
list "$(sed 's/^/- /' <<<"$missing" | grep -v '^- $' || true)"

metrics="$plans/metrics.md"
echo
echo "## Metrics"
echo
if [ -f "$metrics" ]; then
  awk -v first="$first" '
    /^\| Phase \|/ { print; header = $0; getline; print; next }
    header != "" && /^\| [0-9]+ \|/ { split($0, cell, "|"); if (cell[2] + 0 >= first + 0) print }
  ' "$metrics"
  echo
  echo "## Empty cells"
  echo
  list "$(awk -v first="$first" '
    function trim(s) { gsub(/^ +| +$/, "", s); return s }
    /^\| Phase \|/ { columns = split($0, name, "|"); next }
    columns && /^\| [0-9]+ \|/ {
      split($0, cell, "|")
      phase = trim(cell[2])
      if (phase + 0 < first + 0) next
      empty = ""
      for (i = 3; i < columns; i++) if (trim(cell[i]) == "") empty = empty (empty == "" ? "" : ", ") trim(name[i])
      if (empty != "") printf "- %s: %s\n", phase, empty
    }
  ' "$metrics")"
else
  echo "- no $metrics"
fi
