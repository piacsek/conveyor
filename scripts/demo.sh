#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cargo build --release --manifest-path "$root/Cargo.toml"
home="$(mktemp -d)"
trap 'rm -rf "$home"' EXIT
mkdir -p "$home/.config/conveyor"
printf '[ui]\nicons = "nerd"\n' >"$home/.config/conveyor/config.toml"

cd "$root"
CONVEYOR_DEMO_HOME="$home" CONVEYOR_DEMO_BIN="$root/target/release/conveyor" vhs scripts/demo.tape
echo "wrote docs/demo.gif"
