#!/usr/bin/env bash
# Render README screenshots from a throwaway tmux server and a `gh` shim that prints a synthetic
# fixture. Never touches the real gh account: HOME is a tempdir and the shim is first on PATH.
# Usage: scripts/screenshots.sh [out-dir]   (needs tmux, freeze, Google Chrome, cargo build --release)
set -euo pipefail

out="${1:-docs}"
root="$(cd "$(dirname "$0")/.." && pwd)"
bin="$root/target/release/conveyor"
[ -x "$bin" ] || cargo build --release --manifest-path "$root/Cargo.toml"
mkdir -p "$out"
font=()
for f in "$HOME/Library/Fonts/FiraCodeNerdFont-Regular.ttf" /usr/share/fonts/truetype/firacode/FiraCodeNerdFont-Regular.ttf; do
  [ -f "$f" ] && font=(--font.file "$f") && break
done

sock="conveyor-shots-$$"
home="$(mktemp -d)"
trap 'tmux -L "$sock" kill-server 2>/dev/null || true; rm -rf "$home"' EXIT
t() { tmux -L "$sock" -f /dev/null "$@"; }

mkdir -p "$home/bin"
now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
ago() { date -u -v-"$1" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -d "-$1" +%Y-%m-%dT%H:%M:%SZ; }
cat >"$home/prs.json" <<JSON
{"data":{"search":{"issueCount":3,"nodes":[
 {"number":4821,"title":"Retry webhook delivery with backoff","url":"https://github.com/acme/webapp/pull/4821",
  "isDraft":false,"updatedAt":"$(ago 2H)","headRefName":"webhook-retry-backoff",
  "repository":{"nameWithOwner":"acme/webapp"},"author":{"login":"me"},"reviewDecision":"APPROVED",
  "mergeStateStatus":"BLOCKED","additions":12,"deletions":3,"mergeQueueEntry":null,
  "statusCheckRollup":{"state":"FAILURE","contexts":{"totalCount":3,"nodes":[
   {"name":"lint","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://github.com/acme/webapp/actions/runs/1/job/2"},
   {"name":"api / test","status":"COMPLETED","conclusion":"FAILURE","detailsUrl":"https://github.com/acme/webapp/actions/runs/1/job/3"},
   {"name":"e2e","status":"IN_PROGRESS","conclusion":null,"detailsUrl":"https://github.com/acme/webapp/actions/runs/1/job/4"}]}},
  "reviews":{"nodes":[]},"reviewRequests":{"nodes":[]},"comments":{"totalCount":2}},
 {"number":4830,"title":"Add rate limit headers to the API","url":"https://github.com/acme/auth/pull/4830",
  "isDraft":false,"updatedAt":"$(ago 40M)","headRefName":"rate-limit-headers",
  "repository":{"nameWithOwner":"acme/auth"},"author":{"login":"me"},"reviewDecision":"REVIEW_REQUIRED",
  "mergeStateStatus":"CLEAN","additions":80,"deletions":9,"mergeQueueEntry":null,
  "statusCheckRollup":{"state":"PENDING","contexts":{"totalCount":1,"nodes":[
   {"name":"CI/CD","status":"IN_PROGRESS","conclusion":null,"detailsUrl":"https://github.com/acme/auth/actions/runs/2/job/1"}]}},
  "reviews":{"nodes":[]},"reviewRequests":{"nodes":[]},"comments":{"totalCount":0}},
 {"number":4790,"title":"Spike: parallel test runner","url":"https://github.com/acme/webapp/pull/4790",
  "isDraft":true,"updatedAt":"$(ago 13d)","headRefName":"parallel-test-spike",
  "repository":{"nameWithOwner":"acme/webapp"},"author":{"login":"me"},"reviewDecision":null,
  "mergeStateStatus":"UNKNOWN","additions":1,"deletions":1,"mergeQueueEntry":null,
  "statusCheckRollup":{"state":"SUCCESS","contexts":{"totalCount":1,"nodes":[
   {"name":"CI/CD","status":"COMPLETED","conclusion":"SUCCESS","detailsUrl":"https://github.com/acme/webapp/actions/runs/3/job/1"}]}},
  "reviews":{"nodes":[]},"reviewRequests":{"nodes":[]},"comments":{"totalCount":0}}
]}}}
JSON
cp "$root/tests/fixtures/queue.json" "$home/queue.json"
cat >"$home/bin/gh" <<SHIM
#!/bin/sh
case "\$*" in
  *'repository(owner'*) cat "$home/queue.json";;
  *'repo view'*) echo acme/webapp;;
  *actions/runs*) echo '{"workflow_runs":[{"head_branch":"gh-readonly-queue/main/pr-4821-0000000000000000000000000000000000000000","status":"in_progress","conclusion":null,"html_url":"https://github.com/acme/webapp/actions/runs/1"}]}';;
  *) cat "$home/prs.json";;
esac
SHIM
chmod +x "$home/bin/gh"

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
  local w h
  w="$(grep -o 'width="[0-9.]*"' "$svg" | head -1 | grep -o '[0-9]*' | head -1)"
  h="$(grep -o 'height="[0-9.]*"' "$svg" | head -1 | grep -o '[0-9]*' | head -1)"
  "$chrome" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=2 \
    --window-size="$w,$h" --screenshot="$out/$name.png" "file://$svg" >/dev/null 2>&1
}

shoot() {
  local name="$1" width="$2" height="$3" keys="$4"
  t resize-window -t "$shot_window" -x "$width" -y "$height"
  t respawn-pane -k -t "$shot_window" "env HOME='$home' PATH='$home/bin:$PATH' '$bin'"
  sleep 1.5
  [ -n "$keys" ] && t send-keys -t "$shot_window" "$keys" && sleep 0.5
  t capture-pane -e -p -t "$shot_window" | python3 "$home/truecolor.py" >"$home/$name.ansi"
  render "$name" --padding 20,64,20,20 --window
}

shoot columns 160 14 ""
shoot details 160 22 "p"
shoot queue   160 22 "lp"
shoot tabs     80 14 ""
echo "wrote $out/columns.png $out/details.png $out/queue.png $out/tabs.png"
