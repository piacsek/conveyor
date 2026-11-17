# Phase 62: Main builds reads the repository's own default branch

## Context

Issue #123. The Main builds query names `branch=main` (`BuildsSource::fetch`), and no config
key changes it. A repository whose builds run on `master`, `develop`, `trunk` or any other
branch shows an empty column with no error, and every Deployed row there reads `○`, because
`behind_main` is a sha's index in the Main builds list.

## Verified facts (2026-11-17, `main` at 6512665)

- `gh api repos/<owner>/<repo> --jq .default_branch` prints the bare branch name (`main` for
  this repository); the field is a top-level string of the repository object.
- `gh api repos/<owner>/<missing>` fails with `gh: Not Found (HTTP 404)` on stderr, which
  `Failure::said_by_gh` reads as `Other`.
- `actions/workflows/ci.yml/runs?branch=main&event=push` on this repository answers runs whose
  `head_branch` is `main`; `branch=nope` answers `total_count: 0` and no error, which is how a
  wrong branch turns into an empty column today.
- The `Demo` routes REST by path segments; `repos/acme/webapp` (three segments) has no route
  yet and fails with `demo: no fixture for …`.
- The e2e `gh` shim of `tests/e2e/main.rs` answers any call it has no route for with
  `prs.json`, which holds no `default_branch`.

## Decisions (from the issue and the orchestrator)

- An optional `branch` key under `[[repo]]`. `conveyor config` leaves it out while unset, like
  `path`.
- Unset: read `default_branch` once from `repos/{owner}/{name}` and cache it on `BuildsSource`
  next to the resolved workflow. A failed lookup is not cached, so the next fetch asks again.
- A failed lookup is the column's error, never an empty column.
- The column title stays "Main" (titles are configurable under `[ui.titles]`); nothing visible
  moves, so no screenshot changes.
- A blank `branch` is refused at load, naming the repository, like a blank title.
- The branch is percent-encoded in the query, so a name holding `+`, `&` or `#` reaches GitHub
  as written.
- The demo answers the lookup with `main`; the e2e shim answers it with `main`.

## States

| Branch configured | Lookup | Column shows |
| --- | --- | --- |
| yes | not made | the runs of the configured branch |
| no | answers `default_branch` | the runs of that branch; the lookup is not repeated |
| no | fails (`gh` error, 404, no access) | the column error with `gh`'s message; the next fetch asks again |
| no | answers without a `default_branch` string | the column error naming the repository |

## Tasks

1. Plan, `PLAN.md` row, `plans/metrics.md` row.
2. A configured `branch` names the branch of the Main builds query; `conveyor config` leaves it
   out while unset.
3. Unset: the query names the repository's default branch, read once; a failed lookup is the
   column's error. The demo and the e2e shim answer the lookup.
4. A blank `branch` is refused at load.
5. The branch is percent-encoded in the query.
6. README default config, `src/sources/AGENTS.md`, the fragment.

## Outcome

## Hurdles
