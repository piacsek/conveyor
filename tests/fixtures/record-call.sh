#!/bin/sh
echo "${0##*/} $*" >> "$CALLS_LOG"
case "$*" in
  "${FAILING_CALL:-no call fails}"*) exit 1 ;;
esac
