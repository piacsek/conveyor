# Phase 48: the help view says Esc dismisses the error on a card

## Context
A user saw a red `! check out failed: ...` line on a My PRs card and did not know `Esc`
dismisses it. The help view (`?`) lists `Esc` as `close details / zoom / mark / filter`; "mark"
is a word from the code, not from the screen.

## The wording
`KEYS` in `src/ui/help.rs` and the README key table read:

    Esc  close details / zoom, dismiss a failed action's error, clear the filter

The order is what `Esc` does per press (`src/app/AGENTS.md`, the `Esc` bullet): details pane,
zoom, the selected card's failure mark, the live filter. No behaviour changes.

## Tasks
1. A test opens the help view through `run()` and asserts the new line is on screen (red), then
   `KEYS`, the README row and the help snapshot change (green). Fragment `kind: fix`.
2. `scripts/screenshots.sh` shoots the help view as `docs/help.png`, for the pull request; it is
   not added to the README, which holds one screenshot.

## Verified facts (2026-11-16)
- The old wording lives in `src/ui/help.rs` (`KEYS`), `README.md` (the key table),
  `tests/prs/help.rs` and `tests/snapshots/snapshots__help_layout.snap`. `PLAN.md` (phase 39
  outcome) and `plans/phase-13-filter-keys-log.md` quote earlier wordings as history and stay.
- No `AGENTS.md` quotes the help line; `src/app/AGENTS.md` describes the `Esc` order in its own
  words, which still hold.
- `tests/readme.rs` pins the README key table to `KEYS` word for word.
- The help pane does not scroll and fills an 18-row terminal at 160 columns with 15 keys; the
  entry count does not change.
