# Phase 18: elapsed time on every check and job line

## Context

The My PRs details pane (`d`) lists one line per check (`✓ lint`) with no time.
The user wants each line to say how long that check took, on the same line as its name,
ticking while it runs, at second precision (`3m59s`, not `3m`). The Main builds / Merge
queue job lists already show a time but only once finished and only in whole minutes;
they get the same treatment so the two lists read alike.

Target:

```
✓ lint  42s
● api / test  3m12s
✗ e2e  1m05s
- deploy preview
```

## Verified facts

- GraphQL `CheckRun` has `startedAt` and `completedAt` (nullable). `StatusContext` has no
  timing (only `createdAt`), so status-context lines stay without a time.
- REST jobs already parsed with `started_at`/`completed_at` (`src/model/jobs.rs`, `Job::duration`).
- `text::duration` (`src/text.rs:66`) is coarse and also backs `age()`, card ages and queue eta.

## Tasks (one commit each via `scripts/ship.sh`, red test first)

1. **Precise format.** New `text::elapsed(Duration) -> String`: `45s`, `3m59s`, `1h04m`, `2d03h`.
   `duration`/`age`/eta stay coarse (card ages like `12m34s` would be noise and cost width).
   Unit tests in the existing text tests.
2. **Parse check timing.** Add `startedAt completedAt` to the `CheckRun` fragment in
   `src/sources/queries/prs.graphql` and `pr_checks.graphql`. `Check` gains
   `started_at`/`completed_at: Option<SystemTime>` parsed with `prs::parse_timestamp`, plus
   `Check::took(now) -> Option<Duration>`: completed − started when finished, now − started
   while pending, `None` without `startedAt`. Add the fields to `tests/fixtures/prs.json`
   (refresh from a real capture of this repo's PRs).
3. **Render on the check line.** `pr_details` (`src/ui/details.rs:113`) takes `now` and
   appends `  {elapsed}` after the name, mirroring `job_line`. TUI test in the existing
   details suite: a finished check shows `42s`, a pending one shows time since start.
4. **Jobs get the same.** `Job::took(now)` (live while running, like `Build::elapsed`),
   `job_line` uses `text::elapsed`; `run_line` and the build card's took use `elapsed` too.
   Update the existing `parse_jobs`/details assertions that expect `3m`.
5. **Docs + release.** AGENTS.md note on check timing; `scripts/screenshots.sh` (synthetic
   fixture gets timestamps); README unchanged (no key/config change). Bump to `0.16.0`.

## Files

- `src/text.rs`, `src/model/prs.rs`, `src/model/jobs.rs`, `src/ui/details.rs`, `src/ui/rows.rs`
- `src/sources/queries/prs.graphql`, `src/sources/queries/pr_checks.graphql`
- `tests/fixtures/prs.json`, screenshot fixture, `AGENTS.md`, `plans/phase-18-check-times.md`

## Verification

- `scripts/gates.sh` (fmt, clippy, tests, e2e).
- `scripts/dev-install.sh`, run `conveyor-dev`, open `d` on a PR with running CI: times
  appear per check and tick on the 250 ms redraw; a finished check's time matches GitHub's
  checks tab to the second.

## Caveats

- A check that was queued but not started has no `startedAt`: no time shown, not `0s`.
- External status contexts (non-Actions bots posting statuses) never get a time.
- The PR column refreshes on its interval, so a check that finished between refreshes keeps
  ticking until the next fetch (same as running build cards today).
