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
- `refresh_secs` floor of 10 s, the issue's suggestion: at 5 s two My PRs tabs alone are 1,440
  searches an hour, each costing more than one point of the 5,000-point budget once the check
  contexts are counted. One floor for all three tables keeps the rule one sentence.
- `details_percent` 10 to 90, the issue's suggestion: 0 leaves an empty pane, 100 and above
  hide the columns.
- `min_column_width` 0 to 16383: 0 (always four columns) and a width wider than any terminal
  (always the column bar) are both choices that work; only the overflow is broken, and 16383
  (`u16::MAX / 4`) keeps `width * 4` inside `u16`, so the multiplication at `src/ui/mod.rs:63`
  stays as it is.
- `refresh_secs` at most 86400 (a day): `u64::MAX` overflowed `Instant` in `Schedule::fetched`
  and killed that column's fetcher after its first fetch.
- A blank `context` or `namespace` stays allowed: `kubectl --context "" -n ""` uses the current
  context and its namespace (checked against a scratch kubeconfig), so leaving them out works.
- Ignored settings are refused with what conveyor reads instead. Main builds reading only the
  first repository is #114 (phase 66), so a second repository's `builds`, `main_workflow`,
  `branch` and `refresh_secs` are left to it (`tests/config.rs` already sets a later
  repository's `refresh_secs`, which #114 may start reading).

## Settings, ranges and errors

`<file>` is the config path. Every row was reproduced as accepted on `5dd801f`.

| Setting | Accepted | Refused with `<file>: …` |
| --- | --- | --- |
| `[prs] refresh_secs` | 10 to 86400 | `[prs] refresh_secs is 0; use 10 to 86400` |
| `[[repo]] refresh_secs` | 10 to 86400 | `[[repo]] refresh_secs of acme/webapp is 0; use 10 to 86400` |
| `[[repo.deploy]] refresh_secs` | 10 to 86400 | `[[repo.deploy]] refresh_secs of acme/webapp is 0; use 10 to 86400` |
| `[prs] limit` | 1 to 100 | `[prs] limit is 0; use 1 to 100` |
| `[[prs.tab]] limit` | 1 to 100 | `[[prs.tab]] limit of "mine" is 500; use 1 to 100` |
| `[[repo]] builds` | 1 to 100 | `[[repo]] builds of acme/webapp is 0; use 1 to 100` |
| `[ui] details_percent` | 10 to 90 | `[ui] details_percent is 500; use 10 to 90` |
| `[ui] min_column_width` | 0 to 16383 | `[ui] min_column_width is 16384; use 16383 or less` |
| `[[repo]] name` | `owner/name`, both parts non-empty, no spaces or control characters | `[[repo]] name "not-a-repo" is not owner/name` |
| `[[repo.deploy.env]] name` | not blank | `a [[repo.deploy.env]] of acme/webapp has a blank name` |
| `[[repo.deploy.env]] deployment` | not blank | `[[repo.deploy.env]] "prod" of acme/webapp has a blank deployment` |
| `[[repo.deploy.env]] context`, `namespace` | anything; blank means kubectl's current context and its namespace | not refused |
| two `[[repo]]` of one name, compared without case as GitHub does | refused | `two [[repo]] are named "acme/webapp"` |
| two envs of one name in a deploy | refused | `two [[repo.deploy.env]] of acme/webapp are named "prod"` |
| a second `[[repo.deploy]]` on a repository | refused | `acme/webapp has a second [[repo.deploy]]; conveyor reads only the first` |
| a `[[repo.deploy]]` on more than one repository (one on the second repository alone loads) | refused | `the [[repo.deploy]] of acme/infra is ignored: conveyor reads only the one of acme/webapp` |

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

## Outcome

## Hurdles
