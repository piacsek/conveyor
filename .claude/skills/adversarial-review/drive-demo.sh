#!/usr/bin/env bash
# Drive `conveyor --demo` in a detached tmux pane of a given size, send it keys, capture the screen.
# Usage: drive-demo.sh <binary> <width> <height> <config.toml> [tmux key ...]
# Prints `alive`, or `died: exit <code>` and the first lines of stderr. The last screen is in
# $OUT/screen.txt (OUT defaults to a fresh temporary directory, printed on the last line).
set -euo pipefail
binary="${1:?binary}"
width="${2:?width}"
height="${3:?height}"
config="${4:?config file (an empty file is the default config)}"
shift 4
command -v tmux >/dev/null || { echo "drive-demo.sh needs tmux" >&2; exit 1; }
out="${OUT:-$(mktemp -d)}"
mkdir -p "$out/home"
rm -f "$out/exit" "$out/stderr" "$out/screen.txt"
session="drive-demo-$$"
run="$(printf '%q ' env -u XDG_CONFIG_HOME -u XDG_STATE_HOME -u CONVEYOR_LOG \
  "CONVEYOR_CONFIG=$config" "HOME=$out/home" "$binary" --demo)"
tmux new-session -d -s "$session" -x "$width" -y "$height" \
  "$run 2>$(printf '%q' "$out/stderr"); echo \$? >$(printf '%q' "$out/exit"); sleep 30"
trap 'tmux kill-session -t "$session" 2>/dev/null || true' EXIT
sleep 1.2
for key in "$@"; do
  tmux send-keys -t "$session" "$key"
  sleep 0.25
done
sleep 0.5
tmux capture-pane -t "$session" -p >"$out/screen.txt"
if [ -f "$out/exit" ]; then
  echo "died: exit $(cat "$out/exit")"
  head -n 8 "$out/stderr"
else
  echo alive
fi
echo "$out"
