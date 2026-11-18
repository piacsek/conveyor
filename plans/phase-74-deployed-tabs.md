# Phase 74: the Deployed column has tabs, one per deployed system

## Context

Deployed shows one system: the first `[[repo.deploy]]` of the first `[[repo]]` that has one. A
second `[[repo.deploy]]` on the same repository, or one on another repository, stops the config
from loading. A monorepo that deploys several applications, or a user who follows two
repositories that each deploy, sees only one of them. The backlog line is "Deployed tabs, one
per application of the monorepo, switched independently of the queue".

## Verified facts (2026-11-18, at `f1440c8`)

- `Repo::deploy` is already a `Vec<Deploy>` (`src/config.rs`), so `[[repo.deploy]]` repeats in
  TOML today; only `src/config/values.rs` refuses it: `check_read` when two repositories have a
  deploy, and `repo.deploy.len() > 1` for a second one on one repository.
- `spawn_deploy_fetcher` (`src/main.rs`) takes the first deploy of the first repository that has
  one and starts one fetcher through `refresh::spawn(Stage::Deployed, interval, 1, ...)`.
  `DeploySource::new(repo.name, deploy)` already takes the repository and the deploy as values.
- `DeployedStage` (`src/app/stages/deployed.rs`) holds one `Column<Deployment>` and one
  `system`, ignores the tab of `receive`, and inherits the one-tab defaults of `StageView`
  (`active_tab` 0, a no-op `select_tab`, `listing_in_mut` for tab 0 only).
- `Stage::tab_group` already returns `[Stage::Deployed]` for Deployed, so the tab keys
  (`next_tab`, `previous_tab`, `1` to `9`) in that column reach `select_tab` on Deployed alone.
- Main builds is the model to copy: `BuildsStage` holds one `BuildsTab` per `[[repo]]`,
  `refresh::spawn_each` runs one fetcher per tab in the background, each on its own
  `refresh_secs`, `r` asks for the active tab, and a switch clears the filter of the tab left
  and the tab shown with `Column::clear_filter`.
- `Context::deploy_builds`, `main_build`, `behind_main` and `deploy_repo`
  (`src/app/stages/mod.rs`) read the builds tab of the first repository with a deploy. With
  tabs they must read the builds tab of the shown deploy's repository.
- The column title is `named(app, stage, "Deployed", app.deployed.system)` in
  `src/ui/columns.rs`; Main builds draws its tab names through `tab_name`.
- The demo config (`src/sources/demo/config.toml`) has one deploy, `api`, on its first
  repository, and `acme/infra` and `acme/docs` without one.
- The sweep of phase 70 added the rule "anything that belongs to a tab names it" to
  `src/app/AGENTS.md`, and the backlog item that makes the one-tab `StageView` defaults required
  methods. This phase is the first stage to gain tabs since, so it follows the rule from the
  start and does not inherit the defaults.

## Decided by the owner on 2026-11-18

1. **What a tab is.** One tab per `[[repo.deploy]]` of every `[[repo]]`, in config order. That
   covers a monorepo with several applications and two repositories that each deploy, and
   removes both load errors.
2. **What is fetched.** Every tab in the background on its own `refresh_secs` (default 120 s),
   like Main builds, so a switch shows rows at once. Cost: one `kubectl` call per environment of
   every tab per interval, instead of only the shown tab's.
3. **The tab's name.** Its `system`. Two deploys with the same `system` (case ignored) are
   refused at load, as are a blank one once there is more than one deploy. A single deploy with
   no `system` stays valid, as today.
4. **Switching.** Independent of Merge queue and Main builds: switching the Deployed tab never
   moves the queue's repository, and the reverse.

## Design

- **Config.** `values.rs` stops refusing a second deploy and a second deploying repository, and
  refuses repeated or blank `system` names among several deploys. `Config` gains one function
  that lists the deploys in tab order, each with its repository name: the one place that says
  which tab is which, used by `main.rs`, the stage and `Context`.
- **Fetching.** `spawn_deploy_fetcher` becomes `refresh::spawn_each` over that list, one
  `DeploySource` per tab, each on its deploy's `refresh_secs`. No deploy configured starts no
  fetcher, as today.
- **Stage.** `DeployedStage` holds one tab per deploy (`column`, `system`, `repo`) and an active
  index, and implements `active_tab`, `select_tab`, `listing_in_mut` and `receive(tab, ...)`
  itself, so a result lands in the tab it was fetched for whatever is shown. `keep_last_known`
  runs against the rows of that tab. A switch clears the filter of the tab left and the tab
  shown with `clear_filter`.
- **Context.** `deploy_builds` reads the builds tab of the `[[repo]]` entry that holds the shown
  tab's deploy, so "behind main", the build link and the commit link follow the shown system. A
  repository listed twice (one entry per branch) reads the builds of the entry the deploy sits
  on, as today (the code read the first entry with a deploy, not the first entry of that name).
- **UI.** The Deployed title draws the tab names the way Main builds does, the active one
  marked; one tab draws exactly today's title. The help view already words the tab keys per
  column. `README.md`: the `[[repo.deploy]]` comment loses "on one `[[repo]]` at most".
- **Demo and docs.** The demo config gains a second deploy (`worker` on `acme/webapp`) and its
  fixture rows, so `docs/details.png` shows the tabs; `docs/demo.gif` is rerecorded if the
  first frame changes. `src/app/AGENTS.md`, `src/sources/AGENTS.md` and the root layout line
  for `stages/deployed.rs` follow.
- **Fragment.** `changes/phase-74-deployed-tabs.md`, `kind: feat`.

### States

| State | What Deployed shows |
| --- | --- |
| no `[[repo.deploy]]` anywhere | "no [[repo.deploy]] configured", no tabs, tab keys do nothing |
| one deploy | today's column and title, tab keys do nothing |
| several deploys, shown tab not fetched yet | its tab name marked, `fetching…` |
| shown tab fetched, another still loading | the shown tab's rows; the other tab's name is not marked as failed |
| a result for a tab that is not shown | stored in that tab; the shown rows, selection and filter do not move |
| a tab's fetch fails | the failure on that tab only; its last rows stay, like any last rows |
| one environment of a tab fails | that row's own error, the others as fetched (today's behaviour, per tab) |
| switch with a filter typed | the filter of the tab left and of the tab shown is cleared, each keeps its selected row |
| switch with the details pane open | the pane follows the shown tab's selected row |
| the shown tab's repository has no builds yet | rows without "behind main"; the link falls back to the commit page |
| two deploys of repositories that differ | each tab reads the builds of its own repository's tab |
| `r` in Deployed | refreshes the shown tab only; `R` refreshes the shown tab, as in every column (the other tabs keep their own interval) |
| a tenth deploy | reachable with `next_tab`/`previous_tab`; `1` to `9` reach the first nine |
| `--demo` | two tabs from the fixture, nothing spawned |

## Tasks

1. Config: several deploys load; repeated and blank `system` names are refused; the tab list.
2. Stage: tabs, `select_tab`, `receive` and `listing_in_mut` by tab, filter cleared on a switch,
   with a test that switches tabs between a request and its landing.
3. Fetching: one fetcher per deploy; `r` asks for the shown tab.
4. Context: builds, "behind main" and links follow the shown tab's repository.
5. UI: tab names in the title; the narrow layout; `config` output.
6. Demo fixture, screenshots, README comment, `AGENTS.md` files, fragment.

## Outcome

- Deployed holds one `DeployedTab` (`column`, `system`) per `[[repo.deploy]]` of every
  `[[repo]]`, in `Config::deploys` order, named by its `system`, and outside any tab group.
  One fetcher per deploy runs on that deploy's `refresh_secs`; a result, a failure and
  `keep_last_known` land in the tab they were fetched for (`listing_in_mut`).
- `Context::deploy` is the `[[repo]]` of the shown tab, found by identity
  (`std::ptr::eq`), so `↓N`, `b` and `Y` follow the shown tab, and a repeated `[[repo]]` of one
  name reads its own builds tab. The doc comment of `Context::deploy_repo` went wrong with the
  change and was removed.
- Several deploys load; a repeated `system` (trimmed, case ignored) is refused, and so is a
  blank one among several. The one-deploy config, blank system included, loads and draws as
  before (no bar).
- The title names the shown tab's configured `system` before its first fetch, like Main
  builds names its repository.
- The demo deploys `worker` (staging, prod) on `acme/webapp` beside `api`; its fixtures are
  keyed by deployment, then environment. `docs/worker.png` shows the worker tab;
  `docs/demo.gif` presses `Tab` in Deployed.
- `R` refreshes the shown tab of every stage, not every Deployed tab: the plan's first wording
  was wrong and was corrected before code.

## Hurdles

- `test-seam` The filter test first passed with the clear before the switch mutated away, since `switch_tab` clears the shown listing anyway; it now reads the tab left right after the switch → none
- `test-seam` The second ship failed on `tests/kube.rs`, which read `.deployed` and `.column` on two lines that the rename grep missed → none
- `test-seam` The e2e test of the background fetch passed with the hidden tab's background fetch mutated away, since the switch itself asks for a fetch; it now waits for the hidden deploy's kubectl call in the call log before the switch (review round 1, minor) → code
- `test-seam` `DeployedStage::tab`, a `pub` item only tests called, slipped past `tests/dead_code.rs` because `BuildsStage::tab` shares its name; the tests now read through `listing_in_mut` and it is gone (review round 1, minor) → none
- `process` `config_refuses_two_deploys_of_one_system` in `tests/cli.rs` was written after the refusal, so it was never seen red; the `tests/config_values.rs` tests of the same refusal were → none
