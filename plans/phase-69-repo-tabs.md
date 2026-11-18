# Phase 69: Merge queue and Main builds share one row of repository tabs

## Context

Issue #138, requested by the user on 2026-11-17. Phase 66 made Main builds show the repository the Merge
queue's active tab shows, one way: only the queue draws a tab bar, the tab keys do nothing
while Main builds is focused, and a repository without a merge queue shows a red column error
in the queue. The user wants the tabs on both columns, in sync both ways, and wants to see the
builds of a repository that has no merge queue without the queue column shouting an error.

## Decisions (taken with the user, 2026-11-17)

- **The same tab bar in both columns.** Merge queue and Main builds each draw the repository
  bar, one tab per `[[repo]]` in config order, short repo names, like My PRs draws its own. No
  bar spanning the two columns and no bar that moves with the focus.
- **One active repository for both.** `next_tab`, `previous_tab` and `1`-`9` work while either
  column is focused and move both. There is no state in which they show different repositories.
- **A repository without a merge queue keeps its tab and its queue column.** The queue column
  shows one dim line, `no merge queue`, where it shows `queue empty` today, with no error
  colour, no failure mark and no count in the title. The layout never changes on a switch.
- **Its name is struck through in the queue's bar** (owner's decision of 2026-11-18, superseding
  the issue's "dim tab"): `Modifier::CROSSED_OUT`, active or not, on top of the emphasis every
  tab has (the active one bold and reversed, the others dim), so the bar's width never changes.
  A tab is struck only once GitHub has answered "no queue" for it and loses the strike when a
  queue appears; the builds bar never strikes a name.
- **A failed call for a repository known to have no queue** keeps `no merge queue` in the body,
  adds `⚠` to the title and shows the error in the footer while the column is focused, like
  every loaded column (owner's decision of 2026-11-18).
- **GitHub says whether there is a queue.** No config key. `mergeQueue: null` in the queue
  answer is that state; the queue keeps being asked on its normal interval, so a queue that is
  enabled later appears by itself.

## Verified facts (2026-11-17, `main` at 069fa56, re-verified at 683d792)

- `QueueStage` (`src/app/stages/queue.rs`) holds `names`, one `QueueTab { column, repo }` per
  tab and `active`, and implements `tab_names`, `active_tab` and `select_tab` (which clears the
  filter of the tab it leaves and the tab it shows).
- `BuildsStage` (`src/app/stages/builds.rs`) holds one `BuildsTab { column, repo, branch }` per
  repository and `active`, overrides `active_tab`, and has no `tab_names`, so `tab_names()` is
  the trait default `&[]`. `BuildsStage::follow(tab)` is its only way to change tab.
- `App::switch_tab` (`src/app/navigation.rs`) returns early when the focused stage has fewer
  than two tab names (so the keys do nothing in Main builds), calls `select_tab` on the focused
  stage, then `self.builds.follow(self.queue.active_tab())`, clears every stage's filter, and
  returns `Action::Refresh(self.focus)`: one refresh, for the focused stage.
- `draw_column` (`src/ui/columns.rs:86`) draws a bar for any stage whose `tab_names()` has two
  or more names, through `tab_bar(names, active, width)`. Nothing in it is specific to a stage.
- The queue fetcher fetches only the tab last asked for (`spawn_queue_fetcher`); the builds
  fetchers run one per repository in the background (`spawn_builds_fetchers`, phase 66). So a
  switch made from Main builds must also ask the queue for the new tab.
- `model::queue::parse` (`src/model/queue.rs:57-60`) answers
  `Err("no merge queue on this repository")` for `mergeQueue: null`; it reaches the column as a
  `Failure` of kind `Other`, drawn as a red column error. `fetch_queue` asks for the
  merge-group runs only when there are entries.
- The empty texts are in `src/ui/columns.rs`: `queue empty` (line 153) and
  `no builds on <branch>` (line 117).
- The help row is `(TAB_KEYS, "next / previous / nth tab")` in `src/ui/help.rs`; it names no
  column, so `KEYS` and the README key table need no change.
- The demo has two `[[repo]]`, `acme/webapp` and `acme/infra`, both with a queue answer.
  Since phase 68 the built-in demo config passes the checks of `src/config/values.rs`, so a
  third demo repository must be a valid `[[repo]]` (`owner/name`, no second `[[repo.deploy]]`).
- conveyor's own repository has no merge queue, so `gh api graphql` against it is the live
  check of the `mergeQueue: null` shape: re-checked at 683d792, it answers
  `{"data":{"repository":{"mergeQueue":null}}}`.

## States of the queue answer, and what the queue column shows

| Answer for the active repository | Queue column body | Title | Its tab in the queue's bar | Marks, footer |
| --- | --- | --- | --- | --- |
| Not answered yet (first paint, or first visit to a tab) | `fetching…`, as today | `Queue <repo>`, the configured name | normal | spinner, as today |
| Entries | the cards, as today | `Queue <repo> (N)` | normal | as today |
| A queue with no entries | `queue empty`, as today | `Queue <repo> (0)` | normal | as today |
| `mergeQueue: null` | `no merge queue`, dim | `Queue <repo>`, no count | struck through | none: not an error |
| `mergeQueue: null`, then later a queue | the cards or `queue empty` | with its count | not struck | as today |
| A queue, then later `mergeQueue: null` (queue switched off) | `no merge queue`, the old cards gone | no count | struck through | none |
| The call fails (rate limit, offline, timed out, not signed in) | the column error, as today; the last rows stay as today | as today | keeps what it last knew | as today |
| The call fails for a repository last known to have no queue (owner's decision of 2026-11-18) | `no merge queue` stays, like every loaded column | `Queue <repo> ⚠`, no count | struck through | the error in the footer while the column is focused |
| No `[[repo]]`, or one | no bar in either column, as today | as today | no bar | as today |

Main builds is the same for every row above: its own bar, its own rows, its own error.

## Design

- **One place moves the tab.** `App::switch_tab` treats Merge queue and Main builds as a pair:
  when the focus is either, it calls `select_tab(tab)` on both and returns a refresh for both
  (the queue's new tab is not fetched in the background; a builds refresh is cheap and keeps
  the two columns equally fresh). `BuildsStage` gains `names` and implements `tab_names` and
  `select_tab`; `BuildsStage::follow` is deleted with its one caller, since `select_tab` now
  does its work (dead code is forbidden). My PRs keeps its own tabs, untouched.
- **"No queue" is data, not a failure.** `model::queue::parse` answers `Ok` with a `Queue` that
  says it has no queue (a field on `Queue`, not a new `FailureKind`), `QueueTab` remembers it
  per tab, and `receive` clears the tab's rows when it arrives. The column error path is not
  touched, so a real failure still reads as one.
- **The bar learns to strike a tab.** `tab_bar` asks the stage which tabs are struck;
  `StageView::tab_struck` answers it (default: none), which `QueueStage` answers from its tabs,
  and the bar adds `CROSSED_OUT` to that tab's usual style.
- **Nothing new is fetched.** A repository without a queue costs the same one GraphQL call per
  `refresh_secs` while its tab is shown as an erroring one does today.

Not in this phase: a config key to stop the queue calls for a repository; Deployed following
the tabs (it keeps reading the one repository with a `[[repo.deploy]]`); hiding the queue
column.

## Tasks

Each is one red-green-refactor cycle, outside-in from `run()` with scripted inputs
(`tests/queue_tabs.rs` holds the queue and builds tab tests), shipped with `scripts/ship.sh`.

1. **Main builds draws the repository bar.** With two `[[repo]]`, Main builds shows the same
   names as the queue's bar with the active one emphasised; with one or none, no bar.
2. **The tab keys work in Main builds and move both columns.** `next_tab`, `previous_tab` and a
   digit, with Main builds focused, change the repository both columns show; the same keys in
   the queue still move both. Past the last tab or on the active one they do nothing, as in
   My PRs.
3. **A switch from Main builds asks the queue for the new tab.** The requests after the key are
   a refresh of the queue's new tab and of the builds' new tab, in that order. Remove `follow`.
4. **The filter, the selection and the panes survive a switch from either column** as phase 66
   pinned them for a queue switch: both columns clear their filter and keep their selected row,
   the details and log scroll reset. Same tests, driven from Main builds.
5. **`mergeQueue: null` is "no queue", not an error.** `model::queue::parse` unit test with a
   fixture taken from this repository's own answer; then through `run()`: the queue column
   reads `no merge queue`, with no error and no count in its title.
6. **The states table, row by row:** a queue appearing later, a queue going away (old cards
   gone), a failed call after "no queue" (`no merge queue` stays, the title gains `⚠`, the
   footer shows the error while the column is focused, the tab stays struck).
7. **The struck tab.** The queue's bar strikes the tab of a repository known to have no queue,
   active or not; the builds bar does not; the active tab keeps its emphasis.
8. **The demo and the screenshots.** A third demo repository, `acme/docs`, with builds and a
   `mergeQueue: null` answer; `tests/demo.rs` follows. `docs/details.png` now shows the bar in
   both columns; `docs/infra.png` is re-rendered; a new `shoot` line renders the `acme/docs`
   tab as `docs/no-queue.png`. Embed the images in the pull request, pinned to their commit.
9. **Docs and the fragment.** `src/app/AGENTS.md` (the Merge queue and Main builds tab
   bullets, the refresh rule), `src/ui/AGENTS.md` (the bar, the struck tab, the empty texts),
   `src/sources/AGENTS.md` and `src/model/AGENTS.md` (a repository without a queue is no longer
   a column error), the README comment on `[[repo]]`, and `changes/phase-69-repo-tabs.md`
   with `kind: feat`.

## Outcome

## Hurdles
