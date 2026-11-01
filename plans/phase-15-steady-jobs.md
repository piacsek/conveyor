# Phase 15: steady jobs (no duplicates, no blink)

## Context

Watching conveyor for a while shows two annoyances:

1. **Duplicate jobs after a re-run.** The same job shows twice, `✗` and a yellow `●`.
   `fetch_jobs` calls `runs/{id}/jobs` with the default `filter=latest`, which GitHub
   documents as filtering "by `completed_at`". While a re-run is in flight the new attempt's
   job has no `completed_at`, so the previous attempt's failed job with the same name comes
   back too. `parse_jobs` keeps every entry, and sorts failures first, so the stale `✗`
   even becomes the card's headline.
2. **Blink on refresh.** Not the lists themselves (`Column::receive` swaps rows in place).
   It is the job line: on every `Data(Builds|Queue)` `forget_unsettled_jobs`
   (`src/app.rs:293`) **removes** the cached jobs of every running run. The card falls
   back to the sha (`build_jobs`, `src/ui/rows.rs:225`), then `jobs: loading…`, then the
   job again. Running runs are exactly what the queue and main builds show, so it flickers
   every refresh. The details pane job list does the same.

Outcome: one line per job, always the newest attempt; refreshes replace job data in place,
`loading…` only on the very first fetch of a run.

## Tasks (branch `phase-15-steady-jobs`, `/tdd autonomous`, one ship per task)

1. **Plan file.** Copy this plan to `plans/phase-15-steady-jobs.md`; first ship opens the draft PR.
2. **Dedupe jobs by name, newest attempt wins.** In `src/model/jobs.rs`:
   - add `run_attempt: u32` to `Job` (parsed from `run_attempt`, default 1);
   - in `parse_jobs`, keep one job per `name`: max `(run_attempt, id)`, then the existing sort.
   - Red first: unit test in the existing jobs tests with two `"build"` entries
     (attempt 1 `failure`, attempt 2 `in_progress`) → one job, `Running`.
   - Add a TUI test in `tests/builds.rs`: the card shows the running job, not `✗`.
   - Name collisions are safe: matrix and reusable-workflow jobs carry distinct names
     within a run.
3. **Stale-while-refreshing jobs.** In `src/app.rs`:
   - `forget_unsettled_jobs` → mark instead of remove: a `Ready` entry of an unsettled run
     becomes due for refetch, its jobs stay on screen.
   - New `JobsState::Refreshing(Vec<Job>)`: in flight with last-known jobs. `jobs_needed`
     turns a due `Ready` into `Refreshing` and returns the id; never re-requests while
     `Loading` or `Refreshing` (the existing double-request guard).
   - Rendering (`rows.rs` `build_jobs`/`failed_job`, `details.rs` `job_lines`,
     `App::selected_failed_job`) treats `Refreshing(jobs)` like `Ready(jobs)`.
   - `receive_jobs` `Err` while refreshing keeps the old jobs (same spirit as
     `keep_last_known` for Deployed); `Err` on a first fetch stays `Failed`.
   - Red first: TUI test in `tests/builds.rs` — running build with jobs shown, then a
     `Data(Builds)` refresh → screen never contains `jobs: loading…` and still shows the job;
     then `Input::Jobs` lands the new state. Same for a queue entry in the queue tests.
4. **Guidelines.** Update `AGENTS.md` Data sources → Jobs bullet: dedupe rule and
   "refetch in place, never drop a running run's jobs" (replaces the "dropped on each
   Data" sentence). Patch release `v0.14.1` (fix, behaviour change).

## Files

- `src/model/jobs.rs` (`Job`, `parse_jobs`)
- `src/app.rs` (`JobsState`, `forget_unsettled_jobs`, `jobs_needed`, `receive_jobs`, `selected_failed_job`)
- `src/ui/rows.rs`, `src/ui/details.rs` (match arms)
- `tests/builds.rs`, queue TUI tests, `tests/support` helpers (`job`, `jobs`)
- `AGENTS.md`, `PLAN.md` (phase entry + outcome)

## Verification

- `scripts/gates.sh` (fmt, clippy, tests, e2e).
- Unproven premise: no re-run exists in this public repo to capture a real response
  (`gh api …/actions/runs` shows no `run_attempt > 1`). Task 2's fix holds either way;
  after `dev-install.sh`, confirm live in `conveyor-dev` by re-running a failed job and
  watching the card: one line, flipping `●` → result.
- Blink: run `conveyor-dev` with a running build selected for 2+ refreshes; the job line
  and details pane stay put.
