# Phase 42: tabs in the Merge queue column, one per repository

## Context
The user watches two repositories with a merge queue each: an application monorepo and an
infra repo. Today conveyor reads only the first `[[repo]]` for every column, so the infra
queue cannot be seen at all.

Decided with the user on 2026-11-14:
- **Merge queue only, this phase.** One tab per `[[repo]]`, in config order. Main builds and
  Deployed are not touched and keep reading the first `[[repo]]` (Deployed: the first with a
  `[[repo.deploy]]`), as today.
- **Columns switch independently.** Switching the queue tab never moves another column.
- **Where this goes next (not this phase):** Main builds gets the same tabs as Merge queue and
  follows the queue's active tab. Deployed gets its own tabs, one per application of the
  monorepo, switched on their own. Nothing is built for either now (no dead code); this phase
  only avoids choices that would block them (see "Left out on purpose").
- **Same behaviour as My PRs tabs (phase 40):** only the active tab fetches, each tab keeps its
  rows and selection, a switch clears the filter and starts a refresh, one tab draws no bar,
  `Tab`/`Shift-Tab` (as configured in `[ui.keys]`) and `1`-`9` switch.

## Compatibility: today's config keeps working
No config key is added, renamed or made required. A config with no `[[repo]]` or one `[[repo]]`
shows exactly what it shows today. A config with several `[[repo]]` loads as before; what
changes is that the Merge queue column now shows a bar and can reach the second and later
repositories. `tests/fixtures/config-v0.toml` and its `tests/cli.rs` test stay green, unedited.

## Design

```toml
[[repo]]
name = "acme/monorepo"
main_workflow = "CI/CD"

[[repo.deploy]]
system = "web"

[[repo]]
name = "acme/infra"
```

```text
┌Queue monorepo (2)───────────────┐
│ monorepo │ infra                │
│▌ ●  #12 Retry webhooks       2h │
```

- **Tabs.** `App::new` names one queue tab per `config.repo` entry, labelled with the short
  repo name (`short_repo`, the part after `/`). No `[[repo]]` is one unnamed tab, fetched from
  the current directory's repository as today.
- **Fetching.** The queue fetcher fetches `config.repo[tab]`. With no `[[repo]]`, tab 0 is the
  current directory's repository, resolved once as today. The interval stays the first
  `[[repo]]`'s `refresh_secs`, as today.
- **Each tab is its own column state.** `QueueStage` holds one `Column<QueueEntry>` and one
  `repo` per tab, plus the active index, like `PrsStage`. It overrides `listing_in_mut`,
  `holds`, `tab_names`, `active_tab` and `select_tab`.
- **Title.** `Queue <repo>` names the active tab's repository, so the narrow layout and a
  custom `[ui.titles].queue` keep working unchanged.
- **Refresh after a write.** A merge or status change refreshes the queue's active tab, as
  `refresh_request(Stage::Queue)` does today. A tab that is not active refreshes when you
  switch to it.
- **Order matters for the other columns.** Main builds reads the first `[[repo]]`; put the
  monorepo first. Written in `src/app/AGENTS.md` and in the README's config block comment.

### States and what the column shows (review before any code)

| State | Bar | Cards | Title |
| --- | --- | --- | --- |
| No `[[repo]]` | none | current directory's queue, as today | `Queue <repo> (N)` |
| One `[[repo]]` | none | as today | `Queue <repo> (N)` |
| Two `[[repo]]`, first load | first tab highlighted | first repo's queue once fetched; infra never fetched | `Merge queue` until the first fetch |
| Switch to a tab never fetched | that tab highlighted | empty, spinner in the footer; a refresh for that tab starts | `Merge queue` until it lands, then `Queue infra (N)` |
| Switch back to a tab fetched before | highlighted | its last rows and selection, no filter; a refresh starts | its repo and count |
| Result lands for an inactive tab (switched mid-fetch) | unchanged | unchanged; stored in its own tab | unchanged |
| Error lands for an inactive tab | unchanged | unchanged; shown when you switch to it | unchanged |
| Active tab's repo has no merge queue | as above | empty, as a repo without a queue shows today | `Queue <repo> (0)` |
| Merge of #12 into infra from My PRs while monorepo is the active queue tab | unchanged | monorepo tab refreshes (no change); infra refreshes when you switch to it | unchanged |
| Same number #12 queued in both repos | - | known limit: the row key is the number, so a mark or spinner shows on both | - |
| Details pane open, user switches tab | - | pane shows the new tab's selected entry, scroll reset | - |
| Jobs and `L` log of a merge-group run on the infra tab | - | asked of the run's own repository (review round 1: they went to the first `[[repo]]`) | - |
| `Tab` on Main builds or Deployed | - | nothing happens, as today | - |
| Digit past the last tab, or the active tab's digit | - | nothing happens, no refresh | - |

## Verified facts (read on `main` at 8841900, 2026-11-14)
- The tab machinery from phase 40 is generic: `StageView` has `tab_names`, `active_tab`,
  `select_tab`, `listing_in_mut(tab)` and `holds` with one-tab defaults
  (`src/app/stages/mod.rs:111-160`); `switch_tab` in `src/app/navigation.rs:26` works on the
  focused stage; `tab_bar` in `src/ui/columns.rs:219` draws for any stage with two names.
- `refresh::spawn` already hands the fetch closure a tab index and tags results with it.
  `spawn_queue_fetcher` (`src/main.rs:344`) ignores it (`move |_|`) and takes
  `repos(&gh, &config)?.into_iter().next()`.
- `repos()` (`src/sources/fetch.rs:127`) returns `config.repo` or, when empty, the current
  directory's repository from `gh`.
- `QueueStage { column, repo }` (`src/app/stages/queue.rs`, 57 lines) is built with
  `QueueStage::default()` (`src/app/mod.rs:132`). Outside the stage, `app.queue.column` is read
  in `src/ui/columns.rs:49,150` and `src/ui/details.rs:28`, and `app.queue.repo` in
  `src/ui/columns.rs:47`. Those go through a `column()` accessor for the active tab.
- After a merge or status change, `run.rs:60,67` request `refresh_request(Stage::Queue)`,
  which names the active tab (`src/app/navigation.rs:47`).
- `PrsStage` (`src/app/stages/prs.rs`) is the pattern to copy: `names`, `tabs`, `active`.
- `tests/queue.rs` is 463 lines; queue tab tests go in a new `tests/queue_tabs.rs` (or turn
  `tests/queue.rs` into a folder if it fits better), staying under 700.

## Tasks (each starts red, each shipped with `scripts/ship.sh`)
1. **Two repos draw two queue tabs, and `Tab` switches.** TUI test: two `[[repo]]`, focus
   Merge queue, the bar shows `monorepo │ infra`; `Tab` shows the infra tab and sends a
   refresh for tab 1; `Shift-Tab` returns with the selection intact and the filter cleared.
   One `[[repo]]` and none draw no bar (existing snapshots unchanged). `QueueStage::new(names)`
   with per-tab columns.
2. **The fetcher fetches the tab's repository.** Extract the repo choice from
   `spawn_queue_fetcher` into `fetch.rs` (`queue_repo(gh, config, tab)`); unit tests through
   the `Github` fake: tab 1 gives the second `[[repo]]`, no `[[repo]]` gives the current
   directory's, a tab past the list is an error naming it.
3. **A late result lands in its own tab, with its own repo.** Switch mid-fetch: tab 0's
   result and error stay in tab 0; the title shows the active tab's repo, not the last one
   fetched.
4. **The bar in the column.** Snapshot `queue_tabs_layout`: two repos, the second active,
   wide and narrow layouts.
5. **Wrap-up.** `src/app/AGENTS.md` (queue tabs, Main builds and Deployed read the first
   `[[repo]]`), `src/ui/AGENTS.md` if the bar section names My PRs only, README default config
   if it needs the second `[[repo]]` example, `scripts/screenshots.sh` with a second repo so
   `docs/details.png` shows the queue bar, the phase outcome in `PLAN.md`, the backlog entry
   rewritten to the two future steps below, and `changes/phase-42-queue-tabs.md`
   (`kind: feat`).

## Verification
`scripts/gates.sh`. Then `conveyor-dev` with the two real repositories configured: the queue
bar shows both; `Tab` switches and the call log shows only the active repo's queue query on
each interval; Main builds and Deployed stay on the monorepo throughout.

## Left out on purpose
- **Main builds tabs, synced to the queue.** Next phase. Tab names come from `config.repo`
  in one place, so Builds can reuse the list; the sync rule (Builds follows the queue's active
  tab) is designed then.
- **Deployed tabs per application.** A later phase, with its own config shape.
- **Per-tab `refresh_secs`.** The queue fetcher keeps one interval, the first `[[repo]]`'s.
