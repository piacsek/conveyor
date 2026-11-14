# Phase 40: tabs in the My PRs column, switched with Tab or 1-9

Issue: [#79](https://github.com/piacsek/conveyor/issues/79).

## Context
Each column shows one fixed view taken from its config. The user wants several views per
column, cycled with a key that defaults to `Tab`, shown as tabs at the top of the column
with the current one highlighted. Breaking today's `Tab` (focus the next column) is accepted.

Decided with the user on 2026-11-14:
- **My PRs only, this phase.** A tab is a named search query. Merge queue, Main builds and
  Deployed keep one view and draw no tab bar. Tabs there (one per `[[repo]]`, which also
  closes the "repo switcher" backlog item) are a later phase.
- **Only the active tab fetches.** Inactive tabs keep their last rows; switching shows them
  at once and starts a refresh. The number of `gh` calls does not grow with the tabs.
- **The key is configurable**, defaulting to `Tab` (next) and `Shift-Tab` (previous). This
  is the first key conveyor lets you rebind.
- **`1`-`9` pick a tab** in the focused column, and stop focusing columns. Columns are
  reached with `h`/`l` only. The digits are built in, not configurable.
- **The filter resets when the tab changes.** Moving between columns keeps it, as it does
  today.
- **One tab, no bar.** The bar appears only when a column has two tabs or more, so an
  existing config looks exactly as it does today.

## Compatibility: today's config keeps working
Every config file that loads on `main` loads unchanged after this phase and shows the same
columns. Only additions: `[[prs.tab]]` and `[ui.keys]`, both optional, nothing renamed,
moved or made required. `[prs].query` and `[prs].limit` keep their meaning: with no
`[[prs.tab]]` they are the single tab, and `limit` stays the default for every tab.
`conveyor config` on a config without tabs prints no `tab = []` (`skip_serializing_if`
empty); it does gain the `[ui.keys]` defaults, since it is the reference for defaults. The
only new errors are for the new keys (both `[prs].query` and `[[prs.tab]]`, a bad key name).
What does change on purpose is keys, not config: `Tab`/`Shift-Tab` and `1`-`4` no longer move
focus between columns; `h`/`l` still do.

## Design

```toml
[prs]
limit = 20
refresh_secs = 60

[[prs.tab]]
name = "mine"
query = "is:pr is:open archived:false author:@me"

[[prs.tab]]
name = "review"
query = "is:pr is:open review-requested:@me"
limit = 10            # optional; falls back to [prs].limit

[ui.keys]
next_tab = "tab"
previous_tab = "shift-tab"
```

```text
┌My PRs (3)───────────────────────┐
│ mine │ review │ team            │
│▌ ●  #12 Retry webhooks       2h │
│▌    acme/webapp · retry-hooks   │
```

- **Config.** `[[prs.tab]]` holds `name` and `query`, and an optional `limit`. With no
  `[[prs.tab]]`, `[prs].query` is the single tab, as it is today. Setting both `[prs].query`
  and `[[prs.tab]]` is a config error naming the file: one of them would be silently
  ignored. A tab without a name or a query, or two tabs with the same name, is an error
  too. `conveyor config` prints the tabs.
- **Keys.** `[ui.keys] next_tab` and `previous_tab` take a key name: a single character
  (`"]"`), `"tab"`, `"shift-tab"`, or `"ctrl-<char>"`. An unknown name is a config error. So
  is a key conveyor already uses (`j`, `S`, `/`, …), with the error naming what that key
  does: a rebind that silently shadowed `S` would hide the action popup; the digits count as
  in use. `1`-`9` jump to that tab of the focused column: a digit past the last tab, or the
  tab already active, does nothing. `h`/`l` move focus between columns; `Tab` and `1`-`4`
  no longer do.
- **The bar.** It takes one line inside the column block, above the cards. Tab names are
  separated by ` │ `. The active tab is bold and reversed, like the narrow layout's column
  bar, and the others are dim. When the names do not fit, the active one stays visible and
  the bar is cut with `…`. In the narrow layout, the column bar stays on top and the tab bar
  sits inside the column below it.
- **Each tab is its own column state.** Each tab keeps its own rows, selection, scroll,
  error and fetch time, so switching back returns you to where you were. The filter is not
  kept: changing tab clears it (a live one and a kept one alike). The column title
  (`My PRs (5) !1`) counts the active tab.
- **Fetching.** The My PRs fetcher fetches the active tab on its interval. Switching sends a
  refresh that names the new tab. Every result carries the index of the tab it was fetched
  for and lands in that tab, never in whichever tab is active when it arrives.
- **Help and README.** `KEYS` gains `Tab S-Tab 1-9` → `next / previous / nth tab`, and the
  focus row becomes `h/l` → `focus column`. The README mirrors the defaults. The help view (`?`) shows the keys
  that are actually configured.

### States and what the column shows (review before any code)

| State | Bar | Cards | Title |
| --- | --- | --- | --- |
| One tab (no `[[prs.tab]]`, or a single one) | none | as today | `My PRs (N)` |
| Active tab never fetched, fetch running | active highlighted | empty, spinner in the footer | `My PRs` |
| Active tab fetched, refresh running | active highlighted | last rows | `My PRs (N)` |
| Switch to a tab fetched before | new tab highlighted | that tab's last rows and selection, no filter; a refresh starts | its count |
| Filter kept (`/` … `Enter`), user switches tab | new tab highlighted | unfiltered rows of the new tab; switching back shows the old tab unfiltered too | its count |
| Switch to a tab that failed last time | highlighted | its last rows (if any) and its error, as a column error shows today | its count, `⚠` as today |
| Result lands for an inactive tab (switched mid-fetch) | unchanged | unchanged; the rows are stored in that tab for later | unchanged |
| Error lands for an inactive tab | unchanged | unchanged; shown when you switch to it | unchanged |
| Write in flight on #12, user switches tab | - | #12 spins on every tab that lists it | - |
| #12 marked failed, user switches to a tab without #12 | - | mark kept; shown again when #12 is drawn | `!N` counts the active tab's marked rows |
| A successful refresh of a tab without #12 | - | #12's mark is dropped only when no tab's last rows hold #12 | - |
| Merge of #12 waiting for My PRs to catch up | - | the wait ends on the first My PRs fetch that started after the merge, whatever its tab (unchanged rule) | - |
| Details pane open, user switches tab | - | pane shows the new tab's selected row, scroll reset | - |
| Zoomed on My PRs, user switches tab | bar drawn in the zoomed column | new tab | - |
| `Tab` or a digit pressed in a column with one tab | - | nothing happens | - |
| Digit past the last tab, or the active tab's digit | - | nothing happens, no refresh | - |
| Filter being typed (`/`), `Tab` pressed | - | nothing happens. Today `Tab` falls through `handle_filter_key` to the normal keys and moves focus. Digits and a character key such as `]` are typed into the filter, as any character is today | - |
| Moving to another column with a filter kept | - | unchanged from today: the filter stays | - |

## Verified facts (read on `main` at 83e0063, 2026-11-14)
- `Config.prs: Prs { query, limit, refresh_secs, merge_method, update_method }`. Every table
  is `deny_unknown_fields` (`src/config.rs`). `Ui { min_column_width, details_percent }` has
  no keys table.
- `fetch_prs(gh, config)` reads `config.prs.query`/`limit` (`src/sources/fetch.rs:13`).
  `spawn_prs_fetcher` (`src/main.rs:336`) wraps it in `refresh::spawn`, whose refresh
  channel is `Sender<()>` and whose results are `Input::Data(Stage, Result<Rows, String>)`
  with no tab.
- `KeyCode::Tab`/`BackTab` call `refocus(Stage::next/previous)` (`src/app/keymap.rs:145`).
  `KeyCode::Char('1'..='4')` focuses the column at that index (`src/app/keymap.rs:97`). Neither
  `refocus` nor `reset_scroll` touches the filter (`src/app/navigation.rs:26`).
  `KEYS[2]` reads `("h/l Tab 1-4", "focus column")` (`src/ui/help.rs`).
- The narrow layout already draws a bar of column names (`draw_tabs`, `src/ui/mod.rs:83`),
  bold and reversed for the focused one. The new bar reuses that style. `draw_tabs` is
  renamed `draw_column_bar` so that "tab" means only the new thing.
- `App::prs: PrsStage` holds one `column: Column<PullRequest>`. `App::receive` prunes failure
  marks through `Listing::holds` and clears `merge_landing` entries whose fetch had started
  (`src/app/stages/mod.rs:168`).
- Outside `stages/prs.rs`, `app.prs.column` is read in `src/ui/details.rs:21` and
  `src/ui/columns.rs:36,88-97`. Those reads go through an accessor for the active tab.
- `src/app/stages/prs.rs` is 225 lines and `src/config.rs` 186, both well under the
  700-line limit.

## Tasks (each starts red, each shipped with `scripts/ship.sh`)
0. **Today's config is pinned first.** `tests/cli.rs`, before any config change: a fixture
   `tests/fixtures/config-v0.toml` using every key that exists on `main` (`[prs]` with
   `query` and `limit`, `[ui]`, `[[repo]]` with `[[repo.deploy]]` and `[[repo.deploy.env]]`,
   invented `acme/*` values) loads, and `conveyor config` prints it back. That test stays
   green, unedited, through every later task; only the `[ui.keys]` lines are added to the
   expected output, in task 2.
1. **Config reads tabs.** `tests/cli.rs`: `conveyor config` with two `[[prs.tab]]` prints
   them; with none it prints today's output; `[prs].query` together with `[[prs.tab]]`, a tab
   without a query, and a repeated name each exit 1 naming the file. Adds `PrsTab` and
   `Prs::tabs()` (the implicit one when the list is empty).
2. **Key names parse.** Unit tests for the key parser (`tab`, `shift-tab`, `]`, `ctrl-n`, an
   unknown name). `[ui.keys]` in the config, defaults `tab`/`shift-tab`. A key already in use
   is an error naming its meaning.
3. **The fetcher fetches the active tab.** `refresh::spawn` takes a refresh channel that
   carries a tab index. The Prs result carries its tab. A unit test drives `fetch_prs` for
   tab 1 through the `Github` fake and checks the query and limit it sends.
4. **`Tab` cycles My PRs tabs.** TUI test (`tests/prs/tabs.rs`): two tabs, `Tab` shows the
   second tab's rows and sends a refresh for tab 1; `Shift-Tab` comes back to the first tab
   with its selection intact and its filter cleared; `Tab` on Main builds does nothing; `h`/`l`
   still move focus and keep the filter, as today. `PrsStage` holds one `Column` per tab plus
   the active index.
4b. **Digits pick a tab.** Three tabs: `3` shows the third, `1` the first; `4` and the active
   tab's digit do nothing and send no refresh; `2` on Main builds does nothing and focus stays
   (today it would focus Merge queue); while typing a filter, `2` is typed into it.
5. **A late result lands in its own tab.** Switch mid-fetch; tab 0's result arrives and is
   stored in tab 0, not shown in tab 1. Same for an error.
6. **The bar draws.** Snapshot `prs_tabs_layout`: three tabs, the second active; one-tab
   config draws no bar (the existing snapshots stay unchanged); a narrow width keeps the
   active name visible with `…`; the narrow layout draws the column bar and then the tab bar.
7. **Marks, spinners and the merge wait across tabs.** Implements the rows of the state
   table: a mark survives a switch and a refresh of a tab without the row, and drops when no
   tab holds it; a busy row spins on both tabs; the merge wait ends on a post-merge fetch of
   any tab.
8. **A configured key works.** `next_tab = "]"`: `]` switches and `Tab` does nothing. The
   help view shows `]`.
9. **Help, README, wrap-up.** `KEYS` and the README table (`tests/readme.rs` pins them), the
   default config in the README, `src/app/AGENTS.md` (the tab keys, per-tab state, which
   fetch result lands where), `src/ui/AGENTS.md` (the bar), `src/sources/AGENTS.md` (the
   refresh channel carries a tab), root `AGENTS.md` layout lines if a file is added, the
   phase outcome in `PLAN.md`, the backlog entry "tabs for Merge queue, Main builds,
   Deployed (one per `[[repo]]`)", `scripts/screenshots.sh` with a two-tab fixture so
   `docs/details.png` shows the bar, and the fragment `changes/<branch>.md` (`kind: feat`).

## Verification
`scripts/gates.sh`. Then in `conveyor-dev` with two `[[prs.tab]]` (`author:@me` and
`review-requested:@me`): the bar shows both with the first highlighted; `Tab` switches, the
second tab fills after one fetch, and `gh` calls in the log name only the active query on
each interval; `Shift-Tab` returns to the first tab with the same row selected. Set
`next_tab = "j"`: `conveyor config` exits 1 naming the conflict with `move`.

## Left out on purpose
- Inactive tabs show no "stale since" age in the bar; the footer's fetch time covers the
  active tab, and switching starts a refresh.
- Tabs for Merge queue, Main builds and Deployed: a later phase (backlog, task 9).
