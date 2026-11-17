# Phase 22: reruns from GitHub, and a reload after every PR write

## Context

Issue #32: a main build or merge-group run rerun outside conveyor that finishes between two
polls keeps the previous attempt's jobs forever (`✓` card with the old `✗ <job> · <step>`
line). Jobs are cached per run id and a rerun keeps its run id; `age_jobs` only asks again
when the run, or one of the cached jobs, is unsettled.

User request (2026-11-02): reload a pull request after a write action (`U`, `X`). Checked
against the code: both already refresh My PRs as soon as the action lands (`X` on any run that
went through, `U` on `Updated`), so the user decided to keep that behaviour and add no fetches.

## Verified facts

- `repos/{r}/actions/runs` and `runs/{id}` carry `run_attempt` (1 on a run never rerun).
- `runs/{id}/jobs` (default `filter=latest`) returns every job stamped with the newest
  attempt, including the jobs a `--failed` rerun did not run again (checked on this repo's run
  41336356537: `check` failed and `audit` passed, both `run_attempt: 2`).

## Tasks (one commit each via `scripts/ship.sh`, red test first)

1. **`Build::attempt`** from `run_attempt` (default 1) and **`Job::attempt`** likewise. Tests
   in `tests/parse_builds.rs` and `tests/parse_jobs.rs`.
2. **Older attempts go stale.** `age_jobs` turns a `Ready` whose newest job attempt is below
   the run's attempt into `Stale`, settled or not, so `stale_run` asks again and the old line
   stays until the answer lands. TUI test in `tests/builds.rs`: a failed run with its jobs
   loaded comes back passing with `attempt: 2` in one refresh; the card ends on the new jobs.
3. Docs (AGENTS.md, PLAN.md), `/pr-review`, release as a patch.

## Outcome

v0.19.1 (phase 22, `plans/phase-22-rerun-attempts.md`): a main build or merge-group run
rerun from GitHub that passes between two polls no longer keeps its old failed job line
(#32); cached jobs from an older `run_attempt` are fetched again. Asked on the way: reload a
pull request after `U`/`X`. Both already refresh My PRs as soon as the action lands, so the
behaviour was kept and no fetch added.
