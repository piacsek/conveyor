#!/bin/sh
echo "${0##*/} $*" >> "$CALLS_LOG"
for reply in "$REPLIES"/*; do
  [ -f "$reply" ] || continue
  case "$*" in
    *"${reply##*/}"*) cat "$reply" ;;
  esac
done
