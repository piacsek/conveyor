# Phase 19: `X` reruns failed jobs (Main builds and My PRs)

## Context

`L` shows why a run failed, but acting on it (a flaky test, a runner hiccup) still costs a
browser trip to click "Re-run failed jobs". `X` does it from the TUI with a y/n confirm.
It is conveyor's first write action, so the confirm UX is designed here once and reused by
later write actions (backlog: merge, approve, enqueue).

Scope (user's answer, 2026-11-02): **Main builds** (the selected run) and **My PRs** (every
Actions run behind the PR's failing checks). Merge queue: footer notice only, because a failed
merge-group run has already dropped its PR from the queue and rerunning it merges nothing.

Target footer while confirming:

```
rerun failed jobs of run 431? y/n
rerun failed jobs of #812 (2 runs: CI, e2e)? y/n
```

## Verified facts

- `gh run rerun <run-id> --failed -R <owner/repo>` reruns failed and cancelled jobs as a new
  attempt with the **same run id**. It refuses a run that is still in progress.
  **Verified with `gh` 2.103.3 (`gh run rerun --help`): `--failed` and `-R` as above.**
- PR checks already carry the run id: `prs.graphql` and `pr_checks.graphql` select
  `checkSuite { workflowRun { databaseId } }`, and `src/model/prs.rs:232` reads it for the
  superseded-run logic, but `Check` (`src/model/prs.rs:25`) does not keep it.
- A `StatusContext` or a check run from a non-Actions app has no workflow run: not rerunnable.
- After a rerun the run goes back to queued. `App::age_jobs` already turns `Ready` into
  `Stale` for an unsettled run, and `parse_jobs` keeps the newest `run_attempt` per job name,
  so the jobs line refreshes without new code (needs a test, not new logic).
- `KEYS` is exactly 13 lines (help fits a 16-row terminal); `X` shares the `L` line.

## Tasks (one commit each via `scripts/ship.sh`, red test first)

1. **`Github::rerun_failed(repo, run_id)`.** New trait method in `src/sources/github.rs`,
   `CliGh::rerun_failed_args` mirroring `log_failed_args`; `fetch::rerun_failed` maps the
   error to `String`. Tests in `tests/github.rs` with the `SHIMS` mutex: argv pinned, a
   non-zero exit surfaces gh's stderr.
2. **`Check::run_id`.** Keep `workflowRun.databaseId` on `Check` (`Option<u64>`).
   `PullRequest::rerunnable_runs() -> Vec<u64>`: distinct run ids of failing checks whose run
   has no pending check left in the deduplicated list (`newest_checks` output). Unit tests:
   a failure in a finished run, a failure in a run with a pending sibling (excluded), a
   failing `StatusContext` (excluded), two failures in one run (one id).
3. **Confirm mode.** `Mode::Confirm(Rerun { repo, run_ids, label })` in `src/app.rs`.
   `X` in Normal mode:
   - Main builds: selected run, only when settled and `Failure`/`Cancelled`; else notice
     `nothing failed to rerun` / `run 431 is still running`.
   - My PRs: `rerunnable_runs()` of the selected PR, repo from the PR's
     `repository.nameWithOwner`; none → notice `no finished failing run to rerun`.
   - Merge queue / Deployed: notice (queue: `re-queue the pull request instead`).
   `y` returns `Action::Rerun { repo, run_ids }`; any other key (Esc included) cancels with
   notice `rerun cancelled`. Arm goes next to `Mode::Help`, before Ctrl-d/u. `footer_text`
   shows the prompt while in the mode. TUI tests drive `run()` with scripted keys.
4. **Wire the request.** `Request::Rerun { repo, run_ids }` (drop `Copy` from `Request`),
   `Input::Rerun(Result<String, String>)`; a `spawn_rerun_worker` thread in `src/main.rs`
   shaped like `spawn_log_fetcher`, one `gh` call per run id. On `Ok`: notice
   `rerun requested: run 431` and a `Request::Refresh` for the focused stage so the card turns
   queued. `App::rerunning: HashSet<u64>` blocks a second `X` on the same run until the
   result lands (key repeat on `y`). Test: after the refresh shows the run queued, the
   previous jobs stay drawn (`Stale` → `Refreshing`) and the new attempt replaces them.
5. **Help, docs, release.** `KEYS`: `("L/X", "log of the failed step / rerun failed jobs")`,
   `keys_for` per column; README key table (pinned by `tests/readme.rs`). AGENTS.md: replace
   "Rerunning a job stays a backlog item" with an `X` bullet (scope, confirm, same run id);
   PLAN.md backlog loses the rerun item and gains the phase outcome. Bump to `0.17.0`
   (new key = minor).

## Files

- `src/sources/github.rs`, `src/sources/fetch.rs`, `src/main.rs`
- `src/model/prs.rs`, `src/app.rs`, `src/ui/footer.rs`, `src/ui/help.rs`
- `tests/github.rs`, TUI test suites, `tests/readme.rs` (via README), `README.md`,
  `AGENTS.md`, `PLAN.md`, `plans/phase-19-rerun.md`, `Cargo.toml`

## Verification

- `scripts/gates.sh` (fmt, clippy, tests, e2e).
- e2e: extend the `gh` shim to log `run rerun` argv to a file; drive `X` then `y` in tmux and
  assert the file holds `run rerun <id> --failed -R acme/…`. Drive `X` then `n`: file empty.
- Live, after `scripts/dev-install.sh`: in `conveyor-dev`, `X` on a failed main build of this
  repo (or a throwaway PR with a forced failure), `y`, watch the card go queued and the
  job line refresh; GitHub shows attempt 2 on the same run.

## Caveats

- Write action with real side effects: the e2e shim must never let the real `gh` run
  (`env -u XDG_CONFIG_HOME -u CONVEYOR_CONFIG`, shim first on PATH, as today).
- A PR whose failing run was superseded by a newer cancelled-then-rerun run may expose an
  older run id; `rerunnable_runs` reads the deduplicated checks, so only surviving checks count.
- Permissions: rerun needs `actions:write`; a token without it fails with gh's message in the
  footer. Nothing is retried automatically.
