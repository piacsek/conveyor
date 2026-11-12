#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/releases.sh

tag="${1:?usage: scripts/release-unfinished.sh vX.Y.Z}"
state="$(release_state "$tag")"
if [ "$state" = complete ]; then
  echo "unfinished=false"
else
  echo "unfinished=true"
fi
