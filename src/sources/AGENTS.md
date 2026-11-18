# src/sources

How GitHub (through `gh`) and the clusters (through `kubectl`) are reached and what they answer. Rules and working agreements are in the root `AGENTS.md`.

## Data sources (verified 2026-09-10, all through `gh`)

- **My PRs**: GraphQL `search(query: <prs.query>, type: ISSUE_ADVANCED)`. Plain `ISSUE` search
  ignores `OR` and parentheses (the default `(author:@me OR assignee:@me)` returned 0 there); the
  advanced type supports them, dedupes server-side and still accepts plain queries. Useful fields:
  `number title isDraft reviewDecision mergeStateStatus statusCheckRollup{state contexts}`
  `mergeQueueEntry{position state} updatedAt url headRefName repository{nameWithOwner}`.
  `viewerDidAuthor` and `viewerLatestReview{state commit{oid}}` decide the `v` approve offer;
  `id` and `headRefOid` pin the approval it sends.
  `contexts` is a union of `CheckRun` (`name status conclusion detailsUrl`) and
  `StatusContext` (`context state targetUrl`). `mergeStateStatus` is often `UNKNOWN`
  (GitHub computes it lazily): show it as `—`, never as an error.
  **Every page of checks is read.** `contexts(first: 100)` is GitHub's cap, and its order is not
  by time: it groups checks by name with the older run first, so a cut at 100 can drop the newer
  half of a pair and let the old result win. `fetch::read_remaining_checks` follows
  `pageInfo.hasNextPage` with `queries/pr_checks.graphql` (`node(id:)`, so the e2e shim's
  `repository(owner` route never catches it), up to `MAX_CHECK_PAGES` extra pages per pull
  request, before parsing. A failed page fails the column, which keeps its last rows.
- **My PRs fetches one tab at a time.** `fetch_prs(gh, config, tab)` runs the query and limit
  of `config.prs.tabs()[tab]` (a tab without a `limit` takes `[prs].limit`; with no
  `[[prs.tab]]`, `[prs].query` is tab 0). `refresh::spawn` takes the number of tabs to sweep:
  My PRs passes every tab when `[prs] background_tabs` is on (the default), else one. Each
  sweep fetches the tab last asked for, then the others in config order, so one fetch per tab
  per `refresh_secs` (a fetch can take more than one call, see the pages of checks above); with it off the number of `gh` calls does not grow with the tabs. The
  refresh channel carries a tab index and in a burst the last tab wins.
- My PRs, how the checks are deduplicated and what the card's check state reads: see `src/model/AGENTS.md`.
- **Merge queue**: one query per repo (`src/sources/queries/queue.graphql`) plus one REST call
  `repos/{r}/actions/runs?event=merge_group&per_page=30`; runs are matched to entries by the
  `pr-<N>-` segment of `head_branch` and the newest run wins. The REST call is optional: a
  failure leaves the entries with the rollup state from GraphQL. A repository without a queue
  (`mergeQueue: null`) parses to an empty `Queue` with `absent` set, not an error
  (`tests/fixtures/queue-none.json`). `tests/fixtures/queue.json` is
  **synthetic** (built from the schema; no public repo with a queue was at hand) and
  `queue-empty.json` is a real empty response.
- **Merge queue schema**: `repository.mergeQueue.entries` (`position state enqueuedAt
  estimatedTimeToMerge solo jump headCommit{oid} pullRequest{id headRefOid number title author}`).
  `headCommit` is the commit the merge group tests, not the pull request's head: the queue's
  writes pin `headRefOid` and name the pull request by its node `id`; either missing parses as
  empty and dims the queue's offers.
  Entry `state` ∈ AWAITING_CHECKS | LOCKED | MERGEABLE | QUEUED | UNMERGEABLE. The
  merge-group workflow runs (`gh run list --event merge_group`) live on branches
  `gh-readonly-queue/main/pr-<N>-<base sha>`; their `displayTitle` is the workflow name.
  Rulesets returned nothing for the probed repo (classic protection); the GraphQL object is the source.
- What a merge-group run means for its queue entry (`attach_runs`): see `src/model/AGENTS.md`.
- **Main builds**: `repos/{r}/actions/workflows/{id or file}/runs?branch={branch}&event=push&per_page=N`.
  `{branch}` is `branch` under the `[[repo]]` when set, else the repository's `default_branch`,
  read once from `repos/{r}` after the workflow resolves and cached on `BuildsSource` beside it.
  A failed lookup, or an answer without a `default_branch` string, is the column's error and is
  not cached, so the next fetch asks again; an empty column would read as "no builds". The
  branch is percent-encoded except `/`, so `+`, `&` and `#` reach GitHub as written. A blank
  `branch`, or one with spaces around it, is refused at load. The demo answers `repos/{o}/{n}` with `main`, and so does the e2e
  shim's `api repos/*` route. `Builds` carries the branch it read, so an empty column says
  `no builds on <branch>`: GitHub answers a branch with no push runs (a typo included) with
  zero runs, not an error.
  `main_workflow` may be a display name (resolved once through `actions/workflows?per_page=100`,
  matched on `name` or the file basename) or a `.yml` file used directly. The default `CI/CD`
  also tries `cicd.yml`, `CI`, `ci`, `ci.yml`, `build`, `build.yml`, `main.yml` in that order
  (`fetch::DEFAULT_WORKFLOWS`) so an unconfigured repo usually finds its build workflow. Each run's PR comes
  from one batched GraphQL query for all the runs, and from REST for whatever that did not
  answer (see the `Pulls` note below; rebase merges have no `(#N)` suffix, which is
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
  deploy repository's Main builds tab (0 = `at <branch>`, the branch that tab read), so it needs that column loaded and only sees the last
  `builds` runs. kubectl may itself start a teleport browser login when the session is
  expired; `DEADLINE` (below) returns the row to an error instead of hanging, and it kills that
  login with kubectl, so a login cannot be finished through conveyor: log in from a shell.
  **The request timeout is not a limit on the call, so the process has one.**
  `--request-timeout=10s` bounds one HTTP request, and kubectl retries API discovery: against a
  proxy that stopped answering, each call took 40 s (three failed attempts 10 s apart, then the
  error) and the column sat on `fetching…`. `CliKubectl::run` hands kubectl to `runner::run`
  (`runner.rs`, shared with `CliGh`), which spawns it in its own process
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
  of a fetch now takes as long as its slowest environment (the pull request lookup follows:
  one batched call for the shas not seen before, none when all are cached): about 15 s when a Deployment call hangs
  (`DEADLINE`, plus up to 500 ms to collect stderr; the pods call is skipped when the
  Deployment call fails), about 30 s when a Deployment call answers and its pods call hangs,
  however many environments are down. The rows still land together: a healthy row waits for
  the slowest one, because a column takes all its rows in one `Rows::Deployed`. A panic in a
  reader thread is passed on to the fetcher thread (`resume_unwind`), as it was when the read
  ran there, which catches it: the whole column shows `crashed: <msg>` for that fetch. With an
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
  own one, so the same sha is asked for once per column and each keeps its own retry
  window.
  **All the shas of a fetch are asked for in one GraphQL call first** (`Pulls::prefetch`, since
  v0.24.2): `repository { c0: object(oid: "<sha>") { ... on Commit { oid
  associatedPullRequests(first: 1) … } } c1: … }`, built in `commit_pulls_query` and read by
  `model::builds::parse_commit_pulls`, which matches answers by `oid`, not by alias. One `gh`
  call per commit cost about 0.5 s of process start and connection each: 30 builds took 24 s
  on the first load, the batch 2.3 s and one rate-limit point (100 commits, the `per_page`
  cap: about 2 s). Only shas not answered by the cache go in (unknown, or a miss older than
  `MISS_RETRY`), once each, and only 40-hex ones: a malformed oid fails the whole query. Per
  commit: a pull request is cached as a hit, with `[bot]` appended to a `Bot` author because
  GraphQL gives `dependabot` where REST gives `dependabot[bot]`; `nodes: []` goes straight to
  the `(#N)` fallback below without the REST association call, and the outcome is cached as
  for REST; a commit the batch did not answer for (`null` for one GitHub does not know, `{}`
  for an oid that is not a commit, a sha kept out of the query) is left to `for_sha`, which
  asks REST as before. **A batch `gh` refuses is not an answer**: nothing is cached and
  `for_sha` makes no REST call for those shas in that fetch (`Pulls::refused`), so a rate
  limit cannot turn one call into one per commit; the next fetch asks again. The price,
  accepted: while GraphQL is refused and REST is not (they have separate budgets), a commit
  not seen before shows no pull request until a batch gets through; cached hits stay.
  Known and left: a commit the batch answers `null` for, whose REST call answers 422, is
  cached by neither and costs one batch plus one REST call per refresh (a Deployed image
  built from a commit the repository does not have); and an upper-case sha misses the
  lower-case `oid` of its answer and is asked through REST. The query reads
  `repository(name: $name, owner: $owner)` on purpose: the e2e and screenshot shims route
  `repository(owner` to the queue fixture, and this way the batch falls to their default
  route, answers nothing, and every sha takes the REST path there.
  The REST order, for a sha the batch left alone, is: `repos/{r}/commits/{sha}/pulls`; then, only when that did not parse to
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

## The demo (`--demo`)

- **`Demo` is both `Github` and `Kube`, and reaches nothing.** It answers from JSON under
  `src/sources/demo/` (`include_str!`), which `scripts/screenshots.sh` also draws. A
  string `ago:<secs>` becomes the UTC time that many seconds before the call, so ages stay alive
  across refreshes. `graphql` matches `PRS_QUERY` (`review.json` when `q` holds
  `review-requested`, else `prs.json`) and `QUEUE_QUERY` (the queue only for
  `name = "webapp"`, `mergeQueue: null` for `name = "docs"`, an empty queue for any other
  repository) and answers any other query with
  a `repository` holding no commits, so `Pulls::prefetch` falls through to REST
  `commits/{sha}/pulls`, which `pulls.json` answers by sha. `rest` routes by path segments;
  jobs exist for any run id, failed for 1 and 311, running for 2, 4, 312, 313 and 314, and
  start and end with their run (the table in `jobs`).
- **The fixture tells one story.** My PRs are `me`'s and still open, so none of them is a
  Main build; the one queued (`#4827`) sits at the same position in `queue.json`, and each
  merge-group run's `pr-<N>-` branch names its entry. The failing check of `#4821` carries a
  finished failing suite with a run id, so `S` offers `f`. The mine tab is all `viewerDidAuthor: true`, so `v` is dim
  there; the review tab is someone else's, so `S` offers `v`. A run's jobs sit inside its times
  and its failed log names that run's own failure. `tests/demo.rs` holds these
  agreements; a fixture change that breaks one fails there.
- **`Kube` answers per environment name** from `deployments.json` and `pods.json`; any other
  environment (`uat`) fails with `ERROR: Active profile expired.`, so the demo shows an error row.
- **Every write and the checkout fail with `demo: nothing was changed`**, which marks the card
  like any failed write. `log_failed` answers `sessions.log` for run 311 and `failed.log` for any other run, in
  `gh`'s tab-separated shape.
- **`demo::config` keeps the user's config and replaces `[prs]` and `[[repo]]`** with
  `demo/config.toml`; `[ui]` and `[keys]` still apply. The demo runs with `Log::default()`: no
  log file is opened, rotated or written.

## Behaviour that is easy to break

- **Every `gh` and `kubectl` call is logged, nothing else from them.** `CliGh::exec` and
  `CliKubectl::run` time the spawn and hand the raw `Output` to `Log::call`: argv (a
  `query=` argument becomes `query=<N chars>`), milliseconds, then `ok`, `exit N: <stderr lines
  joined by |>`, `timed out[: <stderr lines>]` (`gh`, `git` or kubectl killed at its deadline) or `did not start: <err>`. Stdout is never written: it is megabytes of JSON and
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
  the `spawn()` call and nothing else, in `runner::run`, which both `CliGh` and `CliKubectl` call. On macOS
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
- **A failure records its kind.** The `Github` and `Kube` traits and everything in `fetch.rs`
  return `Result<_, Failure>` (`failure.rs`): the message, printed unchanged by `Display`, and
  a `FailureKind`. The rules, tried in this order:

  | Kind | Recognised by |
  | --- | --- |
  | `ProgramMissing` | spawning `gh`, `git` or `kubectl` fails with `NotFound` (`From<io::Error>`) |
  | `RateLimited` | `gh` failed and its stderr holds `rate limit`, in any case |
  | `NotSignedIn` | `gh` failed and its stderr holds `gh auth login` or `HTTP 401` |
  | `Offline` | `gh` failed and its stderr holds `error connecting to` or `dial tcp` |
  | `TimedOut` | a `gh` or kubectl deadline killed the process (`Failure::timed_out`) |
  | `Other` | anything else, and every `From<String>`: a parse error, a message conveyor wrote |

  Only a failed `gh` is classified by its text (`Failure::said_by_gh`, through `Tool::said`);
  `git` and `kubectl` exits read `Other`, kubectl's connection errors included (their wording
  is in `plans/phase-47-failure-kinds.md`). A helper that rewrites a message calls
  `Failure::reworded`, which keeps the kind. The status code is no key: a 403 is also a
  permission error and GraphQL answers a rate limit with 200, so the rule reads the words.
  Nothing draws a kind yet; every card, footer and log line reads the message as before.
- **`gh` never inherits the terminal.** `CliGh::exec` gives it a null stdin, so a `gh` that
  wants to prompt (no auth, `HOME` pointing elsewhere) fails fast instead of hanging the
  fetch thread behind `fetching…`.
- **Every `gh` call but the checkout has a deadline.** A stalled connection, a hung proxy or a
  credential helper used to hold `gh` (and the fetcher or worker thread waiting on it) for as
  long as it ran: the column stayed on `fetching…` and `r` did nothing, and one hung merge held
  every later merge of the session. `CliGh::exec` runs `gh` through `runner::run`, the same
  process group, pipe draining and group kill as kubectl (above). A read (`graphql`, `rest`,
  `current_repo`) gets `github::READ_DEADLINE` (30 s) and fails as `gh did not answer within
  30s`; the column keeps its last rows and the next interval or `r` tries again. `log_failed`
  (`gh run view --log-failed`) downloads the whole run's log, which on a slow link can take
  longer than a read, so it gets `WRITE_DEADLINE` (60 s) and fails as `gh did not answer within
  60s`. A write (`merge`, `update_branch`, `change_status`, `rerun_failed`) gets
  `WRITE_DEADLINE` too, and since GitHub may have done it anyway, it fails as
  `Failure::unanswered(aftermath, secs)`: its message is `may be merged: no answer in 60s` and
  it carries `Unanswered`, the wording per write (`Aftermath::wording`) and the seconds, so the
  card, the footer and the details pane each word it themselves. The app treats such a write as
  uncertain, not failed (see the failed writes bullet of `src/app/AGENTS.md`). The checkout (`git remote -v`, `gh pr checkout`) has
  none: a fetch over ssh can take long on a large repository, and nothing on GitHub changes
  that `r` would show. **Only a call with a deadline gets a process group of its own**: the
  group exists so the deadline can kill it, and a checkout in one would put `ssh` in a
  background group, stopped at a passphrase or host-key prompt on `/dev/tty` where nobody can
  answer it. Without a deadline `runner::run` waits for the pipes as `Command::output()` does
  (see the known limit in `AGENTS.md`).
  **A call in a process group of its own outlives a closed terminal.** The terminal's hangup
  goes to its foreground process group, which conveyor is in and a `gh` or kubectl with a
  deadline is not, so closing the terminal while one hangs leaves it running until it exits by
  itself; a call that answers, or reaches its deadline before conveyor exits, does not. Known
  limit (`AGENTS.md`).
  Tests shorten the deadlines with `CliGh::with_deadlines(read, write)`.
- What each `S` action makes `CliGh` run (`merge`, `update_branch`, `change_status`, `checkout` and its remotes guard, `rerun_failed`, `log_failed`): see `src/app/AGENTS.md`.
