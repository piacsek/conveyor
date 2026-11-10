# Phase 32: the environments are read at the same time

## Context

Phase 31 capped a kubectl that hangs at 15 s. The Deployed column still asks its environments
one after another, so while one is unreachable the healthy rows wait behind it: about 15 s when
its Deployment call hangs, about 30 s when only its pods call does, and that again for every
other environment that is down.

## Verified facts

- `DeploySource::fetch` maps `fetch_env` over `deploy.env` in order; `fetch_env` calls
  `kube.deployment`, then `Pulls::for_sha` (through `gh`, with a cache behind `&mut self`),
  then `kube.pods`.
- `CliKubectl` holds a path, a `Log` and a `Duration`; each call spawns its own process, so
  nothing is shared between two calls but the log.
- The column receives all its rows in one `Rows::Deployed`; nothing in `App` takes one row.
- In the session that started this, a healthy environment answered both calls in about 1 s
  and four environments took about 4 s in a row.

## Design

1. **kubectl in parallel, GitHub after.** `fetch` reads every environment on its own scoped
   thread (`std::thread::scope`): the Deployment call, then the pods call, into a `Read` that
   holds the two results. `Kube` implementations must be `Sync` for that. When every thread
   is back, the rows are built on the fetcher thread in config order, which is where
   `Pulls::for_sha` runs: the pull cache and `gh` stay single-threaded, and the row order
   cannot depend on which cluster answered first.
2. **A thread that panics is one row's error**, not the column's.
3. The column now takes as long as its slowest environment: about 1 s when all are healthy,
   about 15 s (30 s when only a pods call hangs) however many are down.

## Out of scope

- Rows landing one at a time. Healthy rows still wait for the slowest environment; changing
  that means a column that accepts partial data.

## Tasks

1. The parallel read.
2. `AGENTS.md`, `PLAN.md`, v0.23.2 and its changelog entry.
