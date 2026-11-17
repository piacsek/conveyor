# Phase 68: a config value that would break conveyor or be ignored is refused at load

## Context
Issue #122. `deny_unknown_fields` catches a misspelled key and `config::load` checks tab names,
titles, branches and the tab keys, but nothing checks the values, or whether a setting is read
at all. `conveyor config` printed each of the configs below without complaint, and the TUI then
polls GitHub every second, hides its columns, fails per column at runtime, or drops a setting
without a word. Phases 60 and 65 are reserved, 64, 66 and 67 run in parallel, so this is 68.

## Verified facts (2026-11-17, at `5dd801f`)
- Every config in the table below was written to a file and `conveyor config` (a debug build
  of `5dd801f`) exited 0 on each, printing it back.
- `refresh::Schedule::new` clamps an interval to `max(1 s)`, so `refresh_secs = 0` polls every
  second.
- `[prs] limit` and a tab's `limit` go to the `first:` of the `search` GraphQL query
  (`src/sources/fetch.rs:29`). Asked live: `first: 101` answers `EXCESSIVE_PAGINATION`
  ("exceeds the `first` limit of 100 records"); `first: 0` answers no nodes.
- `[[repo]] builds` is the `per_page` of the runs REST call (`src/sources/fetch.rs:357`). Asked
  live: `per_page=0` answers 30 runs (GitHub's default), `per_page=101` answers 100.
- `details_percent` is a `Constraint::Percentage` of the body below the columns
  (`src/ui/mod.rs:53`); at 100 or more the columns get no rows.
- `min_column_width * cells(4)` is `u16` arithmetic (`src/ui/mod.rs:63`): 16384 or more
  overflows (a debug panic, a release wrap).
- `fetch_queue` splits the repository name on `/` and fails per column with
  `repo ... must be owner/name`.
- `DeployEnv` fields default to `""`, so an env table that leaves out `context`, `namespace`,
  `deployment` or `name` loads. An empty `context` or `namespace` is kubectl's current one; an
  empty `deployment` hands kubectl an empty name, and an empty `name` leaves the row unnamed.
- `spawn_deploy_fetcher` (`src/main.rs`) reads the first `[[repo]]` with a deploy and only its
  first `[[repo.deploy]]`. `spawn_queue_fetcher` and `spawn_builds_fetcher` take their interval
  from the first `[[repo]]`. Two `[[repo]]` of one name and two envs of one name load; a
  Deployed row's key is a hash of its env name.
- `tab_bar` (`src/ui/columns.rs`) computes `width - used - 1` with `width` the column's inner
  width, which is 0 in a column 2 cells wide or less: a debug build panics there.
- The demo config (`src/sources/demo/config.toml`) has one deploy on its first repository,
  distinct env names and every env field set; the defaults are 60, 30 and 120 s, limit 20,
  builds 10, details 40 %, min width 36.

## Design
- One validation pass, `src/config/values.rs`, called from `config::load` after the existing
  checks; its message goes through the same `ConfigError` (`<file>: <message>`), so
  `conveyor config` exits 1 and the TUI starts on the defaults with the notice, as for an
  unknown key. `--help` and `--version` are untouched: they return before the config is read.
- No clamping: every value outside its range is refused, naming the table, the key, the
  repository or env where there is one, the value and the range.
- A value is refused only when something named breaks with it: a panic, an overflow, a call
  GitHub refuses or answers with another page size, the rate limit, a row collision, or a
  setting conveyor never reads. A bound that is only tidy is not one (review round 2).
- `refresh_secs` floor of 5 s, the owner's decision of 2026-11-17 (review round 3; the issue
  suggested 10): at 5 s My PRs plus the Merge queue are estimated at roughly 1,400 to 3,000
  GraphQL points an hour, under the 5,000 limit, while 0 and 1 are shown to exhaust it. One
  floor for all three tables keeps the rule one sentence.
- `details_percent` 1 to 99: 0 leaves an empty pane, 100 and above leave the columns no rows;
  5 or 95 is cramped but draws both.
- `limit` and `builds` 1 to 100: GraphQL answers `first: 0` with no rows and `first: 101` with
  EXCESSIVE_PAGINATION; REST answers `per_page=0` with 30 runs and `per_page=101` with 100.
- `min_column_width` 0 to 16383: 0 (always four columns) and a width wider than any terminal
  (always the column bar) are both choices that work; only the overflow is broken, and 16383
  (`u16::MAX / 4`) keeps `width * 4` inside `u16`, so the multiplication at `src/ui/mod.rs:63`
  stays as it is.
- `refresh_secs` has no ceiling: `u64::MAX` overflowed `Instant` in `Schedule::fetched` and
  killed that column's fetcher after its first fetch, so the deadline is now a `checked_add`
  and one past the end of `Instant` is never due (a refresh key still fetches).
- Two `[[repo]]` of one name are refused only when their `branch` and `main_workflow` match
  too: since phase 66 each is its own tab, so one repository on two branches is a working
  config, while a pure repeat shows the same tab twice.
- A `[[repo.deploy]]` with no `[[repo.deploy.env]]` is refused: conveyor reads nothing from it
  and the Deployed column stays empty without an error.
- Kept, with what breaks: a blank `deployment` makes `kubectl get deploy ""` list every
  deployment of the namespace; a blank env `name` draws a card with no title and collides with
  any other blank one; two envs of one name share one row key, so one selection and one mark;
  a space or control character in a repository name fails every GitHub call and reaches the
  terminal raw.
- A blank `context` or `namespace` stays allowed: `kubectl --context "" -n ""` uses the current
  context and its namespace (checked against a scratch kubeconfig), so leaving them out works.
- Ignored settings are refused with what conveyor reads instead. Since phase 66 (#114) every
  `[[repo]]`'s `builds`, `main_workflow`, `branch` and `refresh_secs` is read by Main builds,
  so each repository is held to the ranges. The Merge queue still refreshes every tab on the
  first repository's `refresh_secs`; that is not refused (the value is read, for Main builds)
  and went to the backlog. The Deployed column reads one `[[repo.deploy]]`.

## Settings, ranges and errors

`<file>` is the config path. Every row was reproduced as accepted on `5dd801f`.

| Setting | Accepted | Refused with `<file>: …` |
| --- | --- | --- |
| `[prs] refresh_secs` | 5 or more | `[prs] refresh_secs is 4; use 5 or more` |
| `[[repo]] refresh_secs` | 5 or more | `[[repo]] refresh_secs of acme/webapp is 4; use 5 or more` |
| `[[repo.deploy]] refresh_secs` | 5 or more | `[[repo.deploy]] refresh_secs of acme/webapp is 4; use 5 or more` |
| `[prs] limit` | 1 to 100, checked only when no `[[prs.tab]]` or one without `limit` falls back to it | `[prs] limit is 0; use 1 to 100` |
| `[[prs.tab]] limit` | 1 to 100 | `[[prs.tab]] limit of "mine" is 500; use 1 to 100` |
| `[[repo]] builds` | 1 to 100 | `[[repo]] builds of acme/webapp is 0; use 1 to 100` |
| `[ui] details_percent` | 1 to 99 | `[ui] details_percent is 100; use 1 to 99` |
| `[ui] min_column_width` | 0 to 16383 | `[ui] min_column_width is 16384; use 16383 or less` |
| `[[repo]] name` | `owner/name`, both parts non-empty, no spaces or control characters | `[[repo]] name "not-a-repo" is not owner/name` |
| `[[repo.deploy.env]] name` | not blank | `a [[repo.deploy.env]] of acme/webapp has a blank name` |
| `[[repo.deploy.env]] deployment` | not blank | `[[repo.deploy.env]] "prod" of acme/webapp has a blank deployment` |
| `[[repo.deploy.env]] context`, `namespace` | anything; blank means kubectl's current context and its namespace | not refused |
| two `[[repo]]` of one name (without case, as GitHub compares) on the same `branch` and `main_workflow` | refused; a different `branch` or `main_workflow` loads | `two [[repo]] named "acme/webapp" follow the same branch and main_workflow; nothing tells their tabs apart` |
| two envs of one name in a deploy | refused | `two [[repo.deploy.env]] of acme/webapp are named "prod"` |
| a `[[repo.deploy]]` with no `[[repo.deploy.env]]` | refused | `the [[repo.deploy]] of acme/webapp has no [[repo.deploy.env]]` |
| a second `[[repo.deploy]]` on a repository | refused | `acme/webapp has a second [[repo.deploy]]; conveyor reads only the first` |
| a `[[repo.deploy]]` on more than one repository (one on the second repository alone loads) | refused; one repository listed twice is told apart by position | `the [[repo.deploy]] of acme/infra is ignored: conveyor reads only the one of acme/webapp`, or `… of acme/webapp ([[repo]] number 2) is ignored: conveyor reads only the one of acme/webapp ([[repo]] number 1)` |

The drawing site: a column 2 cells wide or less draws its tab bar without panicking
(`saturating_sub` in `tab_bar`).

## Tasks
1. `refresh_secs` floor in the three tables; the demo config and the defaults load.
2. `limit`, a tab's `limit` and `builds` in 1 to 100.
3. `details_percent` and `min_column_width`.
4. `[[repo]] name` shaped `owner/name`; no blank env field.
5. Duplicate repository and env names.
6. Ignored settings: a second deploy, a deploy on more than one repository.
7. `tab_bar` in a column 2 cells wide.
8. Fragment, README and `AGENTS.md` facts.
9. Review round 2: each bound held to what breaks outside it; no refresh ceiling and a
   `checked_add` deadline; one repository on two branches loads; a deploy needs an env.
10. Review round 3: the refresh floor is 5; `[prs] limit` checked only when a tab falls back to
    it; a quoted config string is written escaped (`{:?}`), so a message stays on one line.

## Outcome
- Shipped in #137: one validation pass, `src/config/values.rs`, run by `config::load`. A value
  out of its range, a malformed name, a blank env name or deployment, a pure repeat of a
  `[[repo]]` or of an env name, and a deploy conveyor would not read are refused with
  `<file>: <message>`, so `conveyor config` exits 1 and the TUI starts on the defaults.
- The rule the review loop converged on: a value is refused only when something named breaks
  with it. It loosened `details_percent` to 1 to 99, dropped the `refresh_secs` ceiling (the
  `Instant` overflow it guarded is now a `checked_add` deadline that is never due), let one
  repository be listed again for another branch or workflow, and checks `[prs] limit` only
  when a tab falls back to it. It is written into the config bullet of `AGENTS.md`.
- The `refresh_secs` floor is 5 s, the owner's decision of 2026-11-17, for all three tables.
- Kept bounds and what breaks outside each are in the Design section; quoted names are written
  escaped so a message stays on one line.
- Found on the way: `tab_bar` underflowed in a column two cells wide or less (debug builds
  only); the Merge queue refreshes every tab on the first repository's interval (backlog).
- Known gaps, not refused: a repository with no `branch` and the same one with
  `branch = "main"` may follow one branch; a range error on a repository listed twice names the
  repository, not the entry; an integer above `i64::MAX` is the TOML parser's error.

## Hurdles
- `review-loop` Rounds 1 and 2 each found bounds stricter than what breaks (a refresh ceiling, `details_percent` 10 to 90, any two `[[repo]]` of one name), patched one by one until the rule "refuse only what you can name a breakage for" ended the class → rule in `AGENTS.md`
- `design` The `refresh_secs` floor was a product decision, not a code one: 10 from the issue, 5 from the owner after an estimate of the GraphQL points an hour; the round 3 fixes waited about 3 hours on it → none
- `process` A commit went out with a red test because a test file was edited while the ship's gates ran and `git add -A` took it; fixed in the next commit → none
- `ci` The macOS `flake` job was cancelled for lack of a runner and needed a rerun → none
