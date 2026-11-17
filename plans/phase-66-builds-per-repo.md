# Phase 66: Main builds follows the Merge queue's repository

## Context

Issue #114. With several `[[repo]]` tables the Merge queue has one tab per repository, but
Main builds reads only the first one: `spawn_builds_fetcher` in `src/main.rs` takes
`repos(..).next()`, and `BuildsStage` holds one column. Builds of the other repositories never
appear. The backlog entry in `PLAN.md` records the decision taken with the user on 2026-11-14:
Main builds follows the Merge queue's active tab.

## Verified facts (2026-11-17, `main` at 5dd801f)

- `spawn_builds_fetcher` calls `refresh::spawn(Stage::Builds, interval, 1, …)` with the first
  `[[repo]]`'s `refresh_secs` (30 with none) and ignores the tab it is handed.
- `spawn_queue_fetcher` sweeps one tab: it fetches only the tab last asked for, on the first
  `[[repo]]`'s `refresh_secs`. `App::refresh_request` asks for a stage's `active_tab()`.
- `QueueStage` holds one `Column` and `repo` per tab, `QueueStage::new` takes the short names
  from `repo_tabs` (one unnamed tab with no `[[repo]]`). `App::switch_tab` calls `select_tab`
  on the focused stage only and sends one `Refresh` for it.
- `BuildsStage` has public `column`, `repo` and `branch`, read by `ui/columns.rs` (title, empty
  body, links, the Deployed card's `at <branch>`), `ui/details.rs`, `Context::main_build`,
  `Context::behind_main` and `Context::deploy_repo`, and `tests/github/kinds.rs`.
- The Deployed column watches the first `[[repo]]` with a `[[repo.deploy]]`, and reads Main
  builds for `behind_main` (`↓N`), `at <branch>` and `b`/`Y` on a deployed sha.
- `age_jobs` and `jobs_needed` read `StageView::runs()`; the queue answers its active tab's.
- `any_refreshing` reads `listing()` of each stage, the active tab, so a hidden tab's fetch does
  not spin the footer.
- The demo has two `[[repo]]`: `acme/webapp` (with the deploy) and `acme/infra`; every runs
  route answers `runs.json`, whose runs are all `acme/webapp` URLs.

## Decisions

- Main builds has one tab per `[[repo]]`, in config order, like the queue, and no tab bar of its
  own: its active tab is the Merge queue's active tab, whichever column switched it. The tab keys
  in Main builds do nothing (one shown tab, no bar); the title names the repository.
- One fetcher thread per repository, each on its own `refresh_secs`, every repository fetched
  in the background, so a switch shows that repository's last builds at once and the Deployed
  column keeps reading its own repository's builds whatever tab the queue shows.
  `refresh::spawn_each` starts them behind one `Sender<usize>`, so `main`'s map of refreshers
  keeps its shape.
- The Deployed column reads the builds of the deploy repository's tab (the first `[[repo]]`
  with a `[[repo.deploy]]`, else the first tab), never the active one.
- A queue switch sends no Main builds refresh: the background fetcher already keeps it current.
- The demo's `acme/infra` answers no runs, so its tab reads `no builds on main` instead of
  webapp's runs under the infra name.

## States

| State | Main builds shows |
| --- | --- |
| one or no `[[repo]]` | as before: one tab, the first repository, no tab bar |
| the active tab has not answered yet | `fetching…`, title `Main builds` (no repository known yet) |
| the active tab answered | that repository's builds, title `Main <short repo>` |
| the active tab answered no runs | `no builds on <that repository's branch>` |
| the active tab's first fetch failed | the error in the body, `⚠` in the title |
| the active tab failed after it had rows | its last rows, `⚠` in the title |
| a hidden tab answers or fails | lands in that tab only; nothing on screen moves, the footer does not spin |
| the queue switches tab | Main builds shows that tab's rows, selection and error as last left |
| a tab key pressed with Main builds focused | nothing, nothing sent |
| `r` on Main builds, `R` | a refresh of the active tab's repository |
| the deploy repository is not the active tab | Deployed's `↓N`, `at <branch>`, `b`/`Y` read the deploy repository's builds |
| a rerun mark on a hidden tab's build | kept (`holds` asks every tab); the title's `!N` counts the active tab |
| jobs | asked for the active tab's runs only, like the queue |

## Tasks

1. Plan, `PLAN.md` row, `plans/metrics.md` row.
2. A builds result lands in its own tab, and Main builds shows the tab the Merge queue shows.
3. The Deployed column reads the deploy repository's builds, whatever tab is active.
4. `refresh::spawn_each`: one fetcher per tab, on its own interval, behind one sender; `main`
   starts one Main builds fetcher per repository.
5. The demo's `acme/infra` answers no runs.
6. `src/app/AGENTS.md`, `src/ui/AGENTS.md`, the backlog item, the fragment, the screenshot check.

## Outcome

## Hurdles
