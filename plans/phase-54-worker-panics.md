# Phase 54: a crashed background worker shows up where its result would have

## Context
Issue #93. conveyor fetches and writes on background threads: one stage fetcher per column
(`refresh::spawn`), one generic worker per kind of request (`worker::spawn`, since #101), and
the terminal events thread (`events::forward`). None of them catches a panic, so one panic
ends that thread for the rest of the session and nothing reaches the screen.

## Verified facts (2026-11-16)
- Reproduced on a debug build of this worktree, `conveyor --demo` in a scratch tmux server
  (`HOME` in a tempdir), with a temporary `panic!("repro panic")` in the stage fetcher loop for
  Merge queue (not committed). The raw pane output right after the first frame is
  `ESC[?1049l` (leave the alternate screen), then `thread '<unnamed>' panicked at
  src/refresh.rs:121:40: repro panic` and the backtrace note, then the main loop goes on
  drawing frames over the main screen. `stty -a` on the pane reads `icanon isig echo`: raw
  mode is off. A pressed `r` is echoed into the drawing and never reaches conveyor, and `q`
  does not quit either: in canonical mode the keys wait for `Enter`. Merge queue stays on
  `fetching…`.
- So the hook `ratatui::init()` installs (restore the terminal, then the previous hook, which
  prints) runs for a panic on any thread. A worker panic breaks the whole terminal, not only
  its own column: the hook must leave the terminal alone unless the panic is on the main
  thread.
- The panic hook runs at the panic, before unwinding, whether or not something catches it.
  `std::panic::resume_unwind` does not run the hook again.
- `Cargo.toml` sets no `panic = "abort"` in any profile, so unwinding can be caught.
- `DeploySource::fetch` joins its per-environment scoped threads and re-raises a reader's
  panic on the fetcher thread with `resume_unwind` (`src/sources/fetch.rs`).
- `Failure::kind` is never drawn; the card mark, the column error, the jobs and log panes show
  `Failure`'s message.
- If the events thread stopped while the hook left the terminal raw, no key would reach
  conveyor and `Ctrl-C` would not interrupt it (raw mode clears `isig`): the terminal would be
  stuck. So that thread must end the app instead.

## States and what the screen shows
`<msg>` is the panic's message (`a panic with no message` when it carries none).

| Thread that panics | Mode or pane at the time | Write in flight on the row | Screen after |
| --- | --- | --- | --- |
| A column's fetcher (any column, any tab, background tab included) | any | any | That column (that tab) shows `crashed: <msg>` as its error, rows kept as for any failed fetch, spinner gone. The next interval or `r` fetches again. |
| A Deployed reader thread for one environment | any | n/a | Re-raised on the Deployed fetcher: the whole column shows `crashed: <msg>`, as above (one environment's crash costs the column one fetch, not only its row). |
| Jobs | details pane open or closed | n/a | The run's jobs are `Failed(crashed: <msg>)`: the pane shows it as its error, the card shows what it shows for any failed jobs fetch. A refresh of jobs already on screen for a running run keeps them (the existing rule for a failed refresh). Later runs get their jobs. |
| Log | log pane open or closed | n/a | That job's log is `Failed(crashed: <msg>)`, shown in the log pane. Later logs load. |
| Merge, update, status change, checkout | any (`S` popup, confirm, help, changelog, filter) | yes, row still listed | Busy spinner ends; the card gets `! <action> failed: crashed: <msg>`; footer notice as for any failure. Later writes of that kind run. |
| Merge, update, status change, checkout | any | yes, row gone from the column | Spinner state dropped; no mark (as for any failure on a row that left); footer notice. |
| Rerun | any | yes | Every run of the request failed with `crashed: <msg>`: the card gets `! rerun failed: run <id>: crashed: <msg>; …`. Later reruns run. |
| Terminal events | any | any | conveyor ends, the terminal is restored, and it prints `conveyor: terminal events stopped: crashed: <msg>` and exits 1. |
| Main thread (drawing, key handling) | any | any | As today: the terminal is restored and the panic printed. |

Every caught panic writes one line to the call log: `panic on <thread>: <msg> at <file:line>`.
Workers are named `<kind> worker` (`merge worker`), fetchers `<Stage> fetcher`, the events
thread `terminal events`; a Deployed reader thread is unnamed (`<unnamed>`, the location
says `fetch.rs`). Nothing is printed on the terminal for a panic off the main thread.

## Design
1. `src/crash.rs`: `catch(work) -> Result<R, Failure>` (`catch_unwind` with
   `AssertUnwindSafe`, the payload turned into `Failure::crashed`, kind `Crashed`), and
   `hook(log, main_hook)`: the panic hook installed right after `ratatui::init()`, which logs
   every panic with its thread name and location and calls ratatui's hook (restore and print)
   only on the thread that installed it.
2. `worker::spawn` names its thread and catches around one request. What a crash answers is
   the request's: a `worker::Task` trait on each request type (`MergeTarget`, the jobs tuple,
   …) gives the worker's name and `crashed(self, failure) -> Input`.
3. `refresh::spawn` names its thread and catches around one `fetch(tab)`, sending
   `Data(stage, tab, Err(crashed))` and keeping its schedule.
4. `events::forward` catches around its loop and sends `Err(io::Error)` with
   `terminal events stopped: crashed: <msg>`, which ends `run()`.

## Tasks
1. Workers: one test per kind (jobs, log, rerun, merge, update, status, checkout) with work
   that panics on the first request: the `Input` that comes back, and that the second request
   is answered; one screen test that the crashed merge marks its card.
2. Fetcher: a panicking fetch comes back as the column's error and the next refresh fetches;
   a Deployed reader that panics comes back as the Deployed column's error; no profile sets
   `panic = "abort"`.
3. Hook: a panic off the installing thread is logged with its thread name and leaves the main
   hook alone; one on it calls the main hook. The events thread's panic ends the app.
4. Docs: `AGENTS.md` files, `PLAN.md`, the fragment.

## Outcome

Phase 54 (`plans/phase-54-worker-panics.md`, closes #93): a panic on a background thread
no longer breaks the terminal or leaves anything waiting. Reproduced first: a panic in the
Merge queue fetcher ran ratatui's hook, which left the alternate screen and raw mode from
that thread, printed over the main screen, and left the keys in canonical mode (`q` did not
quit) and the column on `fetching…`. `crash::catch` wraps one unit of work: a worker request
answers with `crashed: <msg>` where its result would have been (`worker::Task` per request
type), a column fetch comes back as that tab's error, a Deployed reader's panic (re-raised on
the fetcher) as the Deployed column's error, and the events thread ends the app with
`terminal events stopped: crashed: <msg>`. `crash::hook`, installed before any thread
starts, logs every panic with its thread name and location and calls ratatui's hook on the
main thread only. New `FailureKind::Crashed`. Decisions: the thread keeps running after a
crash (no restart, no cap); the Deployed column fails as a whole, as for any reader panic;
no `panic = "abort"` in any profile, pinned by a test. Fragment `kind: fix`.

## Hurdles

- `process` Catching up with `main` after phases 53 and 55 merged took part of the 41 minutes → none
