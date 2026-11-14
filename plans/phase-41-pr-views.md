# Phase 41: several lists in My PRs

## Context
My PRs shows one fixed query. Issue #80 asks for several named lists ("views") in that column, for example "Mine" and "Review requested", switched with a key and drawn as a strip at the top of the column with the active one highlighted. Scope is My PRs only; the other columns get views in a later phase. Code and docs say "view" because the narrow layout already calls its column bar "tabs".

Decided with the user:
- Tab may stop moving focus; the key defaults to Tab and is configurable.
- `1`-`9` jump to a view in the focused column. The `1`-`4` column jump goes away; `h`/`l` stay.
- One fetcher and one cache per view: switching is instant, and each view keeps its own selection, filter and scroll.

## Design

```toml
[prs]
name = "Mine"
query = "is:open author:@me"

[[prs.view]]
name = "Review"
query = "is:open review-requested:@me"
limit = 10

[keys]
next_view = "tab"
```

```text
┌My PRs (3)───────────────────────┐
│ 1 Mine │ 2 Review               │
│▌ ●  #12 Retry webhooks       2h │
│▌    acme/webapp · retry-hooks   │
```

- **Config.** `[prs]` is the first view, named by `prs.name` (default "Mine"). `[[prs.view]]` adds views after it: `name`, `query`, optional `limit` (default `prs.limit`). More than 9 views, an empty name or query, or a repeated name is a config error naming the file. `[keys].next_view` takes `"tab"` or a single character; anything else is a config error. `view` is skipped when empty so `conveyor config` output is unchanged for existing configs, apart from the new `[keys]` default.
- **State.** `PrsStage` holds one `Column<PullRequest>` per view and the active index. Selection, scroll, filter, error and fetch time are per view. The column title counts the active view.
- **Fetching.** One `refresh::spawn` per view, each with its own query and limit. `Input::Data`, `Input::Fetching` and `Request::Refresh` carry a view index (0 for every other stage); a result for an unknown index is dropped. `r` refreshes the active view, `RefreshAll` every view, and the PR follow-up refresh every PR view.
- **Keys.** The `next_view` key goes to the next view and wraps; Shift-Tab goes to the previous one (only while `next_view` is Tab). `1`-`9` jump to that view. All of them act only when My PRs is focused and has more than one view; a digit past the last view does nothing. A switch resets the scroll and syncs the filter of the new view. `h`/`l` are the only column focus keys.
- **The strip.** One row inside the column block, drawn only with more than one view, so existing setups look the same. Active view bold and reversed (the narrow column bar's style), the others dim, separated by ` │ `. When the names do not fit, the active one stays visible and the strip is cut with `…`. The list and the OSC 8 link overlay use the area below the strip.
- **Help and README.** `KEYS` focus row becomes `h/l` → `focus column`, plus `Tab 1-9` → `switch view`; `keys_for` shows the configured key. The README table mirrors the defaults.

### States and what the column shows

| State | Strip | Cards | Title |
| --- | --- | --- | --- |
| One view | none | as today | `My PRs (N)` |
| Active view never fetched | active highlighted | empty, loading | `My PRs` |
| Active view fetched, refresh running | active highlighted | last rows | `My PRs (N)` |
| Switch to a view fetched before | new one highlighted | its rows, selection, scroll and filter | its count |
| Result lands for an inactive view | unchanged | unchanged, stored in that view | unchanged |
| Error lands for an inactive view | unchanged | unchanged, shown when you switch to it | unchanged |
| Write in flight on #12 | n/a | #12 spins in every view that lists it | n/a |
| #12 marked failed, view without #12 active | n/a | mark kept, shown when #12 is drawn | `!N` counts the active view's marked rows |
| Refresh of a view without #12 | n/a | mark dropped only when no view holds #12 | n/a |
| Details pane open, view switched | n/a | pane follows the new view's selected row | n/a |
| Zoomed, view switched | strip drawn in the zoomed column | new view | n/a |
| Key or digit with one view, a digit past the last, or the active view's digit | n/a | nothing happens | n/a |
| Focus on another column, digit pressed | n/a | nothing happens | n/a |

## Verified facts (read on `main` at 83e0063, 2026-11-14)
- `Config { prs, ui, repo }`, every table `deny_unknown_fields` (`src/config.rs`). `tests/config.rs` round-trips `conveyor config`.
- `Tab`/`BackTab` call `refocus` and `1`-`4` focus a column (`src/app/keymap.rs`). Tests pinning them: `tests/prs/navigation.rs`, `tests/prs/cards.rs`.
- One fetcher thread per `Stage` via `refresh::spawn` (`src/refresh.rs`); the `refreshers` map is keyed by `Stage` (`src/main.rs`). `fetch_prs` (`src/sources/fetch.rs`) reads `prs.query` and `prs.limit`.
- `PrsStage` wraps one `Column<PullRequest>` (`src/app/stages/prs.rs`); `Column` holds state, error, list and filter (`src/app/column.rs`). Only `src/app/stages/mod.rs` may match on `Stage` in `src/app/` (`tests/stage_dispatch.rs`).
- The title comes from `titled` in `src/ui/columns.rs`; the narrow layout's column bar is `draw_tabs` in `src/ui/mod.rs`.
- `KEYS` (`src/ui/help.rs`) is mirrored word for word by the README table (`tests/readme.rs`).

## Tasks (each starts red, each shipped with `scripts/ship.sh`)
1. **Config reads views.** `tests/config.rs`: two `[[prs.view]]` load and print back; none leaves today's output; a repeated name, an empty query and a 10th view are errors naming the file. Adds `PrsView` and `Prs::views()`.
2. **Keys config.** `[keys].next_view`: `"tab"` and a single character load; `"ctrl-x"` is an error; round-trip.
3. **State per view.** TUI test: a result for view 1 lands in view 1 while view 0 is active; an unknown index is dropped. `PrsStage` holds one column per view; `Input`/`Request` carry the index.
4. **One fetcher per view.** A fetch test per view: each sends its own query and limit; `r` refreshes only the active view; a merge refreshes all.
5. **Keys switch views.** `tests/prs/views.rs`: the key cycles, Shift-Tab goes back, `1`-`9` jump, each view keeps its selection and filter; one view or another focused column does nothing; `h`/`l` still move focus; update the Tab and `1`-`4` assertions in `tests/prs/navigation.rs` and `tests/prs/cards.rs`.
6. **The strip draws.** Snapshot `views_layout`: three views, the second active; one view draws no strip; a narrow width keeps the active name with `…`; zoomed and narrow layouts. List and links sit below the strip.
7. **Marks, spinners and the merge wait across views**: the rows of the state table.
8. **A configured key works.** `next_view = "]"`: `]` switches, Tab does nothing, help shows `]`.
9. **Docs.** `KEYS` and the README table, README config block, `src/app/AGENTS.md`, `src/ui/AGENTS.md`, `src/sources/AGENTS.md` if the refresh channel changes, root `AGENTS.md` layout lines, `PLAN.md` outcome, `scripts/screenshots.sh` with a two-view fixture, fragment `changes/phase-41-pr-views.md` (`kind: feat`).

## Verification
`scripts/gates.sh`. Then in `conveyor-dev` with two views: the strip shows both with the first highlighted; the key and `2` switch, each view fills after its own fetch and keeps its selection and filter; `gh` calls in the log show both queries each interval; `next_view = "ctrl-x"` makes `conveyor config` exit 1 naming the file.
