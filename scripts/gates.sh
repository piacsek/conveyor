#!/usr/bin/env bash
# Quality gates. Every step must pass; the script exits on the first failure and never
# swallows output, so do not pipe it through tail.
set -euo pipefail
cd "$(dirname "$0")/.."

step() {
  echo "==> $*"
  "$@"
}

step cargo fmt --check
step cargo clippy --all-targets -- -D warnings
step cargo test
step cargo test -- --ignored
echo "gates passed"
