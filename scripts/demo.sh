#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cargo build --release --manifest-path "$root/Cargo.toml"
bin="$root/target/release/conveyor"
[ -x "$bin" ] || { echo "no binary at $bin" >&2; exit 1; }
home="$(mktemp -d)"
trap 'rm -rf "$home"' EXIT
mkdir -p "$home/.config/conveyor"
printf '[ui]\nicons = "nerd"\n' >"$home/.config/conveyor/config.toml"

cd "$root"
CONVEYOR_DEMO_HOME="$home" CONVEYOR_DEMO_BIN="$bin" vhs scripts/demo.tape
echo "wrote docs/demo.gif"
