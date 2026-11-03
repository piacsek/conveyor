# Phase 26: `S` is the one key that changes things

## Context
Write actions were spread over `X` (rerun failed jobs), `M` (merge) and `U` (update branch).
`S` replaces all three with one popup for the selected card, and adds pull request status
changes: draft ↔ ready, auto-merge on/off, leaving the merge queue, closing.

## Behaviour
`S` opens `Manage #N` (My PRs) or `Manage run N` (Main builds) over the body: one
`letter  label` line per action. An action that does not apply is drawn dim with its reason
(`m  merge · #12 is a draft`) and its letter does nothing. Any other key cancels, `q` included.
Letters avoid every Normal-mode key, so a habit press cancels instead of writing.

Merge, enabling auto-merge (on a merge-queue repo it enqueues), close and a rebase update (force-push) ask `y` in the confirm dialog. The rest act on
the letter: the popup is the confirmation and each is cheap to undo.

| key | action                        | offered when                                      | then        |
|-----|-------------------------------|---------------------------------------------------|-------------|
| `m` | merge                         | `merge_target` rules                              | confirm `y` |
| `u` | update / rebase branch        | `update_target` rules                             | `y` on rebase |
| `f` | rerun failed jobs             | `rerun_target` rules                              | direct      |
| `t` | mark ready / convert to draft | draft, or open and not queued                     | direct      |
| `a` | enable / disable auto-merge   | `viewerCanEnableAutoMerge` / `viewerCanDisableAutoMerge` | `y` to enable |
| `e` | exit the merge queue          | queued                                            | direct      |
| `x` | close                         | always                                            | confirm `y` |

Main builds offer only `f`, with today's newest-run guards. Merge queue and Deployed answer
with a footer notice. One write is in flight at a time across all actions. A PR card's meta
line reads `auto-merge` while it is enabled. No reopen: the default query lists open pull
requests only, so a closed one leaves the column.

## Verified facts (2026-11-03, gh 2.103.3)
- `gh pr ready <N> --undo` converts to draft; `gh pr merge --auto`, `--disable-auto` exist.
- `dequeuePullRequest(input: { id: ID! })` takes the pull request's node id.
- `PullRequest` has `autoMergeRequest { enabledAt … }`, `viewerCanEnableAutoMerge`,
  `viewerCanDisableAutoMerge`: GitHub decides whether auto-merge applies (drafts, repo setting),
  so conveyor does not guess.

## Tasks (each starts red)
1. Parse `id`, `autoMergeRequest`, `viewerCan{Enable,Disable}AutoMerge`; `auto-merge` on the card.
2. `Github::change_status` and its `gh` argv; dequeue through `graphql`; e2e shim logs argv.
3. `Mode::Manage` with offers and reasons; `S` opens it; letters dispatch or confirm.
4. Remove `X`/`M`/`U`; migrate their tests to `S` + letter; one in-flight write.
5. `draw_manage`; worker arm; help `KEYS`, README, AGENTS.md; v0.21.0 and its changelog entry.

## Verification
`scripts/gates.sh`; `conveyor-dev` on a throwaway draft pull request in this repo: `S t`
twice, `S a` twice, `S x y`.
