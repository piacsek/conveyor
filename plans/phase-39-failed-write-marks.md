# Phase 39: a failed action stays on its card

## Context
Since phase 38 several pull requests can have writes running at once, but every result still goes to the one footer line. #14's success can land a moment after #12's failure and replace it, and the busy spinner on #12 ends the same way whether the write worked or not. A result that lands while the `S` popup or a confirm dialog is open is lost the same way: the key that answers the prompt clears the footer. The design and the task list are issue #70; this file is that plan checked against `main` after phase 38.

## Design
A failed write leaves a mark on the card it was started from. Successes leave nothing new.

```text
My PRs (3) !1
▌ ●  #12 Retry webhooks             2h
▌    ! merge failed: Pull request is n…
▌    acme/webapp · retry-hooks · approved
▌    ● build
  ⠹  #14 Bump serde                 1d
     marking ready… · acme/api · bump-serde
     ✓ 9 checks
```

- **The card:** a red line right under the title, `! <action> failed: <reason>`, cut with `…` to the card width. The check glyph and the rest of the card are unchanged.
- **What marks:** any `Err` result, an update that hit conflicts, a rerun where any run failed (`1 of 2 runs failed: …` when partial). Already up to date, merge sent and every other success do not.
- **Main builds too:** a failed rerun started there marks that build card under its title. A rerun from My PRs marks the pull request card.
- **Details pane:** with the marked card selected, `d` starts with `<action> failed at HH:MM:SS: <full reason>` in red, wrapped.
- **Column title:** counts marks, `My PRs (5) !1`, in the column block and in the tabs of the narrow layout.
- **Footer:** unchanged.
- **Clearing:** a new write starts on that row; `Esc` with the marked card selected; the row leaves its column on a successful refresh. A refresh never clears the mark of a row that is still there. `Esc` closes one thing per press: details pane, zoom, the selected card's mark, the live filter.
- The mark is per row key, like phase 38's spinner: the same number in two repositories shares it.

## Verified facts (read on `main` at d6c66e6, 2026-11-12)
- `App::writes: Vec<Write>` holds the writes in flight; `Write { subject, stage, kind }` owns `progress()` (footer) and `word()` (card); `start_write` and `finish_write` are in `src/app/actions/writes.rs`. Every `receive_*` calls `finish_write` and then overwrites `App::notice`, keeping nothing else. `receive_rerun` removes its own entry by run id.
- `BranchUpdate::Conflicts` is an outcome, not an `Err`; `receive_rerun` already builds `run <id>: <message>` per failed run and the partial wording.
- `handle_key` clears `notice` before anything else (`src/app/keymap.rs`), so the key that answers a prompt erases a result that landed under it.
- `escape()` closes the details pane, then zoom, then the live filter (`src/app/keymap.rs`).
- `pr_row(pr, busy, now, selected, width)` and `card(…, rest: Vec<Vec<Span>>, …)`: a red line is one more entry in `rest`. A pull request card is three lines, a build card four; `List` drops a card that does not fully fit.
- `draw_column` (`src/ui/columns.rs`) already looks up `busy` per visible row before `draw_list`; the title comes from `titled`, which also feeds the tabs of the narrow layout.
- `text::wrap(text, width, lines)` and `text::clock(at, utc_offset_secs)` exist in `src/text.rs`.
- `Listing` (`src/app/column.rs`) answers `selected_key` but not whether a key is still in the column; `App::receive` (`src/app/stages/mod.rs`) is where every fetch lands.
- `tests/manage/` is a folder since phase 38 (`popup.rs`, `parallel.rs`, `helpers.rs`), so the new tests go in `tests/manage/failures.rs`, not a new binary.

## Tasks (each starts red, each shipped with `scripts/ship.sh`)
1. **A failed status change marks its card.** `tests/manage/failures.rs`: `S` `t` on #12 fails, `S` `t` on #14 succeeds after it, My PRs refreshes; #12 still reads `! mark ready failed: <reason>` and #14 has no mark. Second test: the failure lands while the `S` popup is open on #14; after a key closes the popup the mark is on #12. Adds `Failure { stage, subject, action, message, at }` and `App::failures`, filled from the `Write` the result removes; `Write` gains the failure wording; `pr_row` takes the row's failure.
2. **Merge, update and checkout failures mark too.** An `Err` from each and an update with conflicts mark; already up to date, merge sent and a checkout that worked do not.
3. **A new write clears the mark.** With #12 marked, `S` `t` again shows the spinner and no mark; a success leaves none, a new failure shows the new reason.
4. **Reruns mark their own card.** `tests/builds/rerun.rs`: a failed rerun from Main builds marks that build card under its title, a partial one reads `1 of 2 runs failed: …`. `tests/prs/rerun.rs`: a failed rerun from My PRs marks the pull request card. `build_row` takes the row's failure like `pr_row`.
5. **The details pane carries the full reason.** `tests/prs/details.rs`: `d` on a marked card starts with `<action> failed at HH:MM:SS: <reason>`, wrapped, a long reason shown whole. One case for a build in `tests/builds/rerun.rs`.
6. **The column title counts marks.** Two marked cards read `My PRs (5) !2` in the column title and in the tab of the narrow layout; no mark, no suffix.
7. **`Esc` clears the selected card's mark.** `tests/prs/navigation.rs`: details pane first, then the mark; with a live filter, the mark first and the filter on the next press; `Esc` on an unmarked card behaves as today.
8. **A row that leaves its column takes its mark along.** A successful fetch without #12 drops its mark and the count; a failed fetch keeps it; a fetch that still holds #12 keeps it. `Listing` gains `holds(key)` so `App::receive` prunes without a `Stage` match.
9. **Snapshot and wrap-up.** `tests/snapshots.rs`: `failed_card_layout`, a marked card next to a busy one. Then `src/app/AGENTS.md` (the `Esc` order, what marks and what clears), `src/ui/AGENTS.md` (the red line, the title count), the root `AGENTS.md` layout, the failure half of the "hidden by that prompt" backlog item in `PLAN.md` marked done, the phase outcome in `PLAN.md`, `scripts/screenshots.sh` only if `docs/details.png` changes, and the fragment `changes/phase-39-failed-write-marks.md` (`kind: feat`).

## Verification
`scripts/gates.sh`. Then in `conveyor-dev`: `S` `c` on a pull request whose repository has no local clone configured (the checkout refuses, nothing is written to GitHub), then at once `S` `t` on a draft. The footer ends on the draft's result; the first card still carries `! check out failed: …` and the title reads `My PRs (N) !1`. `d` shows the whole reason and its time; `Esc` closes the pane, a second `Esc` clears the mark and the count.
