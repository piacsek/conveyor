# Phase 43: resize and the tick clock

## Problem

The terminal event loop in `src/main.rs` (`spawn_terminal_events`) has two defects:

1. `Event::Resize` is dropped (`Ok(_) => continue`), so the screen is redrawn at the new size
   only when the next `Tick` arrives.
2. `Input::Tick` is sent only when `event::poll(250 ms)` times out. While events stream (a held
   key, a window being dragged, key releases) no tick is sent, `app.now` freezes, ages stop and
   `follow_up_due()` never fires.

## Goal

- A resize reaches `run()` as `Input::Resize`, which does nothing but let `run()` draw again.
- A `Tick` is sent whenever 250 ms have passed since the last one, events or not: ticks come from
  a deadline, not from the poll timing out.
- Nothing else changes: no signal handling, no mouse, no config key.

## Events and what the loop sends

| Terminal event | Sent to `run()` |
| --- | --- |
| key press (a repeat included) | `Input::Key` |
| key release or repeat kind, focus, paste, mouse | nothing |
| resize | `Input::Resize` |
| deadline reached (250 ms since the last tick) | `Input::Tick(now)`, the next deadline 250 ms later |
| poll or read error | the error, then the loop ends |
| `run()` gone (send fails) | the loop ends |

## Seam

`src/events.rs`: an `EventSource` trait (`poll`, `read`) and a `Clock` trait (`instant` for the
deadline, `system` for the tick value), with the crossterm and system implementations, and
`forward(source, clock, tx)`, the loop. `main` spawns a thread that calls it. Tests drive it
with a scripted source and a scripted clock that share the elapsed time, so nothing sleeps.

## Readers of `app.now` once ticks come under a held key

- **Open debounce** (`OPEN_DEBOUNCE`, 1 s): measured from the last open, a held `p` would open a
  tab every second. It becomes a quiet window: a repeat inside the window moves it, so a held
  `p` opens once and the next press after 1 s of quiet opens again.
- **Refresh** (`r`/`R`): coalesced in the fetcher, does not read `app.now`; unchanged.
- **Follow-up**: now fires during a held key, which is the fix.
- **Spinner, ages, write marks, jobs aging**: they read the time as it is; they now move.

## Tasks

1. The loop moves into `src/events.rs` behind the seam, keys and idle ticks as today.
2. A resize is sent as `Input::Resize`.
3. Ticks come from a deadline while events stream.
4. A held `p` opens one tab while ticks keep coming.

## Outcome

#88, v0.30.1. No outcome was written for this phase when it closed; this one is rebuilt
from its fragment and pull request. The screen redraws at the new size as soon as the
terminal is resized (`Input::Resize`), and ticks come every 250 ms from a deadline, so ages,
the refresh spinner and the follow-up reload of My PRs keep moving while a key is held. The
terminal events loop moved into `src/events.rs` behind an event source and a clock; a held
`p` or `b` opens one tab while the clock keeps ticking.

## Hurdles

- `process` The phase closed without its outcome in `PLAN.md`; found only when the outcomes moved into the plan files → rule in `AGENTS.md`
