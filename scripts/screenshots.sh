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
now_s="$(date -u +%s)"
iso() { date -u -r "$1" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -d "@$1" +%Y-%m-%dT%H:%M:%SZ; }
run() {
  local n="$1" status="$2" conclusion="$3" sha="$4" title="$5" started_ago="$6" took="$7"
  local started=$(( now_s - started_ago ))
  printf '{"id":%s,"run_number":%s,"status":"%s","conclusion":%s,"head_sha":"%s","display_title":"%s","html_url":"https://github.com/acme/webapp/actions/runs/%s","actor":{"login":"merge-bot"},"run_started_at":"%s","updated_at":"%s"}' \
    "$n" "$n" "$status" "$conclusion" "$sha" "$title" "$n" "$(iso "$started")" "$(iso $(( started + took )))"
}
{
  printf '{"workflow_runs":['
  run 312 in_progress null 3333333333333333333333333333333333333333 "Speed up CI with a warm cache (#4840)" 95 0; printf ','
  run 311 completed '"failure"' 2222222222222222222222222222222222222222 "Retry webhook delivery with backoff (#4821)" 3600 1097; printf ','
  run 310 completed '"success"' 1111111111111111111111111111111111111111 "Add rate limit headers to the API (#4830)" 7200 1002; printf ','
  run 309 completed '"success"' 0000000000000000000000000000000000000000 "Spike: parallel test runner (#4790)" 90000 930
  printf ']}'
} >"$home/runs.json"
pulls() { printf '[{"number":%s,"title":"%s","html_url":"https://github.com/acme/webapp/pull/%s","user":{"login":"%s"}}]' "$1" "$2" "$1" "$3"; }
pulls 4840 "Speed up CI with a warm cache" bob >"$home/pulls-3333333333333333333333333333333333333333.json"
pulls 4821 "Retry webhook delivery with backoff" alice >"$home/pulls-2222222222222222222222222222222222222222.json"
pulls 4830 "Add rate limit headers to the API" carol >"$home/pulls-1111111111111111111111111111111111111111.json"
pulls 4790 "Spike: parallel test runner" dave >"$home/pulls-0000000000000000000000000000000000000000.json"
jobs() {
  local n="$1" status="$2" conclusion="$3" step="$4"
  printf '{"total_count":2,"jobs":[
 {"id":%s01,"name":"check","status":"%s","conclusion":%s,"started_at":"%s","completed_at":%s,
  "html_url":"https://github.com/acme/webapp/actions/runs/%s/job/%s01","steps":[
   {"name":"Run cargo fmt --check","number":1,"status":"completed","conclusion":"success"},
   {"name":"%s","number":2,"status":"%s","conclusion":%s}]},
 {"id":%s02,"name":"audit","status":"completed","conclusion":"success","started_at":"%s","completed_at":"%s",
  "html_url":"https://github.com/acme/webapp/actions/runs/%s/job/%s02","steps":[]}]}' \
    "$n" "$status" "$conclusion" "$(iso $(( now_s - 1200 )))" \
    "$([ "$status" = completed ] && printf '"%s"' "$(iso $(( now_s - 1181 )))" || echo null)" \
    "$n" "$n" "$step" "$status" "$conclusion" \
    "$n" "$(iso $(( now_s - 1200 )))" "$(iso $(( now_s - 1000 )))" "$n" "$n"
}
jobs 312 in_progress null "Run cargo test" >"$home/jobs-312.json"
jobs 311 completed '"failure"' "Run cargo test" >"$home/jobs-311.json"
jobs 310 completed '"success"' "Run cargo test" >"$home/jobs-310.json"
jobs 309 completed '"success"' "Run cargo test" >"$home/jobs-309.json"
cat >"$home/bin/gh" <<SHIM
#!/bin/sh
case "\$*" in
  *'repository(owner'*) cat "$home/queue.json";;
  *'repo view'*) echo acme/webapp;;
  *'actions/workflows?'*) echo '{"workflows":[{"id":22,"name":"CI/CD","path":".github/workflows/cicd.yml"}]}';;
  *'actions/workflows/'*) cat "$home/runs.json";;
  *actions/runs/*/jobs*) run=\$(printf '%s' "\$*" | sed -E 's#.*/actions/runs/([0-9]+)/jobs.*#\\1#'); cat "$home/jobs-\$run.json";;
  *'/commits/'*) sha=\$(printf '%s' "\$*" | sed -E 's#.*/commits/([0-9a-f]+)/pulls.*#\\1#'); cat "$home/pulls-\$sha.json";;
  *actions/runs*) echo '{"workflow_runs":[{"head_branch":"gh-readonly-queue/main/pr-4821-0000000000000000000000000000000000000000","status":"in_progress","conclusion":null,"html_url":"https://github.com/acme/webapp/actions/runs/1"}]}';;
  *) cat "$home/prs.json";;
esac
SHIM
chmod +x "$home/bin/gh"

cat >"$home/bin/kubectl" <<'KUBE'
#!/bin/sh
case "$*" in
  *ctx-dev*) printf 'ghcr.io/acme/api:3333333333333333333333333333333333333333';;
  *ctx-staging*) printf 'ghcr.io/acme/api:2222222222222222222222222222222222222222';;
  *ctx-prod*) printf 'ghcr.io/acme/api:0000000000000000000000000000000000000000';;
  *) echo 'ERROR: Active profile expired.' >&2; exit 1;;
esac
KUBE
chmod +x "$home/bin/kubectl"
mkdir -p "$home/.config/conveyor"
cat >"$home/.config/conveyor/config.toml" <<'TOML'
[[repo]]
name = "acme/webapp"

[[repo.deploy]]
system = "api"

[[repo.deploy.env]]
name = "dev"
context = "ctx-dev"
namespace = "api"
deployment = "api"

[[repo.deploy.env]]
name = "staging"
context = "ctx-staging"
namespace = "api"
deployment = "api"

[[repo.deploy.env]]
name = "prod"
context = "ctx-prod"
namespace = "api"
deployment = "api"

[[repo.deploy.env]]
name = "uat"
context = "ctx-uat"
namespace = "api"
deployment = "api"
TOML

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

shoot columns 160 18 ""
shoot details 160 26 "p"
shoot queue   160 26 "lp"
shoot builds  160 26 "llp"
shoot deployed 160 26 "lllp"
shoot tabs     80 18 ""
echo "wrote $out/columns.png $out/details.png $out/queue.png $out/builds.png $out/deployed.png $out/tabs.png"
