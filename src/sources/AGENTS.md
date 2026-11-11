# src/sources

How GitHub (through `gh`) and the clusters (through `kubectl`) are reached and what they answer. Rules and working agreements are in the root `AGENTS.md`.

## Data sources (verified 2026-09-10, all through `gh`)

- **My PRs**: GraphQL `search(query: <prs.query>, type: ISSUE_ADVANCED)`. Plain `ISSUE` search
  ignores `OR` and parentheses (the default `(author:@me OR assignee:@me)` returned 0 there); the
  advanced type supports them, dedupes server-side and still accepts plain queries. Useful fields:
  `number title isDraft reviewDecision mergeStateStatus statusCheckRollup{state contexts}`
  `mergeQueueEntry{position state} updatedAt url headRefName repository{nameWithOwner}`.
  `contexts` is a union of `CheckRun` (`name status conclusion detailsUrl`) and
  `StatusContext` (`context state targetUrl`). `mergeStateStatus` is often `UNKNOWN`
  (GitHub computes it lazily): show it as `—`, never as an error.
  **Every page of checks is read.** `contexts(first: 100)` is GitHub's cap, and its order is not
  by time: it groups checks by name with the older run first, so a cut at 100 can drop the newer
  half of a pair and let the old result win. `fetch::read_remaining_checks` follows
  `pageInfo.hasNextPage` with `queries/pr_checks.graphql` (`node(id:)`, so the e2e shim's
  `repository(owner` route never catches it), up to `MAX_CHECK_PAGES` extra pages per pull
  request, before parsing. A failed page fails the column, which keeps its last rows.
- My PRs, how the checks are deduplicated and what the card's check state reads: see `src/model/AGENTS.md`.
- **Merge queue**: one query per repo (`src/sources/queries/queue.graphql`) plus one REST call
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
- What a merge-group run means for its queue entry (`attach_runs`): see `src/model/AGENTS.md`.
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
  Many workflows run on main (`issue_comment` bots, per-app deploys), so the build workflow is
  per-repo config (default `CI/CD`).
- Jobs (`repos/{r}/actions/runs/{id}/jobs?per_page=100`, `fetch::fetch_jobs`): see `src/app/AGENTS.md`; the parsing is in `src/model/AGENTS.md`.
- **Deployed**: `kubectl --context C -n NS get deploy D -o json --request-timeout=10s`
  per `[[repo.deploy.env]]` (the image is `.spec.template.spec.containers[0].image`); verified live 2026-09-10: the tag is the bare 40-hex commit sha
  (a GitOps image updater writes it; preview deployments use `<label>-<sha>`, which
  `sha_from_image` also accepts). Errors are per row, never per column: an expired session
  or a missing deployment (`Error from server (NotFound): deployments.apps "x" not found`)
  marks that env `✗`, keeps its last known sha/PR (`keep_last_known`) and shows the
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
- Deploy health, the second kubectl call per environment and how its answer is judged: see `src/model/AGENTS.md`.
- **Deployed, how the environments are read:**
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
- Lenient parsing: see `src/model/AGENTS.md`.

## Behaviour that is easy to break

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
- What each `S` action makes `CliGh` run (`merge`, `update_branch`, `change_status`, `checkout` and its remotes guard, `rerun_failed`, `log_failed`): see `src/app/AGENTS.md`.
