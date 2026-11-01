# Phase 17: a cancelled run's leftover job does not fail a PR card

## Context
A PR card read `✗ <workflow> / <status job>` while every other job of that workflow was
pending. A concurrency group had cancelled the previous run, whose `if: always()` status job
failed; the new run had not created that job yet. `newest_checks` dedupes per
(workflow, event, name), so a job only the old run had survived. Verified live: the old run's
check suite has `conclusion: CANCELLED`, the new one is `IN_PROGRESS`.

## Change
1. Queries (`prs.graphql`, `pr_checks.graphql`): add `checkSuite.conclusion` and
   `workflowRun.databaseId`.
2. `src/model/prs.rs`: before the per-name dedup, drop a check whose suite is `CANCELLED` when a
   newer run (higher `workflowRun.databaseId`) of the same workflow and event exists.
3. Not dropped: checks of an older run that finished on its own (a newer run may skip jobs),
   and a cancelled run with nothing newer.

## Tests (`tests/parse_prs.rs`, red first)
- a job only a cancelled older run has is dropped once a newer run exists (red before the fix)
- a job only a finished older run has stays
- a cancelled run with nothing newer keeps its jobs

## Verification
Gates; both queries run against live GitHub without errors; `conveyor-dev` on the reporting PR.
