#!/usr/bin/env bash
# Quality gates. Every step must pass; the script exits on the first failure and never
# swallows output, so do not pipe it through tail.
set -euo pipefail
cd "$(dirname "$0")/.."

step() {
  echo "==> $*"
  "$@"
}

step scripts/scrub-check.sh
step cargo fmt --check
step cargo clippy --all-targets -- -D warnings
step cargo test
if command -v tmux >/dev/null; then
  step cargo test -- --ignored
else
  echo "==> skipping the ignored e2e tests: tmux is not installed (brew install tmux); CI runs them" >&2
fi
echo "gates passed"
