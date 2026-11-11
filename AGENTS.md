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
src/cli.rs       `conveyor` (TUI) | `config` | `log` | `--help` | `--version`; hand-rolled, no clap
src/config.rs    Config (serde + toml, deny_unknown_fields), XDG path, `CONVEYOR_CONFIG` override, `expand_home`
src/sources/     everything that reaches out to an external service, and nothing else
src/sources/github.rs Github trait (`graphql`, `rest`, `current_repo`, `log_failed`, `rerun_failed`, `merge`, `update_branch`, `change_status`, `checkout`), CliGh spawns `gh api …` / `gh repo view` / `gh run view --log-failed` / `gh run rerun --failed` / `gh pr merge|update-branch|ready|close|checkout` and `git remote -v` (the checkout guard), error mapping
src/sources/kube.rs   Kube trait (`deployment(env)`, `pods(env, selector)`), CliKubectl (`kubectl --context … get deploy|pods … -o json`, a 10 s request timeout and a 15 s `DEADLINE` that kills the process)
src/sources/fetch.rs  repos() (config or cwd), fetch_prs, fetch_queue (+ merge-group runs via REST), fetch_jobs, fetch_log (LOG_TAIL lines), BuildsSource (caches workflow id and sha→PR), DeploySource (kubectl per env on parallel threads + cached sha→PR)
src/sources/queries/  GraphQL documents, `include_str!`ed
src/model/prs.rs PullRequest, CheckState, Check, ReviewDecision, MergeState; lenient `parse` of gh JSON
src/model/queue.rs Queue, QueueEntry, QueueState, MergeGroupRun; `parse`, `parse_merge_group_runs`, `attach_runs`
src/model/builds.rs Build, BuildStatus, PullRef, Builds; `parse_runs`, `pr_number_from_title`, `parse_pull_numbers`
src/model/deployed.rs Deployment, Deployed, Rollout, Condition, Pod, Health; `sha_from_image` (40-hex tag or `-<sha>` suffix), `parse_deployment`, `parse_pods` (problems first), `health`
src/model/jobs.rs Job, `parse_jobs` (failures first), `failed_step`
src/model/log.rs  `tail`: the failed step's log as gh prints it, minus job/step/timestamp prefixes and ANSI
src/app/mod.rs   App (one Column per stage, focus, Mode, query, jobs, logs, log_open, log_scroll/log_page), Stage, Rows, Mode::{Normal, Filter, Help, Changelog, Confirm, Manage}, Input::{Key, Fetching, Data, Jobs, Log, Merged, Rerun, BranchUpdated, StatusChanged, CheckedOut, Tick}, Action, Request::{Refresh, Jobs, Log, Rerun, Merge, UpdateBranch, ChangeStatus, Checkout}; receive, the `focused_*` readers, `pull_target`, `build_url`, open, scroll and filter helpers, `keep_last_known`; re-exports everything below as `app::X`
src/app/column.rs Column<T: Row> (state, error, list, filter), ColumnState, Row and its four impls, Order, `merge_keeping_order`, Navigable
src/app/jobs.rs  JobsState, LogState, `age_jobs`, `jobs_needed` (selected, stale, then the first `EAGER_JOBS` failing builds), `selected_run`, `toggle_log`, `receive_jobs`, `receive_log`
src/app/actions/ the `S` popup: `mod.rs` is the machinery, one file per column for what it offers, one file per action for how it runs
src/app/actions/mod.rs Menu/Offer/Choice, Pending, `ask_to_manage`, `menu` (the one match from the focused column to its offers), `manage`, `confirm`, `reconfirm`, `write_in_flight`, `offer`, `changed`
src/app/actions/prs.rs what My PRs offers: `prs_menu`, `pr_offers`
src/app/actions/builds.rs what Main builds offers: `builds_menu`
src/app/actions/merge.rs MergeTarget, `merge_target`, `confirm_merge`, `receive_merged`
src/app/actions/update.rs UpdateTarget, `update_target`, `confirm_update`, `send_update`, `receive_branch_update`, `conflicts`
src/app/actions/status.rs StatusTarget, `confirm_status`, `send_status`, `receive_status`, the `progress` and `done` wording
src/app/actions/checkout.rs CheckoutTarget, `checkout_target`, `send_checkout`, `receive_checkout`
src/app/actions/rerun.rs Rerun, `rerun_offer` and `rerun_target` (My PRs and Main builds), `send_rerun`, `receive_rerun`
src/app/keys.rs  `handle_key` and the Filter and Normal mode key handlers, `escape`
src/app/run.rs   run(): draw, read one Input, turn each Action into a Request
src/ui/mod.rs    draw: 4 columns or tabs below `4 × ui.min_column_width`
src/ui/style.rs  the shared vocabulary: BAR, INDENT, dim(), glyphs and status words, short_repo, sha8
src/ui/card.rs   card() and meta(): one title line plus indented span lines
src/ui/columns.rs column titles, blocks and draw_list
src/ui/rows.rs   one card builder per stage: pr_row, queue_row, build_row, deployed_row
src/ui/details.rs the `d` pane per stage, including the job list; no URLs
src/ui/footer.rs footer text, the refresh spinner and the static logo
src/ui/dialog.rs the `S` popup (`draw_manage`) and the confirm dialog (`draw_confirm`) for `m`, `x`, enabling auto-merge and a rebase
src/ui/help.rs   KEYS is the generic table the README mirrors; `keys_for(stage)` swaps the `p`/`b` wording per column
src/refresh.rs   `spawn`: one fetcher thread per stage; `wait` sleeps until the interval or a refresh, draining queued signals
src/open.rs      Opener trait; SystemOpener (`open`/`xdg-open`, `pbcopy`/`xclip`): the OS seam, not a service, so it stays out of `sources/`
src/changelog.rs CHANGELOG (the bundled CHANGELOG.md) and `check`, the rules a release entry must meet
src/ui/changelog.rs the `c` pane: wrapped to its width (bullets hang), headings bold without their `#`, scrolled like the details pane
src/log.rs       `path` (CONVEYOR_LOG, else XDG_STATE_HOME/conveyor/<program>.log), `program_name` (argv[0]), Log (`open` rotates to `.1`, `line`, `call`)
src/text.rs      truncate/pad_right with `…`, age(), duration() (coarse, cards), elapsed() (seconds, details pane), strip_ansi()
scripts/gates.sh              the quality gates; fails loudly, never pipe it through tail
scripts/scrub-check.sh        denylist grep over tracked files (see "This repository is public")
scripts/dev-install.sh        release build symlinked as ~/.local/bin/conveyor-dev
scripts/ship.sh               gates + dev-install (unless skippable) + commit + push, aborts on any failure
scripts/dev-install-skippable.sh  exit 0 with the reason when `ship.sh` may leave `conveyor-dev` alone, else 1 with what needs the install
plans/                        one versioned plan file per phase (see Working agreements)
scripts/homebrew-formula.sh   prints the tap formula for a released version
scripts/screenshots.sh        renders docs/details.png from a gh shim + synthetic fixture (truecolor.py converts ANSI)
tests/           outside-in: `tests/cli.rs` runs the real binary; TUI tests drive run() with a TestBackend;
                 `tests/readme.rs` pins the README shape and its key table to `ui::help::KEYS`;
                 `tests/source_size.rs` fails while any Rust file under `src/` is longer than 700 lines, naming it
```

## Data sources (verified 2026-09-10, all through `gh`)

- **My PRs**: GraphQL `search(query: <prs.query>, type: ISSUE_ADVANCED)`. Plain `ISSUE` search
  ignores `OR` and parentheses (the default `(author:@me OR assignee:@me)` returned 0 there); the
  advanced type supports them, dedupes server-side and still accepts plain queries. Useful fields:
  `number title isDraft reviewDecision mergeStateStatus statusCheckRollup{state contexts}`
  `mergeQueueEntry{position state} updatedAt url headRefName repository{nameWithOwner}`.
  `contexts` is a union of `CheckRun` (`name status conclusion detailsUrl`) and
  `StatusContext` (`context state targetUrl`). `mergeStateStatus` is often `UNKNOWN`
  (GitHub computes it lazily): show it as `—`, never as an error.
  **One line per check.** `contexts` lists a check run for every workflow run on the head commit,
  and a workflow can run more than once on the same commit (another `pull_request` event, a
  re-run), so the same job came back two or four times. `newest_checks` keeps the highest
  `databaseId` per (status or check run, workflow name, event, check name), read from
  `checkSuite { app { slug } workflowRun { event workflow { name } } }`; a check run from an app
  that is not Actions has no workflow run, so its app slug stands in for the workflow (two apps
  posting `test` both stay, labelled `<app> / test`). Checks that survive with the same
  name are labelled `<workflow> / <name>`, and `<workflow> / <name> (<event>)` when one workflow
  ran for two events (`on: [push, pull_request]`), so they never look like duplicates.
  `StatusContext`s are already one per context and never collapse into a check run.
  **A cancelled run is dropped once a newer run of its workflow exists.** Per-name dedup alone
  let a job only the old run had survive: a concurrency group cancelled run A, its
  `if: always()` status job failed, and run B had not created that job yet, so the card read `✗`
  while everything else was pending. `superseded_by_a_newer_run` drops every check whose
  `checkSuite.conclusion` is `CANCELLED` when a higher `workflowRun.databaseId` exists for the
  same workflow and event. A run that finished on its own keeps its checks, so a newer run that
  skips a job (a `labeled` trigger) cannot hide a real result. A newer run with no check run yet
  is invisible here, so the old `✗` stays until its first job appears. "Newer" is the higher run
  id, not the later start: re-running an older run that cancels a newer one leaves the newer
  run's leftovers in place. Ordering by check-run id instead was rejected because a cancelled
  run's status job is created at cancel time, often after the newer run's first jobs.
  **The card's check state follows that list, not GitHub's rollup.** `statusCheckRollup.state`
  counts every run on the commit, so a run a concurrency group cancelled kept a PR at `FAILURE`
  after its replacement passed. `prs::overall` reads the deduplicated checks: any failure, else
  any pending, else any unknown, else success; the rollup only answers when there are no
  contexts. The card can therefore read `✓` while GitHub's page still shows a red cross.
  **Each check line carries its time.** `CheckRun.startedAt`/`completedAt` feed `Check::took`:
  finished checks show how long they took, running ones the time since they started (redrawn
  on every tick). A check with no `startedAt`, a `StatusContext` (no timing in the schema) and
  a skipped check show none: GitHub stamps skipped checks with both times (equal on about half
  of them, a second or two apart on the rest), which read as a meaningless `0s`. Job and run lines in the details pane use the same
  `text::elapsed` (`3m59s`, `1h04m`); cards keep the coarse `text::duration` (`3m`) because
  seconds pushed the status word off a 40-column card.
  **Every page of checks is read.** `contexts(first: 100)` is GitHub's cap, and its order is not
  by time: it groups checks by name with the older run first, so a cut at 100 can drop the newer
  half of a pair and let the old result win. `fetch::read_remaining_checks` follows
  `pageInfo.hasNextPage` with `queries/pr_checks.graphql` (`node(id:)`, so the e2e shim's
  `repository(owner` route never catches it), up to `MAX_CHECK_PAGES` extra pages per pull
  request, before parsing. A failed page fails the column, which keeps its last rows.
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
  the two columns is focused, and `age_jobs` sweeps both.
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
  **One line per job, newest attempt wins.** The jobs route's default `filter=latest` goes by
  `completed_at`, so during a re-run the previous attempt's finished job comes back next to the
  new in-progress one under the same name; `parse_jobs` keeps, per name, the jobs of that name's
  newest `run_attempt` (two jobs sharing a name in one attempt both stay).
  A job's failed step is the first step with a failing conclusion.
  `tests/fixtures/jobs.json` is a real capture of this repo's run 34509123916, trimmed to
  `id name status conclusion started_at completed_at html_url steps[]`.
- **Deployed**: `kubectl --context C -n NS get deploy D -o json --request-timeout=10s`
  per `[[repo.deploy.env]]` (the image is `.spec.template.spec.containers[0].image`); verified live 2026-09-10: the tag is the bare 40-hex commit sha
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
  expired; `DEADLINE` (below) returns the row to an error instead of hanging, and it kills that
  login with kubectl, so a login cannot be finished through conveyor: log in from a shell.
  **The request timeout is not a limit on the call, so the process has one.**
  `--request-timeout=10s` bounds one HTTP request, and kubectl retries API discovery: against a
  proxy that stopped answering, each call took 40 s (three failed attempts 10 s apart, then the
  error) and the column sat on `fetching…`. `CliKubectl::run` spawns kubectl in its own process
  group (`process_group(0)`), drains both pipes on threads and at `kube::DEADLINE` (15 s) kills
  the **group** (`kill -KILL -- -<pid>`, the one way to signal a group without a new
  dependency or `unsafe`): the row reads `kubectl did not answer within 15s` and the log line
  ends in `timed out`, followed by `: <stderr lines>` when kubectl had said anything, so the
  log still shows why. The pipes are bounded as well: a helper that outlives kubectl (a login
  plugin) keeps them open, which is what `Command::output()` waits for. Once kubectl has exited,
  its output gets `DRAIN` (500 ms) to arrive; after that the group is killed, which closes the
  pipes, and **kubectl's own answer is used**: its stdout and exit status when it succeeded,
  its stderr as the row's error when it failed. The first cut called that case a timeout and
  threw the answer away. Killing the group, not the process, is what ends a helper: left alive
  it kept one process, two reader threads and two pipes per call for as long as it ran, every
  refresh. A helper that moved itself to another session (`setsid`) escapes the kill; only
  then is an exited kubectl reported as a timeout. The flag stays: it is
  what makes one slow request fail with kubectl's own message before anything is killed.
  **A row's error is what kubectl said, minus its klog lines.** kubectl prints its client-go
  log first on stderr (`E1109 20:36:29.585690   14272 memcache.go:265] "Unhandled Error" …`),
  and a card truncated to its width showed only that. `kube::said` drops lines shaped like
  klog (a severity letter, four digits, a space, and a `.go:` source) unless nothing else is
  left; the log file keeps every line.
- **Deploy health comes from a second call.** After the Deployment, `DeploySource` runs
  `kubectl … get pods -l <matchLabels as k=v,k=v> -o json --request-timeout=10s` and
  `deployed::health(&Rollout, Option<&[Pod]>)` judges the row, in this order: `Alert` when
  replicas are wanted and none is available (`0 of 2 available`), whatever the `Available`
  condition says; `Warning` when the `Available` condition is not `True` but something still
  serves (`1 of 2 available`); `Warning` when a pod that is not
  terminating, `Failed` or `Succeeded` has a container (init containers included) waiting in
  `CrashLoopBackOff`, `ImagePullBackOff`, `ErrImagePull`, `CreateContainerConfigError`,
  `CreateContainerError`, `RunContainerError` or `InvalidImageName` (`1 of 3
  pods CrashLoopBackOff`; `2 of 3 pods stuck` when the reasons differ), or a container that is
  not ready, has restarted and has a last exit (`restarting`: a crashlooping container spends
  part of each cycle running, and a refresh landing there used to read healthy), or the pod
  cannot be placed (`PodScheduled` is `False` with `Unschedulable`: it has no container yet, so
  nothing else fires until the progress deadline, about 10 minutes; with a cluster autoscaler
  this can show for a minute or two of a normal rollout while a node boots), or when `Progressing` is `False` with `ProgressDeadlineExceeded`
  (`rollout stalled`), or when `ReplicaFailure` is `True` (`replicas cannot be created`: a quota
  error leaves no pod to show as stuck); `Unread` when the pods were not read; else `Healthy`.
  The pod's waiting reason and `last exit` come from the container that makes it stuck, not
  from the first one that waits. A pod is ready when its `Ready` condition is `True` (a
  readiness gate can hold a pod whose containers are all ready); the containers decide only
  when the condition is absent. The `restarting` word can be stale: `lastState.terminated`
  stays for as long as `restartCount > 0`, so a pod that crashed long ago and is unready now
  for another reason still reads `restarting` with its old exit. Two states still read healthy
  and are accepted: a crashlooping container with no readiness probe is `ready` whenever it
  runs, so a refresh landing there shows no icon; and a Deployment of four or more replicas
  that lost one pod to a failing readiness probe stays `Available` (the default 25% allowance),
  because warning on `available < desired` would flag every normal rollout, which the user
  declined.
  **Red means nothing is serving, not "Kubernetes says not Available".** `Available` is
  `availableReplicas >= replicas - maxUnavailable`, and the default 25% rounds down to 0 for 2
  or 3 replicas, so one pod down flipped it and the card read `1 of 2 available` in red while
  the other pod served; the user chose red for an outage only. It also catches the reverse: with
  `maxUnavailable` at 100% the condition stays `True` with nothing running.
  The health icon sits in the card's icon slot after the version glyph, which keeps meaning
  "which sha is live": yellow `⚠`, red `‼`, grey `?`. A card with no icon means the pods were
  read and are fine. **Pods that cannot be read are never silent**: a failed pods call (RBAC, a
  timeout), unparseable output or a selector without `matchLabels` sets `pods_error`, not the
  row's `error`, and the card shows the grey `?` with `pods not read: <error>`; under a louder `‼` or `⚠` the
  icon gives way but the line stays. The user asked
  for that: without it a crashlooping version looked healthy wherever `get pods` was denied.
  **The environments are read at the same time.** `DeploySource::fetch` gives each environment
  a scoped thread (`std::thread::scope`) for its two kubectl calls, the Deployment then the
  pods, and builds the rows on the fetcher thread once every thread is back, in config order.
  The sha → pull request lookup runs there, after the threads, so `Pulls` and `gh` stay
  single-threaded and the row order never depends on which cluster answered first; `Kube`
  implementations must be `Sync`. They were read one after another until v0.23.2, so every
  unreachable environment added its own wait in front of the healthy rows. The kubectl part
  of a fetch now takes as long as its slowest environment (the pull request lookups follow,
  one environment after another: up to three `gh` calls for a sha not seen before, none for a
  cached one): about 15 s when a Deployment call hangs
  (`DEADLINE`, plus up to 500 ms to collect stderr; the pods call is skipped when the
  Deployment call fails), about 30 s when a Deployment call answers and its pods call hangs,
  however many environments are down. The rows still land together: a healthy row waits for
  the slowest one, because a column takes all its rows in one `Rows::Deployed`. A panic in a
  reader thread is passed on to the fetcher thread (`resume_unwind`), as it was when the read
  ran there. With an
  expired session every environment's kubectl starts at once, so a login plugin may be started
  once per environment at the same moment. A row whose Deployment call
  failed keeps its last sha and pull request (`keep_last_known`) but not its last health or
  pods, which would be stale under the `✗`. Only `matchLabels` is used, so pods of another
  workload sharing those labels are listed too. The details pane lists the target
  (`context / namespace / deployment`), the replica counts, every failing condition (not `True`, or `ReplicaFailure` that is `True`; yellow, red only
  while nothing serves),
  and one line per pod (status, restarts, age, sha8; red `✗` only for a stuck pod, yellow `●`
  for one that is merely starting, grey `⊘` for a terminating one, grey `○` last for a `Failed`
  or `Succeeded` one) with `last exit: OOMKilled (137) · <waiting
  message>` under a stuck one. `tests/fixtures/deployment.json` and `pods.json` are
  **synthetic**: the only clusters at hand are private.
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
    merge queue, `c` check out locally, `x` close. Main builds: `f` only.
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
- **Every sensitive write asks in one dialog.** `ui::dialog::draw_confirm` draws over the body
  while `Mode::Confirm`, centred over the whole body (the details pane included, so an open pane
  never squeezes it), at most 56 columns inside and 2 off each edge: `Clear`, a
  bordered `Confirm` block, the question (`Pending::{Merge, UpdateBranch, Status}`) wrapped over at most 3 lines (cut with `…` before the
  `?`), a blank line and the dim `y confirms · any other key cancels`, wrapped over 2 lines on a
  narrow body rather than cut. `draw_manage` draws the `S` popup with the same geometry: each
  offer wraps over at most 2 lines with a hanging indent, and nothing else: no hint line (the
  user called it pollution; the dim reasons already explain the popup). On a short terminal the popup's bottom is clipped. The footer carries no
  prompt, only the usual notice or refresh age underneath; it used to hold the question and was
  easy to miss. A new sensitive action adds a `Pending` variant and an arm in `dialog::question`. Widths are counted in characters,
  like the rest of the UI, so a title of wide characters (CJK, emoji) can be clipped at the
  dialog's right border.
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
  the deploy repo (`App::deploy_repo`: the `[[repo]]` with a deploy, else `builds_repo`).
- **Only http(s) URLs reach the browser.** Check URLs (`detailsUrl`/`targetUrl`) are set by
  whatever GitHub App or CI posted the check, not by GitHub, and `open`/`xdg-open` would launch
  `file://` paths, app bundles or any custom URL scheme handler. `open::web_url` is the one
  check; `App::open` applies it (footer notice, nothing spawned) and `SystemOpener::open`
  applies it again as the last line before the spawn.
- **Every `gh` and `kubectl` call is logged, nothing else from them.** `CliGh::run` and
  `CliKubectl::run` time the spawn and hand the raw `Output` to `Log::call`: argv (a
  `query=` argument becomes `query=<N chars>`), milliseconds, then `ok`, `exit N: <stderr lines
  joined by |>`, `timed out[: <stderr lines>]` (kubectl killed at its deadline) or `did not start: <err>`. Stdout is never written: it is megabytes of JSON and
  the job logs. A new runner of an external program takes a `Log` the same way. The file is
  `<program>.log` after `argv[0]`, so `conveyor` and `conveyor-dev` keep separate files; only
  the TUI opens it (and rotates the old one to `.1`), so `conveyor config` and `conveyor log`
  never touch it. A log that cannot be opened is a silent no-op. The file is created `0600`
  (it names repos and kube contexts). `log::path` returns `None`, and the log stays off, rather
  than build a path relative to the working directory from a base it derived: an empty
  `CONVEYOR_LOG` counts as unset, a relative `XDG_STATE_HOME` is ignored (the XDG spec says so) and
  a non-absolute `HOME` gives up. A relative `CONVEYOR_LOG` is an explicit choice and is used as
  given.
- **Every `gh` and `kubectl` is spawned under one lock.** `sources::spawning()` is held across
  the `spawn()` call and nothing else, in `CliGh::output` and `CliKubectl::output`. On macOS
  Rust creates a child's pipes with `pipe()` and marks them close-on-exec afterwards, so a
  thread that spawns at that instant hands its child the other child's pipe ends (Linux has
  `pipe2(O_CLOEXEC)` and is immune, so CI cannot see it). The pipe then stays open for as long
  as the child that inherited it runs. Measured after the environments went parallel in
  v0.23.2: a healthy environment read `kubectl did not answer` in about 9 of 100 fetches while
  another one hung, and a `gh` call started next to a hung one waited for it. The four fetcher
  threads all spawn at startup and on `R`, so `gh` had this race before the parallel read too.
  A new runner of an external program whose output is read must spawn under the same lock.
  `open`/`pbcopy` and the `kill` behind the deadline do not: they exit in milliseconds. Known
  and left: `pbcopy` is fed through a pipe, so a `gh` or `kubectl` spawned in the same
  microseconds as a `y`/`Y` press inherits its write end and the copy blocks the UI until that
  child exits.
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
- **Overlays show what lands under them.** The help and changelog panes replace the footer
  with a hint, so while `App::notice` is set (a merge, rerun or update result that landed while
  the pane was open) the footer shows the notice instead; the next key clears it as usual.
- **The help pane does not scroll.** `draw_help` renders `KEYS` through a `Paragraph`, so a
  list longer than the pane is silently clipped from the bottom: 14 keys plus two borders
  exactly fills a 17-row terminal. `("q", "quit")` is therefore first in `KEYS`, so the one
  key that gets you out is never the line that disappears. Adding a key means checking the
  smallest terminal you care about, or paginating the pane.
- **`c` shows the changelog, and every release writes one.** `CHANGELOG.md` is
  `include_str!`ed, so the pane always matches the binary and needs no network. In the pane
  `j/k`/arrows move a line, `Ctrl-d`/`Ctrl-u` half a page, any other key returns (like the
  help, so `q` closes it rather than quitting). `tests/changelog.rs` runs
  `changelog::check(CHANGELOG, CARGO_PKG_VERSION)`: the top entry is the crate version, every
  heading reads `## vX.Y.Z (YYYY-MM-DD)` with a real calendar date, versions strictly descend, every entry has a `- `
  bullet with text (an indented line continues the bullet above it) and no bullet repeats an
  older one. Every `v*` git tag up to the crate version must keep its entry (`check_released`; a newer tag
  from a sibling worktree's release is ignored; an older one, such as a hotfix released from
  another branch, is asked for, and the message says to merge `origin/main`), so relabelling or
  deleting an old entry fails too; that test needs the tags, hence `fetch-depth: 0` in CI's
  `check` job, and it fails loudly in a clone without them. A bump without its entry fails the gates.
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
- **Refreshes coalesce in the fetcher.** `r`/`R` always send a `Request::Refresh`; each
  fetcher thread wakes through `refresh::wait`, which takes one signal and drains the rest, so
  a burst costs at most one fetch beyond the one running, and a held `r` refetches back to back
  until released, with nothing left queued after. It used to queue a fetch per key repeat, which
  kept running long after the key came up. A time window in `App` was rejected: `app.now` only
  moves on a `Tick`, which never comes while keys arrive under 250 ms apart, and skipping a
  column mid-fetch dropped an `r` meant to catch a change the running fetch predates.
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
- **A PR card carries GitHub's pull-request icon** after its check glyph: `pr_glyph` gives
  U+F407 in green for an open pull request, U+F4DB (git-merge-queue) in yellow once it has a
  merge queue position, and U+F4DD in grey for a draft, so the meta line no longer says
  `draft`. They are Nerd Font octicons, the only Nerd Font glyphs in the UI: a terminal
  without a Nerd Font draws a box there. `card()` takes the icon as an
  `Option<(char, Color)>`; every other stage passes `None`.
- **No URLs in the details pane.** Every line there is text a person reads: the checks, the
  jobs, the sha, the pull request. A URL is 60 columns of noise nobody retypes, and both keys
  that need one (`b`, `p`) already hand it to the browser. `y`/`Y` copy them.
- **The keymap pairs the pull request and the build.** `p`/`b` open them, `y`/`Y` copy them,
  `d` is the details pane, `Ctrl-d`/`Ctrl-u` scroll it. `y/Y`, `r/R`, `?/c` and
  `C-d/C-u` each share one line of `KEYS`; `L` and `S` have one each, so the help needs 17 rows.
- **Colors** come from the ANSI palette so terminal themes apply. Do not hardcode hex.

## Testing traps hit so far

- tmux is a test harness, not a dependency: only the `#[ignore]` e2e tests use it, they panic
  with an install hint when it is missing, and `gates.sh` skips them without tmux (CI has it).
- `tests/kube.rs` shim tests take the same kind of mutex as `tests/github.rs` (fork race).
- **The spawn-race tests are red only some of the time, and only on macOS.**
  `a_call_made_while_another_hangs_…` in `tests/kube.rs` and `tests/github.rs` start 150 rounds
  of four calls at once. Without the spawn lock the kubectl one failed in about half its runs
  at 60 rounds and the `gh` one in every run; on Linux both always pass. One green run of
  either is no proof the lock is still in place.
- `tests/e2e.rs` runs the real binary in a scratch tmux server (`-L`, `-f /dev/null`) with
  `HOME` in a tempdir and a `gh` shell shim that prints `tests/fixtures/prs.json`. The pane
  command must be `env HOME=… PATH=… <binary>`: `respawn-pane -e PATH=…` looked right but
  the pane's login shell rebuilt `PATH` (macOS `path_helper` and rc files), the shim was
  never found and the real `gh` ran. Sockets are per test so parallel tests never share a
  server.
- **A scratch `HOME` does not isolate the config or the log.** `XDG_CONFIG_HOME`,
  `CONVEYOR_CONFIG`, `XDG_STATE_HOME` and `CONVEYOR_LOG` outrank `HOME`, and a shell that exports one made the e2e panes and `screenshots.sh` read the
  real config (real repo names on screen, three e2e failures that CI never saw). Both launch the
  binary with `env -u XDG_CONFIG_HOME -u CONVEYOR_CONFIG -u XDG_STATE_HOME -u CONVEYOR_LOG …`; any new launcher must do the same.
- **`gh` shim tests share one process, so they race on fork.** `tests/github.rs` writes a
  shell script per test and spawns it; on Linux CI one test's `fs::write` was still open when
  another test's `Command` forked, the child inherited the fd and `exec` failed with
  `ExecutableFileBusy` ("Text file busy"). It never showed on macOS and passed on the PR run,
  then failed on `main` after the rebase merge. Every test that creates a shim now takes the
  `SHIMS` mutex for its whole body. Any new test that writes an executable and runs it in the
  same test binary must do the same (or run it in a separate process such as tmux, as e2e
  does). Treat a green PR run as no proof against this class: it is timing-dependent.
  `tests/scripts.rs` needs no mutex because it writes no executable: every shim is a symlink
  to a tracked script, and the scripts under test are symlinked into the scratch directory.
- A card's first line in a 40-column pane holds about 30 characters after the bar, glyph and
  the right-aligned figure; a build card's title gets a line of its own and about `width - 4`.
  Card tests read line groups, not one line per row: a build card is four lines, so row `n`
  starts at screen line `1 + 4n` and a test that wants a third card needs a terminal taller
  than the 12-row default. The helper that finds the selected card looks for `│▌`.
- **Snapshots carry `vX.Y.Z`, not the crate version.** Every snapshot in `tests/snapshots.rs`
  goes through `versionless(h.screen())`, which swaps the footer logo for a placeholder and
  pads the line back to the terminal width, so a release bump touches no `.snap` file.
  `no_snapshot_carries_the_crate_version` fails for a new snapshot taken from a bare
  `h.screen()`. The real version on screen is asserted in `tests/prs.rs`.

## Documentation rule

`README.md` is minimal and holds only what a user needs, in this order and nothing else: the
banner `<img>` (it stands in for the h1; there is no `# ` heading), two sentences on what conveyor does, the one screenshot `docs/details.png` right below them (all
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

- **"Work autonomously" means the whole road to a release, without pausing**, not even for
  the retro: its outcome is written into `PLAN.md` in step 4 and the user reads it there. When the user
  says it (or "autonomous", "march towards release"), assume this flow and ask only when a
  decision is genuinely theirs:
  1. Plan in `plans/phase-N-<topic>.md` (next free N in `PLAN.md`), in a fresh worktree.
  2. `/my-tdd autonomous`: one task per red-green-refactor cycle, each shipped with
     `scripts/ship.sh` (gates, dev install, commit, push, draft PR on the first push).
  3. `/pr-review` the PR (see "Review before ready"). Fix every must and should finding, TDD
     like any change, ship, and review again. Minor findings are fixed when cheap, else listed
     in the PR body. The loop ends only on a round with no must/should findings, and that
     round is **unprimed**: the reviewers get the PR, the repo and this file, never the list
     of earlier findings or what was decided about them. Never post the review on the PR.
  4. The phase outcome is in `PLAN.md` and, for a behaviour change, the version bump and its
     `CHANGELOG.md` entry are on the branch. Re-read the PR title and body, fix what went
     stale, `gh pr ready`, wait for CI green.
  5. Rebase-merge (squash when GitHub cannot rebase the branch).
     For a behaviour change, tag and release per "Releasing", watch `release`
     to green and check the tap formula names the new version.
  6. Remove the worktree and report what shipped, with the release link.
  Stop and ask instead of proceeding when: a gate fails for a reason the change did not
  cause, CI fails twice on the same step, a review finding needs a product decision, or the
  next step would force-push, push to `main` or delete a release.
- **Cadence.** Outside the autonomous flow above, work runs under `/my-tdd autonomous`: one branch per phase (`phase-N-<topic>`),
  one commit per task through `scripts/ship.sh`, no pause between tasks, a pause only for the
  end-of-phase retro. Retro outcomes go to `PLAN.md` (facts, decisions) and here (rules).
- **One worktree per branch, by default.** Start every phase or fix in its own worktree off
  `origin/main`: `git worktree add -b <branch> .claude/worktrees/<branch> origin/main`, then
  `git branch --unset-upstream` so the first `ship.sh` push creates `origin/<branch>` instead of
  tracking `main`. Work, gates and ships run from inside it; the main checkout stays on its own
  branch, so two phases (and two agent sessions) never share an index or a `target/`.
  `.claude/worktrees/` is gitignored. Each worktree builds its own `target/` from cold the first
  time. `conveyor-dev` points at the worktree that last ran `dev-install.sh`, so re-run it in the
  worktree you are testing. Remove the worktree (`git worktree remove`) once its PR is merged.
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
- **Review before ready.** Marking a PR ready for review means running the
  [`/pr-review`](https://github.com/piacsek/dotfiles/blob/main/.claude/skills/pr-review/SKILL.md)
  skill on the PR first, and acting on what it finds. It runs a Claude and a Codex reviewer
  over the diff and merges their findings; run it from the main session, because the skill
  launches Codex there. Give both reviewers the intent of the change, point them at this
  file, have the Claude reviewer run `scripts/gates.sh` itself rather than trust the claim
  that the gates pass, and forbid edits, commits, and any write to GitHub or call to the
  clusters. When the skill asks whether to post the review on the PR, answer no.
  `gh pr ready` comes after the findings are addressed, not before.
- **Merging and releasing.** Rebase-merge the phase PR into `main` when CI is green (squash
  when GitHub cannot rebase it, see the next bullet), then tag
  from `main` (see Releasing). Never push to `main` directly except the very first bootstrap
  commit of an empty repository, and never force-push.
- **Catching up with `main` on a pushed branch is a merge, not a rebase.** A rebase rewrites
  the pushed commits and the next `scripts/ship.sh` push is refused; the no-force-push rule
  then leaves only `git reset --hard origin/<branch>`, `git merge origin/main` and a
  cherry-pick of the new work. The rebase-merge into `main` drops a clean merge commit anyway.
  A merge that resolved conflicts can stop GitHub from rebase-merging: it answers `This branch
  can't be rebased`, because the resolutions live only in the merge commit and replaying the
  branch's own commits hits the same conflicts again (phase 28, after v0.21.2 shipped mid-phase
  and its snapshots and `CHANGELOG.md` conflicted). When GitHub refuses, the branch lands as a
  **squash merge** instead, subject `<type>(<scope>): <summary> (#N)`; the per-task commits stay
  readable on the PR.
- **Two binaries.** `conveyor` on PATH is the Homebrew release; `conveyor-dev` is a working
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
cargo test -- --include-ignored   # one pass; the ignored ones are e2e: real binary, `gh` shim first on PATH
```

Without tmux the last gate is a plain `cargo test` and a notice that the e2e tests were skipped.
`tests/scripts.rs` runs the scripts themselves in a scratch directory and a scratch git
repository, with `cargo`, `gh` and the scripts they call replaced by symlinks to
`tests/fixtures/record-call.sh`, which appends its name and argv to `$CALLS_LOG`.

Ship with `scripts/ship.sh "<message>"`: `cargo fmt`, gates, `dev-install.sh` (unless skipped, see Install), commit, push, all under
`set -e`, refusing to run on `main`, opening the draft PR on the first push of a branch. Do not hand-roll the chain: the interactive shell here is
zsh, where `PIPESTATUS` is undefined and `test "" -eq 0` is true, so a `gates.sh | grep` guard
silently passed a clippy failure into a commit on 2026-09-10.

Run the wrapper, not the commands: piping `gates.sh | tail` once hid a `cargo fmt --check`
failure in tmux-agents and an unformatted commit slipped through. Chaining gates with `&&`
and then `;` before `git commit` does the same (it happened on this repo's bootstrap commit).

Install: `scripts/dev-install.sh` builds the release binary and symlinks it as
`~/.local/bin/conveyor-dev`; `conveyor` on PATH is always the Homebrew release. Never
`cargo install` the crate into a PATH dir. A milestone is done only after the gates pass
**and** `ship.sh` has either run `dev-install.sh` or said why it skipped it, so `conveyor-dev`
is the committed code of every branch that changed what the binary is built from.

`ship.sh` skips the install, printing `skipping dev-install: <reason>`, only when both hold
(`scripts/dev-install-skippable.sh`): `conveyor-dev` resolves to another worktree's build, and
nothing between the merge base with `origin/main` and the working tree, untracked files
included, is outside `tests/`, `docs/`, `plans/`, `scripts/`, `.github/`, `AGENTS.md`,
`PLAN.md`, `README.md`, `LICENSE` and `.gitignore`. Everything else counts as feeding the
binary (`src/`, `Cargo.toml`, `Cargo.lock`, the bundled `CHANGELOG.md`, any path nobody
listed), and a missing or dangling `conveyor-dev`, or no `origin/main`, installs too. So a
branch of docs, tests, scripts or CI never takes `conveyor-dev` over, and `conveyor-dev` then
stays the other worktree's build, not this branch's. Once `conveyor-dev` is this worktree's
build every ship runs `dev-install.sh`: cargo's own freshness check makes it a no-op of about
0.3 s when nothing changed (measured 2026-11-10; 17 s after a `src/` change, 23 s from a cold
`target/`), and it rebuilds after a merge of `main` that no diff of the commit would show.
Embedding a file from one of the listed directories into the binary means taking that
directory off the list in the script.

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
- **`main` is protected by a ruleset** (`main`, Settings → Rules): a pull request, green `check`
  and `audit`, linear history, no force-push or deletion, and no bypass for anyone. The checks
  are matched by job name, so renaming either job in `ci.yml` blocks every merge, the rename's
  own PR included; an advisory with no fixed version does the same through `audit`. Disable the
  ruleset to get out (`gh api -X PUT repos/<owner>/<repo>/rulesets/<id> -f enforcement=disabled`),
  fix, and enable it again.

## Releasing

Releases are GitHub Releases built by `.github/workflows/release.yml` on a `v*` tag: one flat
tarball per target (`conveyor-<target>.tar.gz` holding just the binary; targets
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`) plus `.sha256`,
with generated notes. Asset names carry no version so `releases/latest/download/` links stay
valid. Both macOS targets build in **one** `macos-latest` job (`cargo build --target … --target
…`, Intel cross-compiled); `macos-13` is retired and queues forever. They were two matrix jobs
until v0.19.2: hosted macOS runners regularly failed to come online (the v0.19.1 Intel job was
cancelled after 15 min without a step), and every macOS job is one more wait on that pool. A
stuck job is rerun with `gh run rerun <id> --failed`. The workflow refuses a tag whose version differs from
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

1. Bump `version` in `Cargo.toml` (`Cargo.lock` follows on the next build) and add its
   `## vX.Y.Z (YYYY-MM-DD)` entry at the top of `CHANGELOG.md`: what a user sees, one bullet each.
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
