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
- A blank `branch`, or one with spaces around it, is refused at load, naming the repository, like a blank title.
- The branch is percent-encoded in the query, so a name holding `+`, `&` or `#` reaches GitHub
  as written.
- The demo answers the lookup with `main`; the e2e shim answers it with `main`.

## States

| Branch configured | Lookup | Column shows |
| --- | --- | --- |
| yes | not made | the runs of the configured branch |
| no | answers `default_branch` | the runs of that branch; the lookup is not repeated |
| no | fails (`gh` error, 404, no access) | the column error: `reading the default branch of <repo>: <gh's text>; set branch under its [[repo]]`, keeping the failure's kind; the next fetch asks again |
| no | answers without a `default_branch` string | the column error naming the repository |
| yes, blank or with spaces around it | not made | conveyor refuses to start, naming the file and the repository |
| yes, a branch with no runs (misspelled, or not built yet) | not made | `no builds on <that branch>`: GitHub answers zero runs, not an error |
| no, the default branch has no runs | answers `default_branch` | `no builds on <the default branch>` |
| either, before the first fetch answered | not yet answered | `fetching…`, as before |
| either, a Deployed row at the newest build | answered | the Deployed card reads `at <the branch Main builds read>` (`at main` for `main`) |
| either, a Deployed row before Main builds answered | not yet answered | no `at …` and no `↓N`: nothing is known to be behind |

## Tasks

1. Plan, `PLAN.md` row, `plans/metrics.md` row.
2. A configured `branch` names the branch of the Main builds query; `conveyor config` leaves it
   out while unset.
3. Unset: the query names the repository's default branch, read once; a failed lookup is the
   column's error. The demo and the e2e shim answer the lookup.
4. A blank `branch`, or one with spaces around it, is refused at load.
5. The branch is percent-encoded in the query.
6. README default config, `src/sources/AGENTS.md`, the fragment.

## Outcome

- `[[repo]]` takes an optional `branch`; unset, `BuildsSource` reads `default_branch` from
  `repos/<owner>/<repo>` once, after the workflow resolves, and caches it beside the workflow.
- A failed lookup is the column error `reading the default branch of <repo>: <gh's text>; set
  branch under its [[repo]]`, keeping the failure's kind; it is not cached. An answer without a
  `default_branch` string is the column error naming the repository.
- A blank `branch`, or one with spaces around it, is refused at load, naming the file and the
  repository. `conveyor config` leaves the key out while unset (toml skips `None` by itself).
- The branch is percent-encoded in the query except `/`.
- `Builds` carries the branch it read: an empty column reads `no builds on <branch>` and a
  Deployed row at the newest build reads `at <branch>`. The column title, the help text and the
  "no main build found" messages still say main.
- The README default config shows the key commented out, so a copied config keeps the lookup.
- The demo answers the lookup with `main` and so does the e2e `gh` shim; tests that are not
  about the branch pin `branch = "main"`.
- Not covered: a misspelled branch is not caught (GitHub answers zero runs, not an error); no
  screenshot shows the empty-column text, since the demo always has builds.
- Review: three rounds, the third clean (no must, no should); two minor findings fixed after it.

## Hurdles

- `tooling` the scratchpad is shared with the sibling agents, so ship logs mixed between phases and `pgrep -f scripts/ship.sh` matched other worktrees' ships → none
- `process` `ship.sh` runs `git add -A`, so an edit made while a ship runs would be committed with it, even a red test → none
- `tooling` the text guard hook blocked a `grep` meant to find em dashes in the PR body, because the pattern held one → none
- `review-loop` the README default config showed `branch = "main"` as a live line, pinning a copied config to `main` (round 1) → code
- `review-loop` an empty Main builds column said `no builds on main` whatever branch was read (round 2) → code
- `review-loop` the Deployed card's `at main` was declined twice, then fixed once `Builds` carried the branch (round 3) → code
