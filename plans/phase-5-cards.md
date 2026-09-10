# conveyor phase 5 — cards, build details, static logo (handoff plan)

## Context

conveyor (`~/projects/conveyor`, public repo `piacsek/conveyor`, Rust + Ratatui, v0.4.0 released)
shows four columns: My PRs, Merge queue, Main builds, Deployed. Phases 0–4 are done and tagged.
The user tried v0.4.0 and asked for four improvements. This plan hands them to a fresh agent.

Read `AGENTS.md` first: every working agreement lives there (ship flow, draft PR, screenshots in
PRs, public-repo scrub gate, tmux e2e traps). `PLAN.md` holds phase history and retros. Do not
put employer data (org, repo names, PR titles, logins, hosts, cluster names) anywhere in the repo;
`scripts/scrub-check.sh` is the first gate and reads `~/.config/conveyor-dev/denylist`.

This file is the versioned plan (`plans/phase-5-cards.md`); update it in place as decisions
change, and record the outcome in `PLAN.md` at the end of the phase.

**Leftover working tree** on branch `phase-5-cards` (draft PR already open by the first ship):
- `tests/prs.rs`: the belt test `a_dim_conveyor_belt_rolls_at_the_bottom_right` was replaced by a
  red test `a_static_dim_logo_with_the_version_sits_at_the_bottom_right`, left unstaged. It is the
  Red step of task 1 below; keep it (adjust its logo assertion to your logo) and make it green.
Run `git status` to confirm.

## The four requests, decoded

1. **Static logo with version, bottom right.** The rolling belt (`ui::belt`, footer right) draws
   attention. Replace it with a static, dim, quirky ASCII conveyor logo followed by
   `conveyor v<CARGO_PKG_VERSION>`. Nothing in the footer may change between ticks.
2. **Multi-line cards, no numbers, gentle selection.** Every column item becomes a two-line card
   (three for Deployed if needed). Drop the `1-9` row numbers and the `1-9` jump keys. The selected
   card is marked with a colored left bar and a bold first line, not `> ` and not reverse video.
   Dim second line. Colors only from the ANSI palette.
3. **Richer build details on `p`.** The Main builds details pane fetches the run's jobs lazily
   and lists them failures first with durations and the failed step; keeps run, status, actor,
   PR, sha, URL.
4. **A key that opens the selected build in the browser.** `b` opens the build behind the card in
   every column (see semantics below). `Enter`/`o` keep opening the item itself.
5. **`R` refreshes every column** (`r` keeps refreshing only the focused one).
6. **One spinner, bottom left.** The braille spinner leaves the column titles. While any column
   has `refreshing == true`, the spinner is drawn at the start of the footer, before the
   `refreshed …` text (e.g. `⠋ refreshed at 15:11:42`); when nothing is refreshing, no glyph.

## Architecture you are extending (verified 2026-09-10, read these files)

- `src/app.rs`: `Column<T: Row>` (state, error, refreshing, list: `ListState`, filter), `Row`
  trait (`key`, `url`, `matches`), `App` with one `Column` per `Stage` (`prs`, `queue`, `builds`,
  `deployed`), `Stage::ALL`, `Input::{Key, Fetching, Data, Tick}`, `Action::{Continue, Quit,
  Open, Copy, Refresh}`, `Mode::{Normal, Filter, Help}`, `App::with_focused` dispatch,
  `selected_target()` → `(number, url)`, `behind_main`, `keep_last_known`, `OPEN_DEBOUNCE`,
  `run(terminal, app, inputs, opener, refresh: FnMut(Stage))`.
- `src/ui.rs`: `draw` → `draw_tabs` or four `draw_column`; `draw_list<T>` renders `ListItem`s
  through a per-stage row fn (`pr_row`, `queue_row`, `build_row`, `deployed_row`) that all call
  `line(number, glyph, text, right, width)`; `HIGHLIGHT = "> "` via `List::highlight_symbol`;
  `draw_details` matches `app.focus` and calls `pr_details`, `queue_details`, `build_details`,
  `deployed_details`; `footer_text`, `draw_footer`, `belt`; `KEYS` drives `draw_help`.
- `src/model/builds.rs`: `Build { id, run_number, status: BuildStatus, sha, title, actor, url,
  started_at, finished_at, pr_number, pull: Option<PullRef> }`, `duration()`, `elapsed(now)`,
  `parse_runs`, `parse_pull_numbers`. `src/model/prs.rs`: `PullRequest` with `checks_detail:
  Vec<Check { name, conclusion: CheckConclusion, url }>` and `checks_failures_first()`.
  `src/model/queue.rs`: `QueueEntry.run_url: Option<String>`. `src/model/deployed.rs`:
  `Deployment { env, image, sha, pull, error, fetched_at }`.
- `src/fetch.rs`: `fetch_prs`, `fetch_queue`, `BuildsSource` (caches workflow id and sha→PR),
  `DeploySource`, `repos`. `src/github.rs`: `Github` trait (`graphql`, `rest(path)`,
  `current_repo`), `CliGh`. `src/main.rs`: `spawn_fetcher(stage, interval, tx, fetch)` returns a
  refresh `Sender<()>`; `refreshers: HashMap<Stage, Sender<()>>`; the `run` refresh closure sends
  on it.
- Tests: `tests/support/mod.rs` (`Harness` with `TestBackend` 160×12, `FakeOpener`, `refreshed:
  Vec<Stage>`, builders `pr`, `entry`, `build`, `deployment`, inputs `prs`, `queue`, `builds`,
  `deployed`, `failed`, `fetching`, `tick_at`, `tick_at_ms`, `NOW = 1_800_000_000`), per-column
  files `tests/{prs,queue,builds,deployed}.rs`, `tests/snapshots.rs` (insta), `tests/e2e.rs`
  (`#[ignore]`, scratch tmux + `gh`/`kubectl` shims), `tests/fetch.rs` (FakeGithub with
  `rest_routes`), parse tests per model. Column tests slice the screen with a `column(screen,
  index)` helper (width/4 per column); the highlighted row helper looks for `│> `.
- `scripts/screenshots.sh` renders `docs/*.png` from shims + synthetic `acme/*` data (freeze →
  SVG → headless Chrome). README embeds columns, details, queue, builds, deployed, tabs.

Jobs API shape (verified on this repo's run 34509123916):
`repos/{owner}/{repo}/actions/runs/{id}/jobs` → `{ total_count, jobs: [ { id, name, status,
conclusion, started_at, completed_at, html_url, steps: [ { name, number, status, conclusion } ] } ] }`.
Step `conclusion` ∈ success | failure | skipped | cancelled | null (running). Capture this run
into `tests/fixtures/jobs.json` (public data; keep only the fields above).

## Design decisions (made; do not re-open unless blocked)

**Logo (task 1).** One footer line, right-aligned, dim, static:
`⟦▣⟧━⟦▣⟧━⟦▣⟧━▸ conveyor v0.5.0` (crates on a belt; pick another glyph set if these misalign in
the snapshot, but stay in Unicode box/geometric shapes, no emoji). `ui::belt(now)` becomes
`ui::logo() -> String` with `env!("CARGO_PKG_VERSION")`. Footer layout stays
`[text | Fill] [logo | Length]`. Remove the "belt" wording from README and AGENTS.md.

**`b` semantics (task 2 in execution order).** Key `b` reuses `Action::Open` (a separate
`OpenBuild` variant would have carried an identical `run()` arm, so it was dropped;
`App::build_url()` picks the URL) with the same debounce (`KEYS` gets `("b", "open build")`;
README keys line too). Per focused column:
- Prs: URL of the first check in `checks_failures_first()` that has a non-empty `url` (failures
  first, so a failing check wins); none → footer notice `no checks yet`.
- Queue: `run_url`; none → notice `no merge-group run yet`.
- Builds: `build.url` (the run). `Enter`/`o` on a build now open the PR URL when `pull` is
  known and the run otherwise (today Enter opens the run; change it and update
  `tests/builds.rs::enter_opens_the_run_and_p_shows_its_details`).
- Deployed: the run URL of the Main builds entry whose `sha` equals the row's sha
  (`app.builds.all()`); none → notice `no main build found for <sha8>`, and a row without a
  sha at all → `no deployed sha to look up`.
`OpenBuild` goes through the same `attempt(app, opener.open(url))` and the same 1 s debounce as
`Open` (share `opened_recently`).

**`R` and the single spinner (task 3 in execution order).**
- `R` → `Action::RefreshAll`; `run()` emits one `Request::Refresh(stage)` per `Stage::ALL`
  (the harness then records four entries). In `main`, the refresh closure already looks each
  stage up in `refreshers`; stages without a thread (Deployed when unconfigured) are skipped
  silently. `KEYS` gets `("R", "refresh all")`; README keys line too.
- Spinner: remove the ` ⠋` suffix from `titled()` in `ui.rs` and the `{spinner} fetching…` body
  prefix from `draw_list` (loading bodies read plain `fetching…` again). Add
  `App::any_refreshing()` (`prs.refreshing || queue.refreshing || builds.refreshing ||
  deployed.refreshing`). `footer_text` keeps its priority order (notice, focused error, filter,
  refreshed-at); `draw_footer` prefixes the text with `{spinner} ` (same `App::spinner()` frame
  source, 100 ms per frame) only when `any_refreshing()`. Tests: the two spinner tests in
  `tests/prs.rs` (`a_refreshing_column_spins_a_braille_glyph_in_its_title_until_data_arrives`,
  `the_initial_fetch_spins_too`) are rewritten to assert the footer's first cell instead of the
  title, plus one test that `Fetching(Queue)` while Prs is focused still shows the footer spinner
  and that it disappears after `Data(Queue, …)`. Snapshots lose the `⠋` in titles and bodies.

**Jobs, lazy (task 4).** New `src/model/jobs.rs`: `Job { id, name, status: BuildStatus (reuse),
started_at, completed_at, url, failed_step: Option<String> }`, `duration()`, `parse_jobs(value)`
(failures first, then running, then the rest, as `checks_failures_first` does). `fetch.rs`:
`fetch_jobs(gh, repo_name, run_id) -> Result<Vec<Job>, String>`.
Flow: `run()` grows a second callback or, better, the existing refresh closure becomes
`FnMut(Request)` with `enum Request { Refresh(Stage), Jobs { run_id: u64 } }`; update the
harness to record `Vec<Request>` (rename `refreshed` → `requests`, fix the two tests that read
it). App state: `jobs: HashMap<u64, JobsState>` with `JobsState::{Loading, Ready(Vec<Job>),
Failed(String)}`. When `p` opens details with Builds focused, or the selection moves while
details are open on Builds, and the selected run has no entry → insert `Loading` and emit
`Action::LoadJobs(run_id)` → `Request::Jobs`. New `Input::Jobs(run_id, Result<Vec<Job>,
String>)` stores the result. `main.rs`: one jobs thread with an `mpsc::Receiver<u64>`; it
needs the repo name → resolve with the same `repos()` fallback the builds fetcher uses (share a
small helper). Re-fetch when the build is not settled (`!build.is_settled()`) and details are
still open: simplest is to drop the cache entry on each `Data(Builds, …)` for unsettled runs.
Details pane for a build: line 1 unchanged (`run N  status  took  started at  by actor`), line 2
PR line, line 3 sha + run URL, then `jobs: loading…` / `jobs: <error>` / one line per job:
`✗ check  19s  failed at: Run cargo test  <job url dim>`. The details pane height is
`ui.details_percent` (40 %); a run with more jobs than lines scrolls nothing, so put failures
first and cap at the visible height minus the three header lines.

**Cards (task 5, the big one).** Replace `line(...)` rows with a `card(selected, glyph, title,
meta, right, width) -> Vec<Line>` helper producing two lines:
- line 1: `▌ ✗ #4821 Retry webhook delivery with backoff` — bar `▌` in `Color::Cyan` when
  selected, else a space; glyph colored as today; title bold when selected; truncated with `…`
  by `text::truncate`; right-aligned `right` on line 1 stays (age / eta / `at main` / `↓n`).
- line 2: `  webapp · fix-branch · approved · +12 −3` dim, per column:
  - Prs: `<repo short> · <head_ref> · <review word> · +a −d` (`⇥n` when queued).
  - Queue: `<author> · position n · <state> · enqueued 20m ago`.
  - Builds: `<author or actor> · run N · <took> · <status word>`.
  - Deployed: `#n <author> · <title>` (line 1 is `<env>` + right `at main`/`↓n`; error rows show
    the error dim on line 2 in red).
Render with `ListItem::new(Text::from(lines))`, `List::highlight_symbol("")` (the bar is drawn
by the card itself from `Some(i) == column.list.selected()`), keep `ListState` for scrolling.
Remove `row_number`, the `1-9` arm in `handle_normal_key`, the `("1-9", …)` KEYS entry, and
`digits_jump_to_the_numbered_row` in `tests/prs.rs`. Update every test that asserts `│> 1 ✓ …`
or slices rows by index: the `highlighted_row` helper looks for `│▌`, card tests read line pairs,
`tests/snapshots.rs` snapshots are re-recorded (`INSTA_UPDATE=always cargo test --test
snapshots`, then read the `.snap` diff before committing). Default harness height 12 fits three
cards; raise `Harness::with_size(160, 24)` where a test needs more. Column width is unchanged
(38 inner at 160 cols), so line 1 text budget is about 30 chars: keep test titles short.

## Execution order (one `scripts/ship.sh` per task; strict TDD, outside-in from the harness)

1. Static logo + version: make the existing red test green; delete `belt`; snapshots update.
2. `b` opens the build: tests per column in `tests/{prs,queue,builds,deployed}.rs`, then the
   `Enter`-on-build change; help + README keys.
3. `R` refresh-all and the single footer spinner: introduce `Request` here (needed by `R`
   only as `Refresh(stage)`, extended by task 4), rewrite the two spinner tests, add the
   cross-column spinner test, re-record snapshots.
4. Jobs: `tests/parse_jobs.rs` on `tests/fixtures/jobs.json` (capture with
   `gh api repos/piacsek/conveyor/actions/runs/34509123916/jobs`, trimmed to the fields listed);
   `tests/fetch.rs` for `fetch_jobs` argv/path; `tests/builds.rs` for `p` emitting
   `Request::Jobs`, `Input::Jobs` rendering, loading and error lines; `main.rs` thread; e2e shim
   route `*actions/runs/*/jobs*` → jobs fixture and an assertion in `tests/e2e.rs`.
5. Cards: one commit per column is fine (Prs first, then Queue, Builds, Deployed), or one commit
   for the `card` helper + Prs and one for the rest; snapshots re-recorded once at the end;
   `scripts/screenshots.sh` re-run; README screenshots refreshed.
6. Docs: README (keys incl. `b` and `R`, cards, details, logo, footer spinner), AGENTS.md (cards
   recipe replaces the row recipe; `Request` flow; jobs data source; logo; "Refresh feedback"
   paragraph rewritten for the footer spinner), PLAN.md phase 5 outcome + retro notes,
   `Cargo.toml` 0.5.0, commit `Release v0.5.0`. Then PR body with commit-pinned screenshots, wait for CI,
   `gh pr merge --rebase --delete-branch`, `git tag v0.5.0 && git push origin v0.5.0`, watch the
   release workflow, `brew update && brew upgrade conveyor`, `brew audit --strict --online
   --formula piacsek/tap/conveyor`.

## Verification

- `scripts/gates.sh` green at every ship (scrub-check, fmt, clippy `-D warnings`, tests, ignored
  e2e with tmux). CI green on the PR and on `main` after the merge (a green PR run is not proof
  against timing bugs; see the fork-race note in AGENTS.md).
- Manual, with the user's own `~/.config/conveyor/config.toml` (already in place; never commit
  its contents): `conveyor-dev` shows four columns of two-line cards, no numbers; moving with
  `j/k` moves the cyan bar; `p` on a build shows `jobs: loading…` then the job list within a
  second or two; `b` on a PR opens a check run, on a build opens the run, on a deployed row opens
  the matching main build; the footer bottom right reads the logo and `conveyor v0.5.0` and does
  not move; Enter on a build opens its PR; `R` makes the braille spinner appear at the bottom
  left for a second or two while all four columns refetch, and no column title spins.
- Screenshots regenerated from the synthetic fixture only; scrub-check clean; PR body embeds
  `docs/columns.png` and `docs/builds.png` at the final commit sha.

## Risks and notes for the executing agent

- Editing Rust with Python string replacement broke twice in earlier phases when `cargo fmt`
  had reflowed the target; prefer rewriting whole functions or files, and run `cargo fmt` before
  `ship.sh` (ship.sh runs it too).
- The auto-mode permission classifier blocks force-pushes, history rewrites and commands that
  mention the denylisted terms; none are needed in this phase.
- Keep 40-column-pane truncation in mind for every card test; assert on prefixes, not full
  titles.
- `chrono` is only for the UTC offset in `main`; do not spread it.
- Version bump last; `cargo test` reads `CARGO_PKG_VERSION` at compile time, so the logo test
  passes at any version.
