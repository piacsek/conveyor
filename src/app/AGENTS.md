# src/app

The keys, the `S` popup and its actions, the jobs state, refreshes and row order. Rules and working agreements are in the root `AGENTS.md`.

## Behaviour that is easy to break

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
  footer notice, no request.
- **`S` is the one write key.** It opens `Mode::Manage(Menu)`: a popup titled `Manage #N`
  (My PRs) or `Manage run N` (Main builds) with one `Offer` per action, `key  label`, built by
  `App::menu` from the rows on screen. An offer that does not apply keeps its line, dim, with the
  reason after a `·` (`m  merge · #12 is a draft`), and its letter does nothing, so the popup
  explains itself instead of hiding actions. Any key that is not an offer closes it (`nothing
  changed`), `q` included; `q` never quits from the popup. Letters (`m u f t a e x`) avoid every
  Normal-mode key, so a press from habit closes the popup instead of writing. `c` is the one
  exception, and the user's choice: it is the changelog key in Normal mode, and in the popup it
  checks the pull request out, which writes nothing to GitHub and is undone with `git switch -`.
  Keep any new letter that writes to GitHub off the Normal-mode keys. `S` refuses with a
  footer notice while the focused column's last refresh failed (the rows it would judge by are
  older than they look), while any write is in flight (`merging`, `updating`, `rerunning`,
  `changing`, `checking out`: one at a time across all actions) and in Merge queue and Deployed (`S manages My
  PRs and Main builds`). `X`, `M` and `U` were removed in v0.21.0; `S` replaced them.
  A letter re-derives the menu: a different title (another row selected) or a different offer
  under the letter says `the pull request changed; press S again` (`the run changed` in Main
  builds), and an offer that turned into a refusal says its reason. A refresh that lands while
  the popup is open does not redraw its offers: a dim offer that became possible stays dim until
  `S` is pressed again (nothing is sent either way). **Only sensitive actions
  ask `y` second**: merge (`m`), enabling auto-merge (`a` while off: on a merge-queue repo it
  enqueues, which is a merge), close (`x`) and a rebase update (`u` with `prs.update_method =
  "rebase"`, a force-push) go to the confirm dialog; the rest act on the letter, since the popup
  already was the confirmation and each is cheap to undo. Offers per stage:
  - My PRs: `m` merge (`add to merge queue` on a merge-queue base), `u` update/rebase branch, `f` rerun failed jobs (`(k runs)` when more
    than one), `t` mark ready / convert to draft, `a` enable/disable auto-merge, `e` leave the
    merge queue, `c` check out locally, `x` close PR. Main builds: `f` only.
  - **`t`, `a`, `e`, `x` change the status.** `Request::ChangeStatus(StatusTarget)` carries a
    `StatusChange`; `CliGh::status_args` runs `gh pr ready <N> -R <repo> [--undo]` and `gh pr
    close <N> -R <repo>`, and GraphQL mutations by the pull request's node `id` for the rest:
    `enablePullRequestAutoMerge` (with `prs.merge_method` and the head as `expectedHeadOid`),
    `disablePullRequestAutoMerge` and `dequeuePullRequest`. **Never `gh pr merge --auto` or
    `--disable-auto`** (gh v2.103.3 source): `--auto` merges at once when the merge state is
    `CLEAN`, `HAS_HOOKS` or `UNSTABLE`, which would merge without the `y` a merge asks for, and
    `--disable-auto` exits 0 without disabling anything on a pull request that just joined the
    queue. A queued pull request cannot become a draft, and neither can one whose merge just
    landed (`merge_landing`), since it may be queued already. `a` follows GitHub, not a guess: `viewerCanDisableAutoMerge` offers
    disable, else `viewerCanEnableAutoMerge` offers enable, else it is refused (`GitHub does not
    allow it for #N`: a draft, or the repository has auto-merge off). On a merge-queue repo,
    enabling auto-merge enqueues (gh), so a success sets `merge_landing` like a merge. The card's
    meta line reads `auto-merge` while `autoMergeRequest` is set, before the branch name, which
    `meta()` would otherwise cut it behind on a narrow card. `Input::StatusChanged` puts
    `#N converted to draft` (etc.) or `#N: <gh error>` in the footer; a success refreshes My PRs
    (and the queue, except after `mark ready`) and sets `App::follow_up`. No reopen: the default `prs.query`
    is `is:open`, so a closed pull request leaves the column on the next refresh.
- **`c` checks the pull request out locally.** It acts on the letter and sends
  `Request::Checkout(CheckoutTarget)`: the number, the repo and `dir`, the first `path` among
  the `[[repo]]` entries whose name matches the pull request's repository (ASCII case
  ignored), else `None` for the directory conveyor was started in. The worker expands a leading
  `~` from `HOME` (`config::expand_home`) and calls `CliGh::checkout`, which runs two commands
  in that directory: `git remote -v`, then `gh pr checkout <N> -R <repo>`.
  **The remotes are the guard, not `-R`.** Checked on gh v2.103.3: inside a clone of another
  repository `gh pr checkout 1 -R <other>` exits 0, fetches `refs/pull/1/head` from the other
  repository into this clone and switches to it. So the checkout only runs when some remote's
  URL ends in `<owner>/<repo>` (with or without `.git`, ASCII case ignored, any host);
  otherwise, and where there is no repository at all, nothing is fetched and the footer reads
  `<dir> is not a clone of <repo>`, or `conveyor does not run in a clone of <repo>; set
  \`path\` under its [[repo]]`. The first cut asked `gh repo view` instead; review found it
  names the upstream of a fork cloned with `gh repo clone`, so a fork was refused for its own
  pull requests, and it cost a network call. Any remote counts, so a fork with an `upstream`
  remote works for both repositories. A configured path that is not a directory is named (a
  failed spawn would otherwise read as `git not found on PATH`). **The host is not compared,
  on purpose**: only the last two segments of the URL are, so a remote on another host with
  the same `<owner>/<repo>` passes. Comparing hosts would refuse every clone reached through
  an ssh alias (`github-work:<owner>/<repo>`), which is common, to stop a clone of one
  repository that also carries a same-named remote from another host, which is not. For the
  same reason a fork cloned with plain `git clone` (no `upstream` remote) is refused for a
  pull request opened against the upstream: add the remote. The refusal matches git's English
  `not a git repository`; under a localised git it still refuses, with git's own line. **The offer is never dim**: the popup
  is built from the rows on screen and runs nothing, so the refusal lands in the footer.
  `git` is the one program besides `gh` that `CliGh` spawns: under the same spawn lock, with a
  null stdin, logged as `git`. The checkout runs with `GIT_TERMINAL_PROMPT=0`, so an https
  remote without credentials fails instead of prompting under the TUI. Known and left: an ssh
  remote whose key needs a passphrase and has no agent prompts on `/dev/tty`, and a fetch
  that stalls has no deadline, so `S` stays refused (`checking out #N…`) until it ends;
  `GIT_SSH_COMMAND` is not set because it would override the user's own ssh command. A failed
  checkout shows git's own reason, the first stderr line starting with `fatal: ` or `error: `
  (`gits_reason`), else gh's last line, not gh's trailing `failed to run git: exit status N`
  nor the fetch progress above it: a dirty tree reads `error: Your local changes to the
  following files would be overwritten by checkout:`, a branch open in another worktree
  `fatal: '<branch>' is already checked out at '<dir>'`. Nothing more is done about either; the
  user asked for the message only. The message can understate what happened: gh switches to
  an existing local branch before `git merge --ff-only`, so `fatal: Not possible to
  fast-forward` means HEAD did move (read in gh's source, not reproduced). `Input::CheckedOut`
  puts `#N checked out in <path as configured>` (or `in the current directory`) or `#N:
  <reason>` in the footer and refreshes nothing, since a checkout changes nothing on GitHub.
  `App::checking_out` holds `S` back while it runs. The e2e test puts a `git` shim next to the
  `gh` one and logs `pr checkout …` argv with its working directory.
- **`f` reruns failed jobs.** It reruns failed jobs with
  `gh run rerun <run> --failed -R <repo>`: in Main builds the selected run, only when it is the
  newest main build (highest `run_number`) and has settled as a failure or a cancellation.
  An older one is refused: with `cancel-in-progress` its rerun would cancel the newest build,
  and on a workflow that deploys it would put an older sha back over a newer one. In My PRs every run behind the pull request's
  failing checks whose own check suite has a conclusion (`Check::run_finished`) and while no
  newer run of the same workflow and event is still running (`Check::newer_run_running`, a
  suite with no conclusion); `PullRequest::rerunnable_runs`, from `Check::run_id`. A
  `StatusContext` or a non-Actions app has no run and is skipped. The suite, not the
  deduplicated checks, says whether a run is done: a newer run's same-name check can hide an
  older run's pending job. A newer run blocks only while it runs, because rerunning the older
  one could cancel it through a concurrency group; once it has finished it does not, so the
  `✗` a finished run keeps under a newer `labeled` run that skipped the job stays rerunnable.
  The Merge queue has no `S`: a failed merge-group run has already dropped its pull request
  from the queue, so rerunning it merges nothing. The worker asks for every
  run even after one fails and sends one result per run in `Input::Rerun`; the footer says
  which runs failed, and the column it came from refreshes when any run went through. From My PRs it
  also sets `App::follow_up` (see the `u` bullet). A rerun keeps its run
  id, so `age_jobs` and `parse_jobs` (newest `run_attempt`) already carry the jobs over to the
  new attempt. The e2e `gh` shim appends `run rerun …` argv to `reruns.log`; keep any new write
  action behind the same shim so a test can never reach the real `gh`.
- **`m` merges a pull request, and it asks first.** It sets
  `Mode::Confirm(Pending::Merge)` and the confirm dialog asks `merge #N <title>?`. Only a bare `y`
  goes on, every other key cancels (`merge cancelled`), and the Confirm arm sits before the
  `Ctrl-d`/`Ctrl-u` arms so nothing slips past the prompt. `y` works the target out
  again through `App::reconfirm` (the same `menu()` the popup used, so a My PRs refresh that
  failed while the dialog was open refuses too) and sends `Request::Merge` only if the offer
  under `m` is still the same number, repo, title and head sha; otherwise the footer says why,
  or `the pull request changed; press S again`. `confirm_update` and `confirm_status` (close, enable auto-merge, enqueue) use the
  same check. Drafts and queued pull requests are refused in the popup. The worker runs `gh pr merge <N>
  -R <repo> --<prs.merge_method> --match-head-commit <headRefOid>`; the pin refuses a head pushed
  after the card was drawn. On a merge-queue branch gh only warns that the queue sets the
  strategy and enqueues through auto-merge (checked in gh v2.103.3 source). That warning comes
  first on stderr, so `CliGh::merge` (and `change_status`) drop gh's `! ` lines from an error: the footer shows one
  line, and it used to be the warning instead of the reason. `Input::Merged` puts `merge sent for
  #N` or the error in the footer and, on success, refreshes My PRs and the queue.
  **On a merge-queue base `m` never runs `gh pr merge`.** gh enqueues through
  `enablePullRequestAutoMerge`, which GitHub refuses with `Auto merge is not allowed for this
  repository` when the repository has auto-merge off, while its own "Merge when ready" button
  sends `enqueuePullRequest` and works. So when `isMergeQueueEnabled`
  (`PullRequest::merge_queue_enabled`), the offer reads `m  add to merge queue`, keeps the
  draft and queued refusals, asks `add to the merge queue #N <title>?` (the verb first, so a long
  title cannot cut it) and sends `StatusChange::Enqueue`: `enqueuePullRequest` by node id with
  the head as `expectedHeadOid`. A pull request whose checks still run joins as
  `AWAITING_CHECKS`, as on the web. Like enabling auto-merge (`StatusChange::lands_a_merge`), a
  success sets `merge_landing`. `a` is untouched and still follows GitHub's `viewerCan*` flags.
- The confirm dialog and the `S` popup as drawn (`draw_confirm`, `draw_manage`): see `src/ui/AGENTS.md`.
- **`u` updates a pull request's branch.** The label is `update branch`, or
  `rebase branch` when `prs.update_method = "rebase"` (default `merge`). Merge acts on the
  letter; rebase force-pushes the head branch, so it asks `rebase branch of #N <title>?` first.
  The method is read when the menu is built and travels on
  `UpdateTarget`. Refused in the popup: a queued pull request (a push drops it from the queue) and a
  `mergeStateStatus` of `DIRTY` (`#N has conflicts with its base; resolve them locally`). Drafts
  are allowed. `y` re-derives the target like `m` (`the pull request changed; press S again`).
  A merge that just queued the pull request has no `queue_position`
  until the next refresh, and the push would drop it from the queue. For that reason a
  successful merge, or enabling auto-merge, sets `App::merge_landing` and `u` and `t` (to draft)
  are refused with `waiting for My PRs to catch up with #N` until a My PRs fetch that started after the merge (`Input::Fetching`) has
  landed; a fetch already in flight when the merge came back does not count. The worker runs `gh pr update-branch <N>
  -R <repo> [--rebase]`. Checked in gh v2.103.3 source: success prints `PR branch updated` or
  `PR branch already up-to-date` on stdout and exits 0; conflicts print `Cannot update PR branch
  due to conflicts` on stderr and exit 1, both when GitHub already reports the PR conflicting and
  when the update mutation hits one. `CliGh::update_branch` turns those into
  `BranchUpdate::{Updated, UpToDate, Conflicts}`, so conflicts are an outcome, not an error.
  Only `Updated` refreshes My PRs. The e2e shim logs `pr update-branch …` argv.
  **My PRs reload twice after `u`, `f` or a status change.** The first refresh, sent as the action lands, often
  beats GitHub: measured on a rerun, the rerun's Actions checks are missing from the PR query at
  +0 s (the card can read a false `✓` from the checks that are left) and back as `QUEUED` at
  +2 s. So an `Updated` branch, a rerun from My PRs that went through and a status change that
  landed also set
  `App::follow_up` to `now + FOLLOW_UP` (10 s); the first `Tick` at or after it sends one more
  `Request::Refresh(Stage::Prs)` and clears it. A second action before then moves the due time.
  `UpToDate`, `Conflicts`, errors and a Main builds rerun schedule nothing. `app.now` only
  moves on a tick, which only comes while no key is pressed, so a held key delays it.
- **Deployed `b`/`Y` never dead-end.** No fetched main build for the sha → the commit page of
  the deploy repo (`Context::deploy_repo`: the `[[repo]]` with a deploy, else `BuildsStage::repo`).
- **Only http(s) URLs reach the browser.** Check URLs (`detailsUrl`/`targetUrl`) are set by
  whatever GitHub App or CI posted the check, not by GitHub, and `open`/`xdg-open` would launch
  `file://` paths, app bundles or any custom URL scheme handler. `open::web_url` is the one
  check; `App::open` applies it (footer notice, nothing spawned) and `SystemOpener::open`
  applies it again as the last line before the spawn.
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
- **`q` is the only quit key** (besides `Ctrl-C`). `Esc` closes one thing per press (see the
  `Esc` bullet above) and is a no-op with nothing open; it never quits.
- **Row order is per column, and `merge_keeping_order` alone is not enough.** A refresh merges
  the fresh rows onto the old order and appends whatever is new **at the end**, which is right
  for Deployed (config order), and wrong for any column whose order carries meaning: a new
  main build or a jumped queue entry drifted to the bottom over a long session. `Row::ORDER`
  fixes it per type: `Build` is `ByRankDescending` on `run_number`, `QueueEntry` is
  `ByRankAscending` on `position`, `PullRequest` is `ByRankAscending` with queued pull
  requests first (by queue position), then open ones, then drafts, and `Deployment` is
  `AsFetched`. The sort is stable and runs after the merge, so inside a group the previous
  on-screen order holds and a new pull request joins the end of its group. `Column::receive` applies it on every fetch, so the order holds from
  the first draw through every incremental refresh and never depends on what order the API
  happened to return.
- **Keys act on the focused column.** `App::with_focused` dispatches navigation to the
  column of `app.focus`; `p`/`y` use `pull_target` (the pull request of the row — the
  associated one, else the `(#N)` the run's squash title already showed on the card, built
  from `BuildsStage::repo`; an `Err` message when neither exists, never the run itself),
  `b`/`Y` use `build_url`. `Enter` and `o` do nothing: opening a browser on `Enter` surprised
  the user, and the open UX is to be revisited.
  `run()` reports work back through `FnMut(Request)`; `r` sends one `Request::Refresh`, `R`
  one per `Stage::ALL`, and `main` looks each stage up in `refreshers` (a stage without a
  thread is skipped). Adding a stage means one new
  type in `src/app/stages/` that implements `StageView` (a stage that owns a run overrides
  `selected_run` and `runs`, one that offers actions overrides `menu`), a `Row` impl for its
  rows, a `Stage` and a `Rows` variant, a field on `App` and an arm in each of `App::stage`,
  `App::stage_mut` and `Rows::stage`. Nothing else in `src/app/` matches on the stage, and
  `tests/stage_dispatch.rs` fails when an arm appears outside `stages/mod.rs`. A stage reads the
  rest of `App` only through the `Context` it is handed. The UI still draws by stage: arms in
  `column_title`, `draw_column`, `draw_details`, and in `column_name` and `keys_for` of the help.
- **Opens are debounced.** `p`/`b` on the same URL within `OPEN_DEBOUNCE` (1 s of
  `app.now`) is ignored: terminals send a key-repeat stream for a held key, which opened a tab
  per repeat.
- **Refreshes coalesce in the fetcher.** `r`/`R` always send a `Request::Refresh`; each
  fetcher thread wakes through `refresh::wait`, which takes one signal and drains the rest, so
  a burst costs at most one fetch beyond the one running, and a held `r` refetches back to back
  until released, with nothing left queued after. It used to queue a fetch per key repeat, which
  kept running long after the key came up. A time window in `App` was rejected: `app.now` only
  moves on a `Tick`, which never comes while keys arrive under 250 ms apart, and skipping a
  column mid-fetch dropped an `r` meant to catch a change the running fetch predates.
- Refresh feedback (the footer spinner and the `refreshed` wording): see `src/ui/AGENTS.md`.
- **The keymap pairs the pull request and the build.** `p`/`b` open them, `y`/`Y` copy them,
  `d` is the details pane, `Ctrl-d`/`Ctrl-u` scroll it. `y/Y`, `r/R`, `?/c` and
  `C-d/C-u` each share one line of `KEYS`; `L` and `S` have one each, so the help needs 17 rows.
- The help pane, the changelog pane and the cards: see `src/ui/AGENTS.md`.
- The spawn lock, call logging and `gh` never inheriting the terminal: see `src/sources/AGENTS.md`.
- Deployed rows keeping their last known sha (`keep_last_known`) and `behind_main`: see `src/sources/AGENTS.md`.

## Jobs state

- **Jobs**, for the selected run of whichever of Main builds and Merge queue is focused
  (`App::selected_run`) and eagerly for the first `EAGER_JOBS` failing main builds:
  `repos/{r}/actions/runs/{id}/jobs?per_page=100`
  (`fetch::fetch_jobs`, `model::jobs::parse_jobs`, failures first). `run()` asks through
  `App::jobs_needed()` after every input, so a selection move, a focus change and a refresh of
  either column all trigger it; results land as `Input::Jobs` in `App::jobs`
  (`Loading`/`Ready`/`Stale`/`Refreshing`/`Failed`). The pane being open is **not** a condition:
  both columns' cards render the failing job, which is the whole point of fetching it.
  **A refresh never blanks jobs already on screen.** `App::age_jobs` runs on each
  `Data(Builds, …)` and `Data(Queue, …)`, over that column's runs only (aging both columns on
  either refresh fetched every running run twice per interval): an unsettled run's `Ready` becomes `Stale` (still
  drawn, then `Refreshing` while in flight). Every `Stale` run is asked again, selected or not,
  the selected one first and one request per input: a running card nobody selected used to keep
  the job line of its last selection, so a job that failed mid-run still read `●`; dropping it instead made
  the card and pane flash sha → `jobs: loading…` → job on every refresh of a running run.
  `Loading`/`Refreshing` are never asked twice. A refetch that fails keeps the last jobs (`Ready`,
  retried on the next refresh) while the run is still going; once the run has finished it shows
  `Failed` instead, because nothing would ever ask again and the old jobs would stay forever.
  **A finished run is asked once more.** A snapshot taken while the run was running lands as
  `Ready` with running jobs; once the run is settled, `age_jobs` turns a `Ready` still holding an
  unsettled job into `Stale` and keeps every `Stale` (a re-run keeps its run id, so a `Stale`
  under a settled run is the previous attempt), and `stale_run` asks for it whether or not it is
  selected. The old line stays until the answer replaces it: dropping it instead left a passing
  unselected card on its bare sha until someone selected it.
  **A rerun from GitHub is caught by its attempt.** A rerun that starts and finishes between two
  polls never shows as unsettled, so the rule above never fires and the old attempt's jobs
  stayed forever (#32). `Build::attempt` and `Job::attempt` come from `run_attempt` (1 when
  absent); `age_jobs` also turns a settled run's `Ready` into `Stale` when its newest job
  attempt is below the run's. An empty job list carries no attempt and is never re-asked.
  `filter=latest` stamps every job with the newest attempt, the ones a `--failed` rerun did not
  run again included.
- One line per job, newest attempt wins (`parse_jobs`): see `src/model/AGENTS.md`.
