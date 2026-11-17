# Phase 44: My PRs tabs load and refresh in the background

## Context
My PRs has one tab per `[[prs.tab]]` (phase 40). Only the active tab is fetched: a tab never
visited is empty until you switch to it, and a tab you left shows rows as old as your last
visit. The user runs `mine | review` and wants `review` to be loaded and current without
switching to it, the way the focused tab is.

Asked by the user on 2026-11-16:
- Every My PRs tab loads at start and refreshes on the interval, active or not.
- It is configurable.

Proposed here, to confirm before any code:
- **One key, `[prs] background_tabs`, default `true`.** `false` is today's behaviour.
- **My PRs only.** Merge queue tabs keep fetching the active tab only (see "Left out on
  purpose").

## Compatibility: today's config keeps working
One optional key is added; nothing is renamed or made required. A config without
`[[prs.tab]]` has one tab and behaves exactly as today, whatever the key says. A config with
tabs and no key starts fetching every tab: one more search call per extra tab per
`refresh_secs`. `tests/fixtures/config-v0.toml` and its `tests/cli.rs` test stay green, unedited.

## Design

```toml
[prs]
refresh_secs = 60
background_tabs = true

[[prs.tab]]
name = "mine"
query = "is:pr is:open archived:false (author:@me OR assignee:@me)"

[[prs.tab]]
name = "review"
query = "is:pr is:open archived:false review-requested:@me"
```

- **The sweep.** The My PRs fetcher thread fetches every tab once at start and once per
  `refresh_secs`, one after the other on the same thread (the spawn lock already serialises
  `gh`), the active tab first, then the rest in config order. Each fetch sends its own
  `Input::Fetching(Prs, tab)` and `Input::Data(Prs, tab, …)`, which already land in their own
  tab.
- **A request fetches one tab.** `Request::Refresh(Prs, tab)` (a switch, `r`, a write's
  refresh, the follow-up) fetches that tab only, as today, and makes it the tab the next
  sweep starts with.
- **A request does not wait behind a sweep, and does not delay it.** Between two tabs of a
  sweep the fetcher looks at its refresh channel: a waiting request is fetched next, then the
  sweep goes on with the tabs it has left. The sweep runs on its own deadline (`refresh_secs`
  after the previous sweep started its wait), so switching tabs every few seconds cannot
  starve the hidden ones.
- **`background_tabs = false`** hands the fetcher a sweep of one tab, the active one: the
  loop is then exactly today's.
- **Nothing changes in `App`.** A switch still clears the filter and asks for a refresh of
  the tab it lands on, so the tab you look at is fresher than the interval. The footer
  spinner reads the active tab of each column (`App::any_refreshing`), so a hidden tab's
  fetch does not spin it.
- **`refresh::spawn` takes the sweep size.** Queue, Main builds and Deployed pass one, and
  keep today's loop.

### States and what the column shows (review before any code)

| State | Before | After (`background_tabs = true`) |
| --- | --- | --- |
| Start, two tabs | `mine` fetched; `review` empty until visited | `mine` fetched first, then `review` |
| First switch to `review` | empty, then rows when its fetch lands | its rows at once; a refresh still starts |
| Sitting on `mine` for 10 minutes, then switch | `review` rows 10 minutes old until the refresh lands | at most `refresh_secs` old |
| Switch while the sweep is on another tab | waits for the running fetch | waits for the running fetch only, then the asked tab, then the rest of the sweep |
| `r` on the active tab | that tab | that tab; the others on the next sweep |
| A hidden tab's query fails | not fetched, so no error | its error is kept in the tab and shown on the switch; the next sweep retries |
| Merge from `mine`, PR also listed in `review` | `review` lists it until visited | gone from `review` within `refresh_secs` |
| The merge wait (`merge_landing`) | ends on the first fetch of the merge's tab started after it | same rule; a sweep fetch of that tab counts |
| One tab, or no `[[prs.tab]]` | one fetch per interval | same |
| `background_tabs = false` | - | the "Before" column |
| Filter typed on `mine` while `review` lands | - | untouched: each tab keeps its own filter, selection and scroll |

## Verified facts (read on `main` at 72fb31d, 2026-11-16)
- `refresh::spawn` (`src/refresh.rs`, 43 lines) runs one loop per stage with one `tab`
  variable: fetch it, then `wait` for the interval or a request, which names the next tab.
  `wait` takes one request and drains the rest (the coalescing `src/app/AGENTS.md` describes).
- `spawn_prs_fetcher` (`src/main.rs:299`) passes the tab to `fetch_prs(&gh, &config, tab)`,
  which reads `config.prs.tabs()[tab]` (`src/sources/fetch.rs:16`): one GraphQL search per tab.
- `App::receive` and `App::fetching` (`src/app/stages/mod.rs:191,224`) already route by tab
  through `listing_in_mut(tab)`; `PrsStage` keeps one `Column` per tab. A result for a hidden
  tab is stored there today (phase 40 covers it for a late result).
- `App::any_refreshing` reads `stage.listing()`, the active tab only.
- `merge_landing` entries are `(number, landed_in, started)`; `fetching` and `receive` compare
  `landed_in` with the fetched tab, so a sweep needs no change there.
- `Prs` serialises through `PrsToml` (`src/config.rs:38`); a new key goes in both, and in the
  README default config block (`README.md:65`) and the `tests/cli.rs:257` expectation.
- `tests/refresh.rs` holds the fetcher tests; `tests/e2e.rs` logs `gh` argv through the shim.

## Tasks (each starts red, each shipped with `scripts/ship.sh`)
1. **`[prs] background_tabs` loads and prints.** `conveyor config` prints
   `background_tabs = true` by default and `false` when set; an unknown spelling is still
   refused. README default config and `tests/cli.rs` follow.
2. **The fetcher sweeps every tab.** `tests/refresh.rs`: a sweep of two sends
   `Fetching`/`Data` for tab 0 then tab 1 at start, and again after the interval; a sweep of
   one sends tab 0 only, as today.
3. **A request fetches one tab and the sweep keeps its deadline.** A request for tab 1
   mid-interval fetches tab 1 only; the next sweep starts with tab 1 and is not pushed back;
   a request that arrives during a sweep is fetched before the sweep's remaining tabs.
4. **My PRs uses it.** `spawn_prs_fetcher` passes `config.prs.tabs().len()` when
   `background_tabs`, else one. e2e: two tabs, no keypress, the shim log holds both queries;
   with `background_tabs = false`, the first only.
5. **Wrap-up.** `src/app/AGENTS.md` (the "Only the active tab is fetched" sentence and the
   coalescing bullet), README check, the phase outcome in `PLAN.md`,
   `changes/phase-44-background-tabs.md` (`kind: feat`).

## Verification
`scripts/gates.sh`. Then `conveyor-dev` with the two real tabs: start it, wait for the first
load, press `Tab` and see `review` already filled; the call log shows both searches per
interval. Set `background_tabs = false` and see `review` empty until the switch.

## Left out on purpose
- **Merge queue tabs in the background.** A queue fetch is a GraphQL call plus the
  merge-group runs over REST, and `jobs_needed` deliberately ignores hidden queue tabs
  (aging them refetched every run twice). It also has no config table of its own to hold the
  key. A later phase if two queues are watched at once.
- **Counts or a "new" dot per tab in the bar.** Now possible, since hidden tabs hold fresh
  rows; a visible change with its own design and screenshot.
- **Per-tab opt-out or interval.** One key for the column until a tab is expensive enough to
  need its own.

## Outcome

Phase 44 (`plans/phase-44-background-tabs.md`, #100): every My PRs tab loads at start and
refreshes every `refresh_secs`, active or not, so a tab shows fresh rows on the first switch.
`[prs] background_tabs` (default `true`) turns it off, back to the active tab only.
`refresh::spawn` takes a sweep size: My PRs passes its tab count, the other columns one. A
request fetches its tab only, is fetched before the tabs a running sweep has left, and leads
the next sweep without moving its deadline; a sweep of one restarts the interval after a
request, as before. Review round 1: the scheduling moved into `refresh::Schedule`, handed
the current `Instant`, so the deadline tests no longer sleep (two timing tests could flake on
CI); requests that kept arriving (a held `r`) starved the rest of a sweep and an overdue
deadline, so after a request the sweep's next tab goes first and an overdue request starts
the sweep. The price, accepted: with three or more tabs, a second request that arrives while
the first is fetched waits for one background fetch. Review round 2 changed only docs: a tab
costs one fetch per sweep, which can be more than one call when checks are paged. Deviations: the plan said the `config-v0.toml` test stays unedited; the
fixture is, but its expected output gains `background_tabs = true`, as it gained the
`[ui.keys]` defaults in phase 40. `tests/e2e.rs` became `tests/e2e/` (`main.rs` plus
`background_tabs.rs`) to stay under 700 lines. `refresh::wait` now returns `Wake::{Asked,
Due, Closed}`, since the loop must tell a request from the deadline.

## Hurdles

- `test-seam` Deadline tests slept on the real clock and two could flake on CI; the schedule now takes the current `Instant` → code
- `review-loop` A held `r` starved the rest of a sweep and an overdue deadline → code
- `test-seam` `tests/e2e.rs` passed 700 lines and became a folder → rule in `AGENTS.md`
