# Phase 59: a `gh` call that never returns no longer freezes its column

## Context

Issue #124. Every `kubectl` call runs under a deadline (`kube::DEADLINE`, 15 s): its own
process group, both pipes drained on threads, the group killed at the deadline. `gh` has
nothing of the kind: `CliGh::output` calls `child.wait_with_output()`, so a stalled connection,
a hung proxy or a credential helper keeps the call, and the thread that made it, waiting for
as long as `gh` does. A fetcher is one thread looping over one fetch, so its column stays on
`fetching…` and `r` does nothing; a worker is one thread per kind of write, so one hung
`gh pr merge` blocks every later merge of the session.

## Verified facts (2026-11-17, `main` at 6512665)

- `CliKubectl::output` (`src/sources/kube.rs`) holds the whole deadline machinery: spawn under
  `sources::spawning()` with `process_group(0)`, `drain` threads, a `try_wait` poll every
  10 ms, `kill_group` at the deadline, `DRAIN` (500 ms) for the pipes once the process exited,
  and `Ran::{Finished, TimedOut}`. Nothing in it is kubectl-specific.
- `CliGh::exec` runs every `gh` call and the checkout's `git remote -v`; `CliGh::output`
  spawns under the same lock but waits with `wait_with_output()`.
- `Failure::timed_out` and `FailureKind::TimedOut` exist; only kubectl produces them.
  `Log::timed_out(program, args, took, stderr)` logs `timed out[: <stderr>]`.
- The `gh` reads: `graphql`, `rest`, `current_repo`, `log_failed` (the jobs fetch is a `rest`).
  The writes: `merge`, `update_branch`, `change_status` (ready, draft, auto-merge on and off,
  close, enqueue, dequeue), `rerun_failed`. The checkout is `git remote -v` then
  `gh pr checkout`.
- A failed write's message reaches the footer as `#N: <message>` (merge, update, status) and
  the card as `! <action> failed: <message>`; a rerun's as `run <id>: <message>`.
- `tests/kube.rs` drives the kubectl deadline with shims that sleep (`with_deadline` is on
  `SEAMS`); `tests/github/` has the `SHIMS` mutex and a `shim` helper.

## States and what the screen shows

The deadlines: a read 30 s (`github::READ_DEADLINE`), a write 60 s (`WRITE_DEADLINE`), the
checkout none. "Timed out" means `gh` was still running at its deadline and its process group
was killed. Every outcome is logged as today; a timeout as `gh <argv> <ms>ms timed out[: <stderr>]`.

| Call | Outcome | Column / card | Footer |
| --- | --- | --- | --- |
| A read of a fetch (My PRs, Merge queue, Main builds, the sha lookup of Deployed) | answers in time | as today | as today |
| same | fails in time | as today: the column's error, its last rows kept | as today |
| same | times out | the column's error `gh did not answer within 30s` (kind `TimedOut`), its last rows kept, spinner gone; the next interval or `r` fetches again | the error when the column is focused, as for any failed fetch |
| Jobs / failed log | times out | the jobs pane shows `gh did not answer within 30s`, the log pane `gh did not answer within 60s`, as its error | as for any failed jobs or log fetch |
| Merge (`gh pr merge`) | times out | `! may be merged` | `#N: may be merged: no answer in 60s` |
| Update branch | times out | `! branch may be updated` | `#N: branch may be updated: no answer in 60s` |
| Mark ready / convert to draft | times out | `! may be marked ready` / `! may be a draft` | `#N: ` and the same, then `: no answer in 60s` |
| Enable / disable auto-merge | times out | `! auto-merge may be on` / `! auto-merge may be off` | `#N: ` and the same, then `: no answer in 60s` |
| Add to / leave the merge queue | times out | `! may be in the queue` / `! may be out of the queue` (25 characters, the longest) | `#N: ` and the same, then `: no answer in 60s` |
| Close | times out | `! may be closed` | `#N: may be closed: no answer in 60s` |
| Rerun failed jobs, every run | times out | `! may be rerunning` | `run <id>: may be rerunning: no answer in 60s`, joined by `; ` |
| Any write | answers or fails in time | as today | as today |
| Checkout (`git remote -v`, `gh pr checkout`) | any | as today: no deadline, a hang stays a hang (known limit) | as today |

A timed-out write is uncertain, not failed: GitHub may have done it and only the answer was
lost. Its mark, footer notice and `d` pane never say "failed". The card says only what may have
happened (`! may be merged`): no duration, no run id, at most 25 characters, so no supported
width cuts it. The `d` pane carries the rest in words that are true, the write, that `gh` did
not answer, and when conveyor gave up: `merge: may be merged; gh did not answer within 60s (no
answer by 11:20:02)`. The time is when the deadline fired, never worded as when it was sent.
It ends the busy spinner and marks the card like a failure, and it also gets what a success
gets for safety:

| Write | Times out: card | Popup | Guard lifts |
| --- | --- | --- | --- |
| Merge | `! may be merged` | `u` and `t` (to draft) refused with `waiting for My PRs to catch up with #N` | the first My PRs fetch that started after the timeout lands; My PRs and Merge queue reload once, now |
| Add to the merge queue | `! may be in the queue` | same as merge | same guard; My PRs and Merge queue reload now, My PRs again 10 s later |
| Enable auto-merge | `! auto-merge may be on` | same as merge | same as add to the merge queue |
| Ready, draft, disable auto-merge, leave the queue, close | the mark with its wording | as for any failed write | no guard; My PRs (and Merge queue unless ready) reload now, My PRs again 10 s later |
| Update branch | the mark with its wording | as for any failed write | no guard; My PRs reloads now and 10 s later |
| Rerun failed jobs | the mark with its wording | as for any failed rerun | no guard; its column reloads now |

A rerun of several runs words itself from the kind of each result, not from the first error:
runs that failed in time and runs that timed out are counted apart, and "failed" appears only
when a run did fail.

| Rerun of 2 runs | Card | `d` pane | Footer |
| --- | --- | --- | --- |
| one ok, one timed out | `! may be rerunning` | `rerun: may be rerunning; gh did not answer within 60s (no answer by HH:MM:SS)` | `rerun requested for 1 of 2 runs; run <id>: may be rerunning: no answer in 60s` |
| one failed, one timed out | `! rerun failed: 1 of 2 runs failed: run <id>: <error>; run <id>: may be rerunning: …` | `rerun failed at HH:MM:SS: ` and the same message | `rerun failed: run <id>: <error>; run <id>: may be rerunning: no answer in 60s` |
| both timed out | `! may be rerunning` | as one ok, one timed out | `run <id>: may be rerunning: no answer in 60s; run <id>: …` |
| one ok, one failed (as today) | `! rerun failed: 1 of 2 runs failed: run <id>: <error>` | `rerun failed at HH:MM:SS: …` | `rerun requested for 1 of 2 runs; run <id>: <error>` |

The mark stays until `Esc`, a new write on the row, or the row leaving, as for any failed
write; a refresh does not clear it. A write that fails in time (any other kind) neither lands
nor reloads, as today. The worker is free for the next write.

## Decisions

- One runner, `src/sources/runner.rs`, lifted from `CliKubectl::output`: it takes a `Command`
  and an optional deadline, spawns under `sources::spawning()` in its own process group and
  returns `Ran::Finished(Output)` or `Ran::TimedOut(stderr)`. `CliKubectl` and `CliGh` both
  call it; each keeps its own wording and logging.
- The write message does not repeat `#N`: the footer already starts with it and the card is
  the pull request's own, so `may be merged` reads right in both.
- The card, the footer and the `d` pane each say one thing (review round 3, after two rounds of
  findings in one long sentence): the card what may have happened, the footer that and the wait,
  the `d` pane the write, the wait and the time conveyor gave up. A source `Failure` of a timed-out
  write carries its wording and its seconds (`Unanswered`) so each place words itself.
- The failed log (`gh run view --log-failed`) downloads the whole run's log, so it gets the
  write deadline (60 s), not the read one.
- The checkout gets no deadline (a fetch over ssh can take long on a large repository, and a
  checkout changes nothing on GitHub that `r` would show). It still runs through the runner,
  but outside a process group: the group exists only so a deadline can kill it, and a group of
  its own would stop an ssh prompt on `/dev/tty` (passphrase, host key) where nobody can
  answer it.
- Tests set short deadlines through `CliGh::with_deadlines(read, write)`, a seam on `SEAMS`.

## Tasks

1. A read past its deadline fails as `TimedOut` with `gh did not answer within Ns`, logged
   `timed out`, and My PRs shows it as its column error instead of `fetching…`; the runner is
   shared with `CliKubectl`.
2. A write past its deadline says it may already have happened: merge, then update, status
   changes and rerun.
3. The checkout keeps no deadline.
4. Docs: `src/sources/AGENTS.md`, the Known limits of `AGENTS.md`, the fragment.

## Outcome

Shipped in #132 over four review rounds, the last one clean. Every `gh` call but the checkout
runs through `sources::runner`, which `CliKubectl` now shares: a read gives up after 30 s, the
failed log and every write after 60 s, each in a process group of its own that the deadline
kills. A read that times out shows `gh did not answer within 30s` as its column's error and the
next refresh tries again. A write that times out is uncertain, not failed
(`Failure::unanswered`): its card says only what may have happened (`! may be merged`), the
footer adds the wait, the `d` pane the write, the wait and when conveyor gave up, and it gets
what a success gets for safety (the merge wait, the reloads). A rerun of several runs counts the
runs that failed apart from the ones that timed out and names the failed ones first. The checkout
keeps no deadline and no process group, so an ssh prompt can still be answered. New known limit:
a hung `gh` or `kubectl` outlives a closed terminal. Not covered: a failed-log download that
needs more than 60 s (about 10 MB at 1 Mbit/s) fails and each retry starts over; no screenshot
shows a timed-out write, since the demo cannot produce one.

## Hurdles

- `review-loop` Round 1 found a timed-out write handled as a plain failure (no merge wait, no reload) and the checkout put in a process group of its own, where an ssh prompt could not be answered → code
- `review-loop` Round 2 found the card cutting off the uncertain half of a timed-out write's line at four columns → code
- `review-loop` Round 3 found the `d` pane's "sent at" time was when the deadline fired; the second wording finding in a row led to the redesign that split the sentence between card, footer and details → code
- `review-loop` Round 4 was clean; four minor findings were fixed after it → code
- `test-seam` Timeout messages count whole seconds, so tests that assert them needed 1 s and 2 s deadlines; the slow ones now assert the kind and the wording under 100 ms deadlines on parallel threads → code
- `tooling` `ship.sh` was piped through `tail` once, and `pgrep -f scripts/ship.sh` matched the ships of other worktrees while waiting for this one → none
- `process` Merging `main` after phase 61 conflicted in the same write functions (`receive_merged`, `receive_status`, the targets of the inputs), and the uncertain-write behaviour had to be reapplied on top → none
- `process` The timed-out card has no screenshot, declined because `--demo` never runs `gh` and so cannot time out → none
