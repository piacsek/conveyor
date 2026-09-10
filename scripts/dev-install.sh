#!/usr/bin/env bash
# Build the release binary and expose it as `conveyor-dev`, separate from the Homebrew `conveyor`.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --locked
mkdir -p ~/.local/bin
ln -sfn "$PWD/target/release/conveyor" ~/.local/bin/conveyor-dev
echo "conveyor-dev -> $PWD/target/release/conveyor"
