# Phase 38: manage several pull requests at once

## Context
`S` refuses while any write is in flight (`App::write_in_flight`, `src/app/actions/mod.rs`): after `mark ready` on #A, `S` on #B only prints `marking #A ready…` until GitHub answers. The user wants to act on other pull requests meanwhile, and wants each busy card to show it.

Outcome: `S` is held back only on the row that already has a write running. A busy My PRs card swaps its check glyph for the footer's spinner and starts its meta line with the action (`marking ready…`), as the user picked:

```
▌ ⠹  #12 Retry webhooks            2h
     marking ready… · webapp · retry-hooks
     ✓ 14 checks
```

## Verified facts (read in the code, 2026-11-12)
- In-flight state is five single slots on `App` (`src/app/mod.rs`): `merging`, `updating`, `changing`, `checking_out`, `rerunning`. Each `send_*` sets one, each `receive_*` clears it. `write_in_flight` is read only by `ask_to_manage`.
- Results name their pull request: `Input::Merged(n, …)`, `BranchUpdated(n, …)`, `StatusChanged(n, change, …)`, `CheckedOut(target, …)`. `Input::Rerun(Vec<(run_id, result)>)` names only run ids, and `receive_rerun` takes `(Stage, label)` from the single slot.
- Each write kind has one worker thread draining a channel (`spawn_merger`, `spawn_status_changer`, … in `src/main.rs`). Two writes of the same kind already queue and run in order; different kinds run in parallel. No worker change is needed.
- Row keys (`Row::key`, `src/app/column.rs`): the pull request number in My PRs, the run id in Main builds. `Listing::selected_key` returns it.
- `merge_landing: Option<(u64, bool)>` is global too: `PrsStage` refuses `u` and `t` (to draft) on every pull request while it is set, without comparing the number (`src/app/stages/prs.rs`). After merging #A, `u` on #B is refused for one refresh.
- `pr_row(pr, now, selected, width)` (`src/ui/rows.rs`) is called from `src/ui/columns.rs` with no app state. `card()` takes the glyph as `(char, Color)`; `meta()` joins parts and cuts from the end, so a first part survives a narrow card.
- `App::spinner()` (`src/app/stages/mod.rs`) picks a braille frame from `app.now`; the 250 ms tick advances it.
- The footer holds one `notice`; with several writes running, the latest progress or result wins.
- Tests pinning the global block: `tests/merge.rs` (around line 231) and `tests/update_branch.rs` (around line 313) press `S` on the same pull request, so they stay valid as same-row tests.
- `tests/stage_dispatch.rs` forbids matching on `Stage` in `src/app/` outside `stages/mod.rs`.

## Design
- **One list instead of five slots.** New `src/app/actions/writes.rs`: `Write { subject: u64, stage: Stage, kind }` with `kind` one of `Merge`, `Update`, `Status(StatusChange)`, `Checkout`, `Rerun { run_ids, label }`, held in `App::writes: Vec<Write>`. `subject` is the row key of the row the write was started on (`selected_key` of the focused stage, which needs no `Stage` match). It owns the wording: `progress()` (the footer line, as today) and `word()` (the card: `merging…`, `updating…`, `marking ready…`, `rerunning…`, `checking out…`).
- **`S` asks about the selected row only.** `ask_to_manage` refuses with that write's `progress()` when `writes` holds the focused stage and selected key; otherwise it opens the popup.
- **Results remove their own entry.** Pull request results match on number and kind. `receive_rerun` matches on the run ids of the result, and reads stage and label from the entry.
- **`merge_landing` becomes per pull request.** `landing: Vec<(u64, bool)>` on `App` and `Context`; `fetching(Prs)` marks every entry started, a My PRs `receive` drops the started ones. `PrsStage` refuses `u` and `t` only when the selected pull request is in it. Wording unchanged.
- **The card.** `pr_row` gains `busy: Option<(char, &str)>` (spinner frame, word); `columns.rs` looks it up per row from `app.writes`. Busy: the spinner replaces the check glyph and the word leads the meta parts. Main builds cards are left as they are; a rerun there still shows in the footer.
- Same pull request number in two repositories shares a key, as rows already do: one blocks the other. Left as is.

## Tasks (each starts red, each shipped with `scripts/ship.sh`)
1. `S` opens on #B while a status change on #A is in flight, and still refuses on #A. `tests/manage.rs`. Introduces `writes.rs` for the status kind.
2. The same for merge, update and checkout: the remaining slots move into `writes`. `tests/merge.rs`, `tests/update_branch.rs`, `tests/checkout.rs`.
3. Two reruns in flight: each result names its own label and refreshes its own stage. `tests/manage.rs` (or the rerun tests under `tests/builds/`).
4. After a merge of #A lands, `u` and `t` on #B are offered; on #A they are still refused until My PRs catches up. `tests/update_branch.rs`.
5. A busy card shows the spinner and the action word; it returns to the check glyph when the result lands; two busy cards each show their own word. `tests/prs/`, snapshot if one covers a busy card.
6. Guidelines and wrap-up: `src/app/AGENTS.md` (the `S` refusal and `merge_landing` bullets), `src/ui/AGENTS.md` (the busy card), fragment `changes/phase-38-parallel-writes.md` (`kind: feat`), outcome in `PLAN.md`.

Setup before task 1: worktree `phase-38-parallel-writes` off `origin/main`, this plan copied to `plans/phase-38-parallel-writes.md` and committed with the first ship. #69 (clickable numbers) also edits `src/ui/columns.rs`; whichever merges second takes a small merge of `main`.

## Not in this phase
- A result that lands while a popup or confirm dialog is open stays hidden by it (already in the backlog).
- Parallel workers per kind: two status changes still run one after the other, each card spinning until its own answer.

## Verification
- `scripts/gates.sh` in the worktree.
- `conveyor-dev`: `S` `t` on a draft, move down, `S` `t` on another draft before the first answers. Both cards spin with `marking ready…`, both end ready, the footer names the last result. `S` on a spinning card prints its progress line and opens nothing.
