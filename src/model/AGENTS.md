# src/model

How the answers from `gh` and `kubectl` are parsed and judged. Rules and working agreements are in the root `AGENTS.md`.

## Data sources (verified 2026-09-10, all through `gh`)

- **My PRs, the checks** (the query and its paging are in `src/sources/AGENTS.md`):
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
- **Jobs, the parsing** (the route and the jobs state are in `src/app/AGENTS.md`):
  **One line per job, newest attempt wins.** The jobs route's default `filter=latest` goes by
  `completed_at`, so during a re-run the previous attempt's finished job comes back next to the
  new in-progress one under the same name; `parse_jobs` keeps, per name, the jobs of that name's
  newest `run_attempt` (two jobs sharing a name in one attempt both stay).
  A job's failed step is the first step with a failing conclusion.
  `tests/fixtures/jobs.json` is a real capture of this repo's run 34509123916, trimmed to
  `id name status conclusion started_at completed_at html_url steps[]`.
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
- Deployed, the kubectl calls, their deadline and how the environments are read: see `src/sources/AGENTS.md`.
- Parse leniently: unknown fields ignored, unknown enum values → `Unknown`.
  `tests/fixtures/` carries verbatim `gh` output; refresh it when GitHub changes shape.
- The glyphs and cards that show these states: see `src/ui/AGENTS.md`.
