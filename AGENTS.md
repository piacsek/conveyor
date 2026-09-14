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
src/sources/     everything that reaches out to an external service, and nothing else
src/sources/github.rs Github trait (`graphql`, `rest`, `current_repo`, `log_failed`), CliGh spawns `gh api …` / `gh repo view` / `gh run view --log-failed`, error mapping
src/sources/kube.rs   Kube trait (`image(env)`), CliKubectl (`kubectl --context … get deploy … -o jsonpath`, 10 s timeout)
src/sources/fetch.rs  repos() (config or cwd), fetch_prs, fetch_queue (+ merge-group runs via REST), fetch_jobs, fetch_log (LOG_TAIL lines), BuildsSource (caches workflow id and sha→PR), DeploySource (per-env kubectl + cached sha→PR)
src/sources/queries/  GraphQL documents, `include_str!`ed
src/model/prs.rs PullRequest, CheckState, Check, ReviewDecision, MergeState; lenient `parse` of gh JSON
src/model/queue.rs Queue, QueueEntry, QueueState, MergeGroupRun; `parse`, `parse_merge_group_runs`, `attach_runs`
src/model/builds.rs Build, BuildStatus, PullRef, Builds; `parse_runs`, `pr_number_from_title`, `parse_pull_numbers`
src/model/deployed.rs Deployment, Deployed; `sha_from_image` (40-hex tag or `-<sha>` suffix)
src/model/jobs.rs Job, `parse_jobs` (failures first), `failed_step`
src/model/log.rs  `tail`: the failed step's log as gh prints it, minus job/step/timestamp prefixes and ANSI
src/app.rs       Column<T: Row> (state, error, list, filter), App (one Column per stage, focus, Mode, query, jobs, logs, log_open, log_scroll/log_page), Input::{Key, Fetching, Data, Jobs, Log, Tick}, Request::{Refresh, Jobs, Log}, run()
src/ui/mod.rs    draw: 4 columns or tabs below `4 × ui.min_column_width`
src/ui/style.rs  the shared vocabulary: BAR, INDENT, dim(), glyphs and status words, short_repo, sha8
src/ui/card.rs   card() and meta(): one title line plus indented span lines
src/ui/columns.rs column titles, blocks and draw_list
src/ui/rows.rs   one card builder per stage: pr_row, queue_row, build_row, deployed_row
src/ui/details.rs the `d` pane per stage, including the job list; no URLs
src/ui/footer.rs footer text, the refresh spinner and the static logo
src/ui/help.rs   KEYS is the generic table the README mirrors; `keys_for(stage)` swaps the `p`/`b` wording per column
src/open.rs      Opener trait; SystemOpener (`open`/`xdg-open`, `pbcopy`/`xclip`): the OS seam, not a service, so it stays out of `sources/`
src/text.rs      truncate/pad_right with `…`, age(), strip_ansi()
scripts/gates.sh              the quality gates; fails loudly, never pipe it through tail
scripts/scrub-check.sh        denylist grep over tracked files (see "This repository is public")
scripts/dev-install.sh        release build symlinked as ~/.local/bin/conveyor-dev
scripts/ship.sh               gates + dev-install + commit + push, aborts on any failure
plans/                        one versioned plan file per phase (see Working agreements)
scripts/homebrew-formula.sh   prints the tap formula for a released version
scripts/screenshots.sh        renders docs/details.png from a gh shim + synthetic fixture (truecolor.py converts ANSI)
tests/           outside-in: `tests/cli.rs` runs the real binary; TUI tests drive run() with a TestBackend;
                 `tests/readme.rs` pins the README shape and its key table to `ui::help::KEYS`
```

## Data sources (verified 2026-09-10, all through `gh`)

- **My PRs**: GraphQL `search(query: <prs.query>, type: ISSUE)`. Useful fields:
  `number title isDraft reviewDecision mergeStateStatus statusCheckRollup{state contexts}`
  `mergeQueueEntry{position state} updatedAt url headRefName repository{nameWithOwner}`.
  `contexts` is a union of `CheckRun` (`name status conclusion detailsUrl`) and
  `StatusContext` (`context state targetUrl`). `mergeStateStatus` is often `UNKNOWN`
  (GitHub computes it lazily): show it as `—`, never as an error.
- **A queue entry carries its whole merge-group run.** `parse_merge_group_runs` builds a
  `Build` per run through `builds::parse_run` (the two endpoints return the same run shape) and
  `attach_runs` hangs it on `QueueEntry::run`, so the queue's details pane shows the run line
  and its jobs exactly as Main builds does, `b` opens it, and the card carries its run number.
  Once an entry has a run, **the run speaks for the entry**: the card's glyph and status word
  come from `Build::status` (`build_glyph`/`status_word`), not from `CheckState`, and the pane
  drops its `checks:` line because the run line carries the same fact. `CheckState` has no word
  for a cancelled run — a merge queue cancels runs routinely when it re-batches — and squeezing
  one in made the card say `✗ failure` while the pane below it said `cancelled`. The GraphQL
  rollup is left untouched on the entry and is what a run-less entry still reads from.
  `App::selected_run` is what the jobs machinery keys on, so jobs are fetched for whichever of
  the two columns is focused, and `forget_unsettled_jobs` sweeps both.
- **A run that parses to `BuildStatus::Unknown` still speaks for the entry**, so the card reads
  `? · ?` even when the GraphQL rollup had an opinion. That is the contract, not an oversight:
  one source per entry beats two that can disagree, and an unrecognised `status`/`conclusion`
  pair from GitHub is a reason to say "I do not know", not to quietly fall back to a rollup
  computed at a different moment.
- **The newest run wins, by `id`.** `attach_runs` takes the `max_by_key(id)` among the runs
  matching an entry's number, never the first match: a queue that re-batches leaves two runs on
  the same `pr-<N>-` branch prefix, and REST order is not a promise. Ids are monotonic in time
  across the whole of GitHub, which `run_number` is not (it is per workflow, and a merge group
  can run more than one).
- **A build card is four lines**: the number (or `run N` when no pull request is known) with
  the age on the right, then the **title on a line of its own**, then the meta line, then the
  jobs line. The title used to share the first line with the number and the age and was the
  part that got clipped — the one thing on the card you cannot reconstruct from the others.
  Row `n` therefore starts at screen line `1 + 4n`; tests that index a card's lines and any
  test asserting a third card need a terminal taller than the 12-row default.
  It costs density, deliberately: `List` drops an item that does not fully fit, so a card needs
  **four whole rows** — a Main builds pane under 6 rows draws a title over an empty body, and
  the last card of a full column vanishes whole rather than half-drawn. The bold that marks the
  selection moved with the title (`card()` only bolds its first line, which is now just the
  number), so `build_row` styles the title line itself.
  An empty `display_title` renders no title line at all rather than a blank one inside the
  card.
- **A cancelled run blames nobody.** GitHub marks the jobs a cancellation killed as failed, so
  a cancelled card used to carry a red `✗ <job> · <step>` line for work that never actually
  failed. `build_jobs`/`failed_job` drop the job line when the run is `Cancelled` and fall back
  to the sha; the details pane still lists every job, where the conclusions are stated as facts
  rather than as a headline.
- **`⊘` means cancelled, everywhere.** `build_glyph` gives `BuildStatus::Cancelled` its own
  glyph; the grey `-` stays for a *skipped* check (`conclusion_glyph`), which is a different
  fact. Never collapse the two: a cancelled run is work that was stopped, a skipped check is
  work that was never required.
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
  (first PR; see the `Pulls` note below; rebase merges have no suffix, which is
  why this repo's own runs exercise the fallback). `updated_at` stands in for the finish time;
  running and queued runs show elapsed time instead. `tests/fixtures/runs.json` and
  `commit-pulls.json` are real captures from this repo.
- **Jobs**, for the selected run of whichever of Main builds and Merge queue is focused
  (`App::selected_run`) and eagerly for the first `EAGER_JOBS` failing main builds:
  `repos/{r}/actions/runs/{id}/jobs?per_page=100`
  (`fetch::fetch_jobs`, `model::jobs::parse_jobs`, failures first). `run()` asks through
  `App::jobs_needed()` after every input, so a selection move, a focus change and a refresh of
  either column all trigger it; results land as `Input::Jobs` in `App::jobs`
  (`Loading`/`Ready`/`Failed`). The pane being open is **not** a condition: both columns' cards
  render the failing job, which is the whole point of fetching it.
  An unsettled run's cache entry is dropped on each `Data(Builds, …)` and `Data(Queue, …)` so it
  is asked again — **unless it is still `Loading`**, which would put a second identical request
  in flight on every refresh while the first is unanswered.
  A job's failed step is the first step with a failing conclusion.
  `tests/fixtures/jobs.json` is a real capture of this repo's run 34509123916, trimmed to
  `id name status conclusion started_at completed_at html_url steps[]`.
- **Deployed**: `kubectl --context C -n NS get deploy D -o jsonpath={.spec.template.spec.containers[0].image} --request-timeout=10s`
  per `[[repo.deploy.env]]`; verified live 2026-09-10: the tag is the bare 40-hex commit sha
  (a GitOps image updater writes it; preview deployments use `<label>-<sha>`, which
  `sha_from_image` also accepts). Errors are per row, never per column: an expired session
  or a missing deployment (`Error from server (NotFound): deployments.apps "x" not found`)
  marks that env `✗`, keeps its last known sha/PR (`App::keep_last_known`) and shows the
  message in the footer when the row is selected. The row glyph is `✓` at main, a yellow `◐`
  when behind (deliberately not the `●` that means "running" in Main builds; a Deployed row
  never reports a rollout in flight, only which sha is live), `○` for a sha that is not in the
  builds list and `✗` on error. `behind_main` is the sha's index in the
  Main builds column (0 = `at main`), so it needs that column loaded and only sees the last
  `builds` runs. kubectl may itself start a teleport browser login when the session is
  expired; the 10 s timeout returns the row to an error instead of hanging.
- **sha → pull request** goes through `fetch::Pulls`; `BuildsSource` and `DeploySource` each
  own one, so the same sha costs one association call per column and each keeps its own retry
  window. The order is: `repos/{r}/commits/{sha}/pulls`; then, only when that did not parse to
  a pull request (an empty array, but also a non-array or a first entry with no numeric
  `number`), the `(#N)` the caller already knows (`Build.pr_number`, off
  the run's squash title) or else a read of `repos/{r}/commits/{sha}` to take `(#N)` from the
  first line of the message; then `repos/{r}/pulls/{N}`, which is **accepted only if its
  `merge_commit_sha` or `head.sha` is that sha**, so a cherry-pick that kept an upstream squash
  subject cannot attribute the wrong pull request to a row.
  An error from `gh` on the **association** call is not an answer: it returns immediately
  without the fallback, so a rate limit cannot triple the call volume, and nothing is cached.
  An error later in the chain (the commit read, the pull read) is indistinguishable from a
  miss and does suppress the row for one window.
  A hit is cached for the life of the process. A miss is cached for `MISS_RETRY` (120 s) and
  then asked again — the middle ground that fixes the original bug without the cost. Caching a
  miss forever froze `no pull request found for this commit` on a Deployed row read seconds
  after a rollout, because GitHub can briefly report no associated pull request for a commit a
  merge queue has just landed, and only a restart cleared it. Never caching a miss instead cost
  up to two extra calls per unresolved row per refresh **forever**, which on a repo whose main
  branch carries direct pushes ran to thousands of calls an hour against a 5,000/hour limit.
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
- **The filter is a value, editing is a mode.** `App.query` is the live filter on all four
  columns; `Mode::Filter` only means it is being typed (footer cursor `▏`). `Enter` commits
  and returns to Normal with the query kept, `Esc` while typing clears it, `/` reopens it.
  Every column title reads `(visible/total)` while a query is live.
- **`Esc` closes one thing per press**, in order: details pane, zoom, live filter; then it is a
  no-op. It never quits. `1`–`4` focus a column in both layouts.
- **`L` is read-only, and the log has a pane of its own.** It asks `gh run view <run> --job
  <job> --log-failed` for the first failed job of the selected run (Main builds or Merge
  queue), keeps the last `LOG_TAIL` lines per job id in `App.logs`, and toggles `App.log_open`.
  While open, the details area splits in two: details left, `log: <job> · <step>` right,
  scrolled to its last line (`log_scroll = u16::MAX` means "the end", clamped at draw like
  the details), and `Ctrl-d`/`Ctrl-u` move the log pane instead of the details while the pane
  is open. `reset_scroll` puts both panes back at rest on every selection move, focus change,
  refresh, `d` and `Esc`; a log arriving for a run that is not selected does not move the one
  on screen. v0.13.0
  appended the log under the job list, where a 37-job run hid it below the fold: never put
  the thing a key opens where the user has to scroll to find it. No failed job known yet →
  footer notice, no request. Rerunning a job stays a backlog item (write action, confirm UX).
- **Deployed `b`/`Y` never dead-end.** No fetched main build for the sha → the commit page of
  the deploy repo (`App::deploy_repo`: the `[[repo]]` with a deploy, else `builds_repo`).
- **Only http(s) URLs reach the browser.** Check URLs (`detailsUrl`/`targetUrl`) are set by
  whatever GitHub App or CI posted the check, not by GitHub, and `open`/`xdg-open` would launch
  `file://` paths, app bundles or any custom URL scheme handler. `open::web_url` is the one
  check; `App::open` applies it (footer notice, nothing spawned) and `SystemOpener::open`
  applies it again as the last line before the spawn.
- **`gh` never inherits the terminal.** `CliGh::run` gives it a null stdin, so a `gh` that
  wants to prompt (no auth, `HOME` pointing elsewhere) fails fast instead of hanging the
  fetch thread behind `fetching…`.
- **Zoom.** `z` toggles `App::zoom`; `ui::draw` then renders only `app.focus` across the whole
  body and skips the tabs check, so the narrow layout is unaffected. Nothing else reads the flag.
- **The details pane scrolls, the columns do not.** `Ctrl-d`/`Ctrl-u` move `App::details_scroll`
  by half of `details_page` (the pane's inner height, written by `draw_details`); the title
  grows `↑`/`↓` while there is more in that direction. **`draw_details` owns the clamp**
  (`details_scroll.min(overflow)` each frame), so the key handler only ever adds or subtracts:
  the geometry it reads is last frame's, which is fine because `run()` draws before every
  input, and a press with the pane closed is forgotten because `d` resets the scroll. A pane
  too short to have an inside (`height < 2`) reports `page = 0`, no overflow and no arrows,
  so a pane that shows nothing never advertises more. The scroll returns to the top on every
  selection move, focus change, `d`, and on a refresh that lands a different row under the
  selection (`App::receive` compares `selected_key` across the update).
  The keys are handled next to `Ctrl-C` but **after** the `Mode::Help` arm, so the help pane
  keeps its "any key returns" contract; in Filter mode they scroll instead of typing `d`/`u`
  into the query. Because the pane scrolls, the job list is no longer truncated to what fits:
  `job_lines` renders every job.
- **The help pane does not scroll.** `draw_help` renders `KEYS` through a `Paragraph`, so a
  list longer than the pane is silently clipped from the bottom: 13 keys plus two borders
  exactly fills a 16-row terminal. `("q", "quit")` is therefore first in `KEYS`, so the one
  key that gets you out is never the line that disappears. Adding a key means checking the
  smallest terminal you care about, or paginating the pane.
- **`q` is the only quit key** (besides `Ctrl-C`). `Esc` closes one thing per press (see the
  `Esc` bullet above) and is a no-op with nothing open; it never quits.
- **Row order is per column, and `merge_keeping_order` alone is not enough.** A refresh merges
  the fresh rows onto the old order and appends whatever is new **at the end**, which is right
  for My PRs (the search query owns the order) and for Deployed (config order), and wrong for
  any column whose order carries meaning: a new main build or a jumped queue entry drifted to
  the bottom over a long session. `Row::ORDER` fixes it per type: `Build` is
  `ByRankDescending` on `run_number`, `QueueEntry` is `ByRankAscending` on `position`, the
  rest are `AsFetched`. `Column::receive` applies it on every fetch, so the order holds from
  the first draw through every incremental refresh and never depends on what order the API
  happened to return.
- **Keys act on the focused column.** `App::with_focused` dispatches navigation to the
  `Column` of `app.focus`; `p`/`y` use `pull_target` (the pull request of the row — the
  associated one, else the `(#N)` the run's squash title already showed on the card, built
  from `builds_repo`; an `Err` message when neither exists, never the run itself),
  `b`/`Y` use `build_url`. `Enter` and `o` do nothing: opening a browser on `Enter` surprised
  the user, and the open UX is to be revisited.
  `run()` reports work back through `FnMut(Request)`; `r` sends one `Request::Refresh`, `R`
  one per `Stage::ALL`, and `main` looks each stage up in `refreshers` (a stage without a
  thread is skipped). Adding a stage means a new
  `Column<T>` field, a `Row` impl, a `Rows` variant, and arms in `receive`, `focused_*`,
  `pull_target`, `build_url`, `selected_key`, `with_focused`, `column_title`, `draw_column` and
  `draw_details`; a stage that owns a run also needs an arm in `selected_run`.
- **The `gh` shim in e2e and screenshots dispatches on the query text.** Match the queue query
  on `repository(owner`, not `mergeQueue`: the PR query also contains `mergeQueueEntry`.
- **Opens are debounced.** `p`/`b` on the same URL within `OPEN_DEBOUNCE` (1 s of
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
  indented line per `rest` entry, each a `Vec<Span>` so a line can be dim, red or a mix. The
  cyan `▌` is drawn on **every** line of the selected card, so it reads as the card's left
  edge; that plus the bold title is the whole highlight. **Only the focused column draws it**:
  `draw_list` takes `focused` and filters `column.list.selected()` on it, so four cards never
  look selected at once. The unfocused column keeps its selection (and the list keeps it in
  view); it just does not paint it. No background, no reverse video: the
  user asked for a minimal one, twice. A
  per-stage row fn builds them (`pr_row`, `queue_row`, `build_row`, `deployed_row`) and
  `draw_list` wraps each in `ListItem::new(Text::from(lines))` with no `highlight_symbol`:
  the bar is drawn by the card from `Some(i) == column.list.selected()`. Cards carry what a
  browser trip would otherwise cost: the unhappy checks, the failed job and step, the
  deployed sha. Prefer adding a line there over adding one to the details pane. Deployed cards
  go further: one field per line rather than a joined-and-truncated `meta()`, the PR title
  before the author, wrapped with `text::wrap` over at most `DEPLOYED_TITLE_LINES` lines. Card
  height varies per row; the list widget handles it.
- **No URLs in the details pane.** Every line there is text a person reads: the checks, the
  jobs, the sha, the pull request. A URL is 60 columns of noise nobody retypes, and both keys
  that need one (`b`, `p`) already hand it to the browser. `y`/`Y` copy them.
- **The keymap pairs the pull request and the build.** `p`/`b` open them, `y`/`Y` copy them,
  `d` is the details pane, `Ctrl-d`/`Ctrl-u` scroll it. `KEYS` stays 13 lines — `y/Y`, `r/R`
  and `C-d/C-u` each share one — so the help still fits a 16-row terminal.
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
- A card's first line in a 40-column pane holds about 30 characters after the bar, glyph and
  the right-aligned figure; a build card's title gets a line of its own and about `width - 4`.
  Card tests read line groups, not one line per row: a build card is four lines, so row `n`
  starts at screen line `1 + 4n` and a test that wants a third card needs a terminal taller
  than the 12-row default. The helper that finds the selected card looks for `│▌`.

## Documentation rule

`README.md` is minimal and holds only what a user needs, in this order and nothing else: two
sentences on what conveyor does, the one screenshot `docs/details.png` right below them (all
four columns with the details pane open), then `## Installation`, `## Usage` (commands, the key
table, the default config) and `## Development`. The key table mirrors `ui::help::KEYS` word
for word; `tests/readme.rs` fails when the README and the help view drift, when a second
screenshot, a subsection or another section appears, or when the description grows past two
sentences (the help view is the single source: change `KEYS`, then the table). Feature depth, glyph meanings and data-source notes belong here, not there. After
any visible layout change run `scripts/screenshots.sh` (needs `brew install
charmbracelet/tap/freeze`, Google Chrome, and the FiraCode Nerd Font in `~/Library/Fonts`) and
commit the new images. Freeze lays out an SVG with the font embedded; headless Chrome
rasterises it, because freeze's own PNG output cannot draw Nerd Font glyphs. The script adds
Menlo (DejaVu Sans Mono on Linux) as the SVG's fallback font family: FiraCode lacks `✗ ⊘ ▸`,
and Chrome's default fallback is wider, so every row holding one shifted and the borders jogged.
Fixture SHAs look real (`8c1f2a7d…`), not `33333333`. The script never
reads the real `gh` account: `HOME` is a tempdir and a `gh` shim printing a synthetic
`acme/*` fixture is first on PATH (through `env`, see the e2e trap below). Do not screenshot
real data: this repository is public.

Whenever behaviour changes, check that `README.md` is still true: a key, a default, a
subcommand, a config key. Do not add prose for a feature; if it needs explaining, the card or
the help view should carry it.

## Working agreements

All guidelines live in this file, versioned with the code; nothing binding lives only in an
agent's private memory. When the user says "update the guidelines", edit this file.

- **Cadence.** Work runs under `/tdd autonomous`: one branch per phase (`phase-N-<topic>`),
  one commit per task through `scripts/ship.sh`, no pause between tasks, a pause only for the
  end-of-phase retro. Retro outcomes go to `PLAN.md` (facts, decisions) and here (rules).
- **TDD is never skipped.** Every change to `src/` starts with a failing test, run and seen
  red, before the production code is touched; then the smallest change that goes green, then
  refactor. This holds for one-line fixes, security patches, patch releases and "obvious"
  changes alike. Writing the test and the fix in the same edit is not TDD, even when the test
  is then backed out to prove it fails. Show the red run in the update.
- **Draft PR first.** The first ship on a branch opens the draft PR assigned to the user;
  every later commit lands on that PR. Before merging, re-read the PR title and body and fix
  what went stale without rewording the user's edits.
- **Screenshots in PRs.** A PR with a visible change embeds `docs/details.png` in its body as
  a commit-pinned `raw.githubusercontent.com` URL.
- **Review before ready.** Marking a PR ready for review means spawning a sub agent first to
  review the branch diff thoroughly, and acting on what it finds. Give it the intent of the
  change, point it at this file, tell it to run the gates itself rather than trust the claim
  that they pass, and forbid edits, commits and any call that touches the user's real GitHub
  account or clusters. `gh pr ready` comes after the findings are addressed, not before.
- **Merging and releasing.** Rebase-merge the phase PR into `main` when CI is green, then tag
  from `main` (see Releasing). Never push to `main` directly except the very first bootstrap
  commit of an empty repository, and never force-push.
- **Catching up with `main` on a pushed branch is a merge, not a rebase.** A rebase rewrites
  the pushed commits and the next `scripts/ship.sh` push is refused; the no-force-push rule
  then leaves only `git reset --hard origin/<branch>`, `git merge origin/main` and a
  cherry-pick of the new work. The rebase-merge into `main` drops the merge commit anyway.
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

## CI

`.github/workflows/ci.yml` runs on every push to `main` and every pull request: a `check` job
(fmt, clippy `-D warnings`, `cargo test -- --include-ignored` with tmux installed for the e2e
suite) and an `audit` job (`cargo audit`). Learnings from the 2026-09-14 speed-up, when the
pipeline went from ~3 min 20 s to 37 s:

- **Measure before touching anything.** Per-step timings are one call away:
  `gh run view <run id> --json jobs --jq '.jobs[] | .name, (.steps[] | .name, .startedAt, .completedAt)'`.
  The whole pipeline was bound by one step: `rustsec/audit-check` ran `cargo install cargo-audit`
  from source on every run (about 3 min 10 s); everything else together was under 45 s.
- **Never `cargo install` a tool in CI.** Take the release binary instead:
  `taiki-e/install-action` with `tool: <name>` downloads the binary and checks it against the
  sha256 committed in the pinned action; `fallback: none` keeps it from quietly falling back to
  `cargo-binstall` and a source build when the download fails. `audit` is now 8 s. The same
  applies to any future tool (`cargo-insta`, `cargo-deny`).
- **Do not build what another workflow already builds.** `check` ran `cargo build --release`
  (14 s) although nothing consumed it: the e2e tests take the debug binary from
  `CARGO_BIN_EXE_conveyor`, and `release.yml` is what proves the release profile.
- **One test invocation.** `cargo test -- --include-ignored` runs the unit, TUI and e2e suites in
  one pass; two invocations rebuilt the test harness twice for nothing.
- **The floor is setup.** In the 33 s `check` job, about 16 s is job setup + checkout +
  toolchain + `Swatinem/rust-cache` restore and 6 s is `apt-get install tmux`; fmt + clippy +
  tests are about 8 s. Shaving further means caching the apt package or a container image, not touching
  cargo.
- **Trade-off taken.** `rustsec/audit-check` opened a GitHub issue when a new advisory hit
  `main`; with plain `cargo audit` a red `audit` check is the only signal. Add a `schedule`
  trigger if advisories should be caught between pushes.
- **Pins.** Every action stays pinned to a commit SHA with the tag in a trailing comment (see
  Releasing for how to bump one).

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
piacsek/conveyor`). CI also runs `cargo audit` (a prebuilt binary via `taiki-e/install-action`;
the `rustsec/audit-check` action compiled cargo-audit from source on every run, ~3 min of a
~3.5 min pipeline). The `check` job runs `cargo test -- --include-ignored` in one pass; the e2e
tests take the debug binary from `CARGO_BIN_EXE_conveyor`, so no release build there.

Only behaviour changes get packaged. A release (version bump, tag, tarballs, tap) follows a
change a user can observe in the binary: a key, a card, a default, a fix, a config key. Docs,
CI, tests, scripts, refactors, guideline edits and copy-only wording (a word in the help view,
an error message) merge to `main` without a bump and ride the next behaviour release. Do not
tag a docs-only or CI-only `main`.

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
