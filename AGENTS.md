# AGENTS.md

Notes for agents and humans working on `conveyor`. Keep this file short and
factual; verify against the code before treating anything here as live state.

## What this is

A Rust + Ratatui TUI that shows where a change is on its way to production:
my open pull requests, the merge queue, the last main builds, and what each
environment runs. One column per stage, side by side; a focused column,
actions on the selected row. `conveyor config` prints the effective config.
`PLAN.md` holds the phase history, retros, and the backlog.

## This repository is public

No employer data, ever: no org or repo names, PR numbers or titles, logins, hostnames, cluster
or kube-context names, ticket prefixes. Fixtures come from this repo's own public PRs;
screenshots and TUI tests use invented `acme/*` data; plans and docs use `<owner>/<repo>`
placeholders. `scripts/scrub-check.sh` (first gate) fails when a tracked file matches the
local denylist at `~/.config/conveyor-dev/denylist` (kept outside the repo on purpose; the
check is skipped where the file is absent, so CI does not enforce it).

## Layout

```
src/main.rs      CLI dispatch, config load, wiring
src/cli.rs       `conveyor` (TUI) | `config` | `--help` | `--version`; hand-rolled, no clap
src/config.rs    Config (serde + toml, deny_unknown_fields), XDG path, `CONVEYOR_CONFIG` override
src/github.rs    Github trait (`graphql`, `rest`, `current_repo`), CliGh spawns `gh api …` / `gh repo view`, error mapping
src/fetch.rs     repos() (config or cwd), fetch_prs, fetch_queue (+ merge-group runs via REST), BuildsSource (caches workflow id and sha→PR), DeploySource (per-env kubectl + cached sha→PR)
src/model/prs.rs PullRequest, CheckState, Check, ReviewDecision, MergeState; lenient `parse` of gh JSON
src/model/queue.rs Queue, QueueEntry, QueueState, MergeGroupRun; `parse`, `parse_merge_group_runs`, `attach_runs`
src/model/builds.rs Build, BuildStatus, PullRef, Builds; `parse_runs`, `pr_number_from_title`, `parse_pull_numbers`
src/model/deployed.rs Deployment, Deployed; `sha_from_image` (40-hex tag or `-<sha>` suffix)
src/kube.rs      Kube trait (`image(env)`), CliKubectl (`kubectl --context … get deploy … -o jsonpath`, 10 s timeout)
src/app.rs       Column<T: Row> (state, error, list, filter), App (one Column per stage, focus, Mode), Input::{Key, Data, Tick}, run()
src/ui/mod.rs    draw: 4 columns or tabs below `4 × ui.min_column_width`
src/ui/style.rs  the shared vocabulary: BAR, INDENT, dim(), glyphs and status words, short_repo, sha8
src/ui/card.rs   card() and meta(): one title line plus indented span lines
src/ui/columns.rs column titles, blocks and draw_list
src/ui/rows.rs   one card builder per stage: pr_row, queue_row, build_row, deployed_row
src/ui/details.rs the `p` pane per stage, including the job list
src/ui/footer.rs footer text, the refresh spinner and the static logo
src/ui/help.rs   KEYS drives the help view
src/open.rs      Opener trait; SystemOpener (`open`/`xdg-open`, `pbcopy`/`xclip`)
src/text.rs      truncate/pad_right with `…`, age()
src/queries/     GraphQL documents, `include_str!`ed
scripts/gates.sh              the quality gates; fails loudly, never pipe it through tail
scripts/scrub-check.sh        denylist grep over tracked files (see "This repository is public")
scripts/dev-install.sh        release build symlinked as ~/.local/bin/conveyor-dev
scripts/ship.sh               gates + dev-install + commit + push, aborts on any failure
plans/                        one versioned plan file per phase (see Working agreements)
scripts/homebrew-formula.sh   prints the tap formula for a released version
scripts/screenshots.sh        renders docs/*.png from a gh shim + synthetic fixture (truecolor.py converts ANSI)
tests/           outside-in: `tests/cli.rs` runs the real binary; TUI tests drive run() with a TestBackend
```

## Data sources (verified 2026-09-10, all through `gh`)

- **My PRs**: GraphQL `search(query: <prs.query>, type: ISSUE)`. Useful fields:
  `number title isDraft reviewDecision mergeStateStatus statusCheckRollup{state contexts}`
  `mergeQueueEntry{position state} updatedAt url headRefName repository{nameWithOwner}`.
  `contexts` is a union of `CheckRun` (`name status conclusion detailsUrl`) and
  `StatusContext` (`context state targetUrl`). `mergeStateStatus` is often `UNKNOWN`
  (GitHub computes it lazily): show it as `—`, never as an error.
- **Merge queue**: one query per repo (`src/queries/queue.graphql`) plus one REST call
  `repos/{r}/actions/runs?event=merge_group&per_page=30`; runs are matched to entries by the
  `pr-<N>-` segment of `head_branch` and the newest run wins. The REST call is optional: a
  failure leaves the entries with the rollup state from GraphQL. A repository without a queue
  (`mergeQueue: null`) is a column error, not a crash. `tests/fixtures/queue.json` is
  **synthetic** (built from the schema; no public repo with a queue was at hand) and
  `queue-empty.json` is a real empty response.
- **Merge queue schema**: `repository.mergeQueue.entries` (`position state enqueuedAt
  estimatedTimeToMerge solo jump headCommit{oid} pullRequest{number title author}`).
  Entry `state` ∈ AWAITING_CHECKS | LOCKED | MERGEABLE | QUEUED | UNMERGEABLE. The
  merge-group workflow runs (`gh run list --event merge_group`) live on branches
  `gh-readonly-queue/main/pr-<N>-<base sha>`; their `displayTitle` is the workflow name.
  Rulesets returned nothing for the probed repo (classic protection); the GraphQL object is the source.
- **Main builds**: `repos/{r}/actions/workflows/<file>/runs?branch=main&event=push`. Many
  workflows run on main (`issue_comment` bots, per-app deploys), so the build workflow is
  per-repo config (default `CI/CD`). Squash titles end in `(#N)`; `repos/{r}/commits/{sha}/pulls`
  maps the rest.
- **Main builds**: `repos/{r}/actions/workflows/{id or file}/runs?branch=main&event=push&per_page=N`.
  `main_workflow` may be a display name (resolved once through `actions/workflows?per_page=100`,
  matched on `name` or the file basename) or a `.yml` file used directly. The default `CI/CD`
  also tries `cicd.yml`, `CI`, `ci`, `ci.yml`, `build`, `build.yml`, `main.yml` in that order
  (`fetch::DEFAULT_WORKFLOWS`) so an unconfigured repo usually finds its build workflow. Each run's PR comes
  from the squash suffix `(#N)` in `display_title`, else `repos/{r}/commits/{sha}/pulls`
  (first PR; cached per sha for the life of the process; rebase merges have no suffix, which is
  why this repo's own runs exercise the fallback). `updated_at` stands in for the finish time;
  running and queued runs show elapsed time instead. `tests/fixtures/runs.json` and
  `commit-pulls.json` are real captures from this repo.
- **Jobs**, only for the build whose details pane is open: `repos/{r}/actions/runs/{id}/jobs?per_page=100`
  (`fetch::fetch_jobs`, `model::jobs::parse_jobs`, failures first). `run()` asks through
  `App::jobs_needed()` after every input, so `p`, a selection move and a Builds refresh all
  trigger it; results land as `Input::Jobs` in `App::jobs` (`Loading`/`Ready`/`Failed`).
  An unsettled run's cache entry is dropped on each `Data(Builds, …)` so it is asked again.
  A job's failed step is the first step with a failing conclusion.
  `tests/fixtures/jobs.json` is a real capture of this repo's run 34509123916, trimmed to
  `id name status conclusion started_at completed_at html_url steps[]`.
- **Deployed**: `kubectl --context C -n NS get deploy D -o jsonpath={.spec.template.spec.containers[0].image} --request-timeout=10s`
  per `[[repo.deploy.env]]`; verified live 2026-09-10: the tag is the bare 40-hex commit sha
  (a GitOps image updater writes it; preview deployments use `<label>-<sha>`, which
  `sha_from_image` also accepts). Errors are per row, never per column: an expired session
  or a missing deployment (`Error from server (NotFound): deployments.apps "x" not found`)
  marks that env `✗`, keeps its last known sha/PR (`App::keep_last_known`) and shows the
  message in the footer when the row is selected. `behind_main` is the sha's index in the
  Main builds column (0 = `at main`), so it needs that column loaded and only sees the last
  `builds` runs. kubectl may itself start a teleport browser login when the session is
  expired; the 10 s timeout returns the row to an error instead of hanging.
- Parse leniently: unknown fields ignored, unknown enum values → `Unknown`.
  `tests/fixtures/` carries verbatim `gh` output; refresh it when GitHub changes shape.

## Behaviour that is easy to break

- **`--help`/`-h`/`help` and `--version`/`-V`/`version`** print to stdout and exit 0 before
  the config is read, so a broken config never hides them.
- **Config is loaded before any subcommand runs.** Missing file = `Config::default()`; an
  unreadable file or unknown key is an error naming the file (`deny_unknown_fields` on every
  table). `conveyor config` exits 1 on it and doubles as the reference for defaults.
- **Errors stay inside the TUI.** A failed action or fetch sets an error drawn in the footer
  or column title; the app never exits on it.
- **Draw before the first read.** `run()` renders, then waits for input.
- **`gh` never inherits the terminal.** `CliGh::run` gives it a null stdin, so a `gh` that
  wants to prompt (no auth, `HOME` pointing elsewhere) fails fast instead of hanging the
  fetch thread behind `fetching…`.
- **Keys act on the focused column.** `App::with_focused` dispatches navigation to the
  `Column` of `app.focus`; `Enter`/`y` use `selected_target` (a build opens the PR it merged
  when one is known, else the run), `b` uses `build_url`.
  `run()` reports work back through `FnMut(Request)`; `r` sends one `Request::Refresh`, `R`
  one per `Stage::ALL`, and `main` looks each stage up in `refreshers` (a stage without a
  thread is skipped). Adding a stage means a new
  `Column<T>` field, a `Row` impl, a `Rows` variant, and arms in `receive`, `focused_*`,
  `selected_target`, `with_focused`, `column_title`, `draw_column` and `draw_details`.
- **The `gh` shim in e2e and screenshots dispatches on the query text.** Match the queue query
  on `repository(owner`, not `mergeQueue`: the PR query also contains `mergeQueueEntry`.
- **Opens are debounced.** `Enter`/`o` on the same URL within `OPEN_DEBOUNCE` (1 s of
  `app.now`) is ignored: terminals send a key-repeat stream for a held key, which opened a tab
  per repeat.
- **Refresh feedback.** Fetchers send `Input::Fetching(stage)` before each fetch and set the
  column's `refreshing` flag; `Data` clears it. One braille spinner (frame from `app.now`, 100
  ms per frame, so the 250 ms tick advances it) is drawn at the very start of the footer while
  `App::any_refreshing()` holds, whichever column is focused; column titles and loading bodies
  carry no spinner. The footer says
  `refreshed just now` for 3 s, then `refreshed at HH:MM:SS` local time
  (`utc_offset_secs` from `chrono::Local` in `main`, 0 in tests). The bottom right holds the
  static `ui::logo()` with the crate version; nothing there changes between ticks.
- **Cards, not rows.** `ui::card(selected, glyph, title, right, rest, width)` renders a title
  line (`▌`/space, glyph, bold-when-selected title padded, right-aligned figure) plus one
  indented line per `rest` entry, each a `Vec<Span>` so a line can be dim, red or a mix. A
  per-stage row fn builds them (`pr_row`, `queue_row`, `build_row`, `deployed_row`) and
  `draw_list` wraps each in `ListItem::new(Text::from(lines))` with no `highlight_symbol`:
  the bar is drawn by the card from `Some(i) == column.list.selected()`. Cards carry what a
  browser trip would otherwise cost: the unhappy checks, the failed job and step, the
  deployed sha. Prefer adding a line there over adding one to the details pane.
- **Colors** come from the ANSI palette so terminal themes apply. Do not hardcode hex.

## Testing traps hit so far

- tmux is a test harness, not a dependency: only the `#[ignore]` e2e tests use it, they panic
  with an install hint when it is missing, and `gates.sh` skips them without tmux (CI has it).
- `tests/kube.rs` shim tests take the same kind of mutex as `tests/github.rs` (fork race).
- `tests/e2e.rs` runs the real binary in a scratch tmux server (`-L`, `-f /dev/null`) with
  `HOME` in a tempdir and a `gh` shell shim that prints `tests/fixtures/prs.json`. The pane
  command must be `env HOME=… PATH=… <binary>`: `respawn-pane -e PATH=…` looked right but
  the pane's login shell rebuilt `PATH` (macOS `path_helper` and rc files), the shim was
  never found and the real `gh` ran. Sockets are per test so parallel tests never share a
  server.
- **`gh` shim tests share one process, so they race on fork.** `tests/github.rs` writes a
  shell script per test and spawns it; on Linux CI one test's `fs::write` was still open when
  another test's `Command` forked, the child inherited the fd and `exec` failed with
  `ExecutableFileBusy` ("Text file busy"). It never showed on macOS and passed on the PR run,
  then failed on `main` after the rebase merge. Every test that creates a shim now takes the
  `SHIMS` mutex for its whole body. Any new test that writes an executable and runs it in the
  same test binary must do the same (or run it in a separate process such as tmux, as e2e
  does). Treat a green PR run as no proof against this class: it is timing-dependent.
- A card's title line in a 40-column pane holds about 30 characters after the bar, glyph and
  the right-aligned figure; behaviour tests use short titles, snapshots carry the truncation.
  Card tests read line pairs or triples, not one line per row: with three-line cards row `n`
  starts at screen line `1 + 3n`, and the helper that finds the selected card looks for `│▌`.

## Documentation rule

`README.md` embeds `docs/columns.png`, `docs/details.png`, `docs/queue.png`, `docs/builds.png`,
`docs/deployed.png` and `docs/tabs.png`. After any
visible layout change run `scripts/screenshots.sh` (needs `brew install
charmbracelet/tap/freeze`, Google Chrome, and the FiraCode Nerd Font in `~/Library/Fonts`) and
commit the new images. Freeze lays out an SVG with the font embedded; headless Chrome
rasterises it, because freeze's own PNG output cannot draw Nerd Font glyphs. The script never
reads the real `gh` account: `HOME` is a tempdir and a `gh` shim printing a synthetic
`acme/*` fixture is first on PATH (through `env`, see the e2e trap below). Do not screenshot
real data: this repository is public.

Whenever behaviour changes, check that `README.md` still covers it and is still true: a key,
a default, a subcommand, a config key. The README must stay terse: one line per feature,
defaults in the TOML block, no prose that repeats the code. Depth belongs here.

## Working agreements

All guidelines live in this file, versioned with the code; nothing binding lives only in an
agent's private memory. When the user says "update the guidelines", edit this file.

- **Cadence.** Work runs under `/tdd autonomous`: one branch per phase (`phase-N-<topic>`),
  one commit per task through `scripts/ship.sh`, no pause between tasks, a pause only for the
  end-of-phase retro. Retro outcomes go to `PLAN.md` (facts, decisions) and here (rules).
- **Draft PR first.** The first ship on a branch opens the draft PR assigned to the user;
  every later commit lands on that PR. Before merging, re-read the PR title and body and fix
  what went stale without rewording the user's edits.
- **Screenshots in PRs.** A PR with a visible change embeds the `docs/*.png` screenshots in
  its body as commit-pinned `raw.githubusercontent.com` URLs.
- **Merging and releasing.** Rebase-merge the phase PR into `main` when CI is green, then tag
  from `main` (see Releasing). Never push to `main` directly except the very first bootstrap
  commit of an empty repository, and never force-push.
- **Two binaries.** `conveyor` on PATH is the Homebrew release; `conveyor-dev` is the working
  tree (`scripts/dev-install.sh`). Never `cargo install` the crate into a PATH directory.
- **Plan before code, plans in the repo.** Each phase starts from a written plan, re-planned
  after the previous retro, with its "Verified facts" checked against live tools before use.
  Plans are versioned here: write each one to its own file under `plans/` (for example
  `plans/phase-5-cards.md`) and commit it with the first ship of the phase. Never leave a plan
  only in an agent's private plan directory or memory. `PLAN.md` stays the index of phases,
  retros and the backlog; a phase's outcome is appended there when it closes.

## Development

Strict TDD: one failing test, minimal code, refactor. Outside-in: start from the binary or
from `run()` with scripted inputs, drop to unit tests only for pure helpers. No code
comments; names and tests carry the intent.

Gates, all required before a task is done, wrapped by `scripts/gates.sh`:

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test -- --ignored        # e2e: real binary with a `gh` shim first on PATH
```

Ship with `scripts/ship.sh "<message>"`: `cargo fmt`, gates, `dev-install.sh`, commit, push, all under
`set -e`, refusing to run on `main`, opening the draft PR on the first push of a branch. Do not hand-roll the chain: the interactive shell here is
zsh, where `PIPESTATUS` is undefined and `test "" -eq 0` is true, so a `gates.sh | grep` guard
silently passed a clippy failure into a commit on 2026-09-10.

Run the wrapper, not the commands: piping `gates.sh | tail` once hid a `cargo fmt --check`
failure in tmux-agents and an unformatted commit slipped through. Chaining gates with `&&`
and then `;` before `git commit` does the same (it happened on this repo's bootstrap commit).

Install: `scripts/dev-install.sh` builds the release binary and symlinks it as
`~/.local/bin/conveyor-dev`; `conveyor` on PATH is always the Homebrew release. Never
`cargo install` the crate into a PATH dir. A milestone is done only after the gates pass
**and** `dev-install.sh` has run, so `conveyor-dev` is the committed code.

## Releasing

Releases are GitHub Releases built by `.github/workflows/release.yml` on a `v*` tag: one flat
tarball per target (`conveyor-<target>.tar.gz` holding just the binary; targets
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`) plus `.sha256`,
with generated notes. Asset names carry no version so `releases/latest/download/` links stay
valid. Both macOS targets build on `macos-latest` (Intel cross-compiled); `macos-13` is
retired and queues forever. The workflow refuses a tag whose version differs from
`Cargo.toml`. Every action is pinned to a commit SHA with the tag in a trailing comment;
bump pins with `gh api repos/<owner>/<repo>/git/ref/tags/<tag>`. Workflows default to
`contents: read`; only `publish` gets `contents: write`, `id-token` and `attestations: write`
for `actions/attest-build-provenance` (`gh attestation verify <tarball> --repo
piacsek/conveyor`). CI also runs `rustsec/audit-check`.

1. Bump `version` in `Cargo.toml` (`Cargo.lock` follows on the next build).
2. Run the gates, commit `Release vX.Y.Z`, merge to `main`.
3. `git tag vX.Y.Z && git push origin vX.Y.Z`, then `gh run watch` until `release` is green
   and `gh release view vX.Y.Z` lists three tarballs.
4. The `tap` job regenerates `Formula/conveyor.rb` in `github.com/piacsek/homebrew-tap`
   (local clone `~/projects/homebrew-tap`) with `scripts/homebrew-formula.sh <version>` and
   pushes with the `TAP_TOKEN` repo secret (fine-grained PAT, contents:write on
   homebrew-tap; set per repo with `gh secret set TAP_TOKEN --repo piacsek/conveyor`). If
   that job fails, run the script by hand and push the tap. Check with `brew audit --strict
   --online --formula piacsek/tap/conveyor` after changing the script.

Installed copies: Homebrew puts the release binary in `$(brew --prefix)/bin/conveyor`;
dev builds are only reachable as `conveyor-dev`. `rustsec/audit-check` v2.0.0 (latest as of
2026-09-10) still targets Node 20 and CI prints a deprecation annotation; bump the pin when a
newer release exists.

Never force-push a tag that has a release: the push re-runs `release.yml`, republishes the
assets and rewrites the tap formula for that old version (seen 2026-09-10 after a history
rewrite, when the `v0.0.1` re-run raced the `v0.1.0` run for the formula). If a released
tag must move, delete the release and the tag first, or bump the version instead.

Semver: minor for new columns/keys/config, patch for fixes, major on a config-format break.
