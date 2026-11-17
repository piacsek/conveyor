#!/usr/bin/env bash
# Usage: scripts/screenshots.sh [out-dir]   (needs tmux, freeze, Google Chrome, cargo build --release)
set -euo pipefail

out="${1:-docs}"
root="$(cd "$(dirname "$0")/.." && pwd)"
bin="$root/target/release/conveyor"
cargo build --release --manifest-path "$root/Cargo.toml"
mkdir -p "$out"
font=()
for f in "$HOME/Library/Fonts/FiraCodeNerdFont-Regular.ttf" /usr/share/fonts/truetype/firacode/FiraCodeNerdFont-Regular.ttf; do
  [ -f "$f" ] && font=(--font.file "$f") && break
done

sock="conveyor-shots-$$"
home="$(mktemp -d)"
trap 'tmux -L "$sock" kill-server 2>/dev/null || true; rm -rf "$home"' EXIT
t() { tmux -L "$sock" -f /dev/null "$@"; }

shot_window="$(t new-session -d -s main -x 160 -y 14 -c "$home" -P -F '#{window_id}')"
t set -g status off

cp "$(dirname "$0")/truecolor.py" "$home/truecolor.py"

chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
[ -x "$chrome" ] || chrome="$(command -v google-chrome || command -v chromium || true)"

render() {
  local name="$1"
  shift
  local svg="$home/$name.svg"
  [ -s "$home/$name.ansi" ] || { echo "empty capture for $name" >&2; exit 1; }
  if [ -z "$chrome" ]; then
    freeze "$home/$name.ansi" -o "$out/$name.png" --theme catppuccin-mocha \
      --font.size 14 --margin 0 "$@" </dev/null
    return
  fi
  freeze "$home/$name.ansi" -o "$svg" --theme catppuccin-mocha "${font[@]}" \
    --font.family "FiraCode Nerd Font" --font.size 14 --margin 0 "$@" </dev/null
  # FiraCode has no ✗ ⊘ ▸; Chrome's default fallback is wider and shifts the rest of the row, so
  # the borders jog. Menlo has them at (almost) the same advance, and DejaVu is its Linux twin.
  sed -i.bak 's/font-family="FiraCode Nerd Font"/font-family="FiraCode Nerd Font, Menlo, DejaVu Sans Mono, monospace"/' "$svg"
  local w h
  w="$(grep -o 'width="[0-9.]*"' "$svg" | head -1 | grep -o '[0-9]*' | head -1)"
  h="$(grep -o 'height="[0-9.]*"' "$svg" | head -1 | grep -o '[0-9]*' | head -1)"
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=2 \
    --window-size="$w,$h" --screenshot="$out/$name.png" "file://$svg" >/dev/null 2>&1
}

shoot() {
  local name="$1" width="$2" height="$3" keys="$4"
  t resize-window -t "$shot_window" -x "$width" -y "$height"
  t respawn-pane -k -t "$shot_window" "env -u XDG_CONFIG_HOME -u CONVEYOR_CONFIG -u XDG_STATE_HOME -u CONVEYOR_LOG HOME='$home' '$bin' --demo"
  sleep 1.5
  [ -n "$keys" ] && t send-keys -t "$shot_window" "$keys" && sleep 0.5
  t capture-pane -e -p -t "$shot_window" | python3 "$home/truecolor.py" >"$home/$name.ansi"
  render "$name" --padding 20,64,20,20 --window
}

shoot details 160 26 "jd"
echo "wrote $out/details.png"
shoot help 160 26 "?"
echo "wrote $out/help.png"
