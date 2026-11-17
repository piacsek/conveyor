# Phase 56: each phase's outcome, hurdles and numbers live with its plan

## Context

`PLAN.md` had grown to 976 lines: the approved plan of 2026-09-10, every phase outcome since,
the backlog, and three "Time and tokens" entries written as prose. The plans already lived in
`plans/phase-N-*.md`, but their outcomes did not, so the story of one phase was split across
two files, and what went wrong in a phase sat inside a long paragraph nobody could scan across
phases. The user asked for one file per plan, a table for time and tokens, and a record of each
iteration's learnings and hurdles that a periodic sweep can read to improve the code and the
process.

## Design

- Each `plans/phase-N-*.md` ends with `## Outcome` (moved from `PLAN.md` verbatim) and, from
  phase 40, `## Hurdles`: one bullet per hurdle, `` `<tag>` <what happened> → <where it went> ``,
  where the tag is one of `review-loop`, `ci`, `tooling`, `test-seam`, `process`, `api-shape`,
  `design`, and where it went is `code`, `backlog`, `known limit`, `none` or
  `` rule in `<path>` ``. A hurdle marked `→ none` is what a sweep looks at first.
- `plans/metrics.md` holds one row per phase from phase 53 (the first to record them); no
  backfill of guesses for earlier phases.
- `PLAN.md` becomes the index (one row per phase: number, linked plan title, release) and the
  backlog.
- The pre-phase-0 plan (context, verified facts, architecture, phases 0 to 4, verification,
  retro checkpoints, risks) moves to `plans/original-plan.md` as history, marked stale.
- Phase 7 had no plan file: `plans/phase-7-keys-and-lookup.md` holds its outcome. Releases with no
  phase (v0.12.1, v0.16.1, v0.16.2, phase 36, v0.26.1, v0.27.0, the toolchain pin) go to
  `plans/unphased.md`. Phase 43 never had an outcome; it is rebuilt from its fragment and PR.
- `tests/plans.rs` keeps the shape: every phase plan has `## Outcome`, every plan from phase 40
  has well-formed `## Hurdles`, every phase from 53 has a metrics row, and `PLAN.md` links
  every file under `plans/`.
- `AGENTS.md`: the autonomous flow, "Time and tokens", "Cadence" and "Plan before code" point
  at the new places; the next phase's first ship backfills the previous rows' ready, merged
  and released cells; every phase whose number is a multiple of 5 opens with a sweep.

## Tasks

1. `tests/plans.rs`, seen red.
2. Move the outcomes, write the hurdles of phases 40 to 55, `plans/metrics.md`, the new `PLAN.md`.
3. `AGENTS.md` (root): layout, working agreements, known limits' pointer to phase 39.

## Outcome

Shipped as described in the design, docs and tests only, so no fragment and no release. Hurdles
were written for phases 40 to 55 from their outcomes and review notes; earlier phases carry
their outcome only. Deviations from the autonomous flow: the work ran from a cloud session on
its assigned branch instead of a fresh worktree, was committed and pushed with `scripts/gates.sh`
and `git` instead of `scripts/ship.sh` (which opens a draft PR, and none was asked for yet), and
had no `/pr-review`, which this session cannot run. The draft PR, #119, was opened once the user
asked; the review is left to the owner.

## Hurdles

- `process` Phase 43 closed with no outcome in `PLAN.md`, unnoticed until the outcomes moved; `tests/plans.rs` now fails on a phase plan without `## Outcome` → rule in `AGENTS.md`
- `process` Phase 52's outcome has no "Time and tokens" entry although #116 made it a rule before #110 merged → none
- `tooling` The cloud session has no `/pr-review` skill and no Codex, so the review loop did not run → none
