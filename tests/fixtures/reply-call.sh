#!/bin/sh
echo "${0##*/} $*" >> "$CALLS_LOG"
case "$*" in
  "${FAILING_CALL:-no call fails}"*)
    [ -z "${FAILING_STDERR:-}" ] || echo "$FAILING_STDERR" >&2
    exit 1
    ;;
esac
for reply in "$REPLIES"/*; do
  [ -f "$reply" ] || continue
  case "$*" in
    *"${reply##*/}"*) cat "$reply" ;;
  esac
done
