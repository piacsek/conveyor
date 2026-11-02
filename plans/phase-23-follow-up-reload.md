# Phase 23: a second My PRs reload after `X` and `U`

## Context

`X` from My PRs and a `U` that updated the branch refresh My PRs as soon as the action lands.
That refresh often runs before GitHub has posted the new checks, so the card keeps the old
state until the next poll (`prs.refresh_secs`, 60 s). User decision (2026-11-02): add one more
My PRs refresh a few seconds after the first.

## Verified facts

- `Input::Tick` arrives every 250 ms while no key is pressed (`main.rs`), and `app.now` moves
  only on it, so a due time kept in `App` is checked on the tick.
- `refresh::wait` drains queued signals, so a follow-up that lands while a fetch runs costs at
  most one fetch.
- Measured 2026-11-02 on PR #35 (`gh run rerun` of a finished CI run, PR query polled every
  second): at +0 s the rerun's Actions checks are **gone** from `statusCheckRollup.contexts`
  (only the non-Actions check is left, so the card would read `✓`); at +2 s they are back as
  `QUEUED`, at +6 s `IN_PROGRESS`. The immediate reload can therefore land on a false green.
  `FOLLOW_UP` is 10 s: several times the measured gap, and room for a branch update, whose
  push has to start its workflows from scratch (not measured: no branch was behind).

## Tasks (one commit each via `scripts/ship.sh`, red test first)

1. (done above) Measure the lag.
2. **Follow-up reload.** A landed `X` from My PRs (any run went through) and a `U` that
   returned `Updated` set `App::follow_up` to `now + FOLLOW_UP`; the first tick at or after it
   sends `Request::Refresh(Stage::Prs)` once and clears it. A second action before it is due
   moves the due time. Tests in `tests/prs.rs` and `tests/update_branch.rs`: the refresh is
   sent once, not before the delay, and not for `UpToDate`, `Conflicts`, errors or a Main
   builds rerun.
3. Docs (AGENTS.md, PLAN.md), `/pr-review`, release as a patch.
