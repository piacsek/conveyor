#!/usr/bin/env bash
set -euo pipefail
paths="${1:-}"
build="${2:-}"
flake="${3:-}"

case "$flake" in
  success)
    echo "the flake builds"
    ;;
  skipped)
    if [ "$paths" = success ] && [ "$build" = false ]; then
      echo "no input of the flake changed"
    else
      echo "the flake build was skipped, but the paths job ended in $paths with build=$build" >&2
      exit 1
    fi
    ;;
  *)
    echo "the flake build ended in ${flake:-no result}" >&2
    exit 1
    ;;
esac
