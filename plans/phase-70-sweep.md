# Phase 70: the sweep of phases 61 to 69

## Context

Issue #141, opened by the `sweep` workflow once phase 69 merged. Phase 70 is a multiple of 5,
so it is the sweep (see "The sweep" in the root `AGENTS.md`), run with the `sweep` skill:
gather with `scripts/sweep-report.sh`, read every Outcome and Hurdles in its range, analyse,
propose, and apply what the owner takes. Phases 71, 72 and 73 were built beside it.

## Plan

1. Gather: the report, the Outcome and Hurdles of phases 61 to 69, the phase 60 sweep, and the
   empty metrics cells read from GitHub.
2. Analyse and propose; the owner decides each proposal.
3. Apply what was taken: the rules into their `AGENTS.md`, the backlog line and the missing
   phase 65 row into `PLAN.md`, the two issues opened, the metrics rows backfilled.

## Outcome

### Sweep

Phases 61 to 69. Read: the report of `scripts/sweep-report.sh` (35 hurdles: review-loop 12,
tooling 7, test-seam 6, design 4, process 3, ci 2, api-shape 1; 16 `→ none`), every Outcome and
Hurdles of phases 61 to 69, the phase 60 sweep and `plans/metrics.md`. Phase 60 had already read
phases 40 to 68, so only phase 69 was new; phases 61 to 68 were re-read for repeats across the
two sweeps. There is no phase 65: phase 68's plan reserved it, but it was never opened and the
`sweep` workflow never asked for it (below). The owner took every proposal on 2026-11-18.

- State that belongs to one tab landed on the tab shown: a trait default `active_tab` of 0, jobs
  aged and a rerun's follow-up sent for the shown tab, `set_filter(None)` moving the selection
  (66 four times, 69 once; 61 left a merge result in the backlog). The My PRs rule that every
  result carries its tab did not reach the other stages → rule in `src/app/AGENTS.md`
  ("Anything that belongs to a tab names it", for new and changed code; the shared caches are
  keyed by id; the follow-up refresh of a merge, update or status change still asks for the
  active tab, written as the exception) and backlog: the one-tab `StageView` defaults become
  required, `set_filter` never takes `None`, and those follow-ups name their tab.
- Owner decisions held up the review loop: the `refresh_secs` floor waited about 3 hours in 68,
  and 69's "dim tab" could not be seen, since inactive tabs are dim already (68, 69) → rule in
  root `AGENTS.md` ("Plan before code": ask while planning, each option checked against the
  screen).
- Sweep bookkeeping breaks under parallel phases: the issue opens only for "highest phase + 1",
  so it missed 60 (63 merged first) and 65 (67 merged first); the report starts after the sweep
  phase's number, so it re-read 61 to 68 and never reads a sweep phase's own hurdles (60, 65,
  70) → rule in root `AGENTS.md` ("The sweep": `### Sweep` opens with `Phases A to B`, the next
  reads from B + 1), the `PLAN.md` row for 65, and
  [#145](https://github.com/piacsek/conveyor/issues/145) for the scripts and the test.
- Tests that passed for the wrong reason: a prefix match on a notice (61), a filter to the row
  already first (66, 69) → rule in `tests/AGENTS.md` ("A test must be able to fail for the bug
  it names").
- The screenshots cannot show some states, twice `→ none` (64: a gap after CJK characters; 69:
  struck-through text). `scripts/truecolor.py` keeps only bold, dim and colours, so it drops
  reversed and struck-through text itself → [#146](https://github.com/piacsek/conveyor/issues/146).
- Minor findings fixed after the clean round were checked only by the orchestrator (60, 61, 62,
  63, 64, 66, 68, 69), which no rule covered → rule in root `AGENTS.md` ("Review before ready":
  minor findings after the clean round).
- The metrics start was the worktree's creation, made at plan time in 69, hours before the build;
  waits on the owner were in the Notes only when remembered (68, 69) → rule in root `AGENTS.md`
  ("Time and tokens": Start is the build's start, waits on the owner with their span).
- A sweep runs beside the phases numbered after it (60 after 61 to 68; 70 beside 71 to 73) →
  declined: parallel throughput is the owner's choice, and the range line makes what a sweep
  read explicit.
- Hurdles the phase 60 sweep already decided (`git add -A` during a ship, `ship.sh | tail`, the
  shared scratchpad, the macOS runner, a test after its code, a test binary exiting 101, clippy
  after a targeted test) → declined: none came back in 69, the one phase since those rules.
- The Codex reviewer and the orchestrator are `unknown` in every row from 53 → declined again:
  no tool here can read them.
- 63's flake `build=false` path never ran in real CI → declined, resolved: #136 (phase 67) ran
  `paths`, skipped `flake` and passed `flake-ok`.
- Single hurdles (60: a rewording widened a rule's scope; 61: a review asked for a known limit
  that cannot happen; 62: the text guard hook blocked a grep for em dashes) → declined: seen
  once each.
- Metrics, phases 61 to 69 unless named: review rounds (61: 2, 62: 3, 63: 1, 64: 2, 66: 2,
  67: 1, 68: 4, 69: 2; 58 and 59 had 4, 60 had 3), Claude reviewer tokens 102,103 (67) to
  148,612 (61) a round (58 to 60: 75,661 in 59 to 154,312 in 60) and builder tokens 101,213
  (61) to 169,131 (64) a hand-back show no drift. Start to draft PR is 7 to 12 minutes in every
  row of 61 to 69 (69 from the build's start); start to the clean review is 28 to 66 minutes in
  61 to 67 and 69 (69 from the build's start), and in 68 5h15m19s, of which its Notes record
  about 3 hours as waiting, not work; merge to release about 2 minutes except 62 (a runner failure) and 66.

### Phase

- The metrics rows of 69, 71, 72 and 73 got their ready, merged and released cells from GitHub,
  and `PLAN.md` the releases of 69, 71 and 72.
- Phase 60's docs-only branch ran the full flake build, since it touched `src/ui/AGENTS.md` and
  the flake reads `./src` whole; correct as the inputs stand, so left as found.
- Review: two rounds, the second unprimed and clean. Round 1 found that the tab rule stated as
  fact what the code does not do (the follow-up refresh of a write asks for the active tab), so
  the rule now covers new and changed code and names that exception, which joined backlog item
  B; it also corrected the metrics ranges, made the report's argument explicit until #145 lands,
  and filled the phase 70 row. The minor findings of round 2 (the follow-up's 10 s and when it
  asks, the `sweep` skill still running the report with no argument, an adjusted figure for 68
  more precise than its Notes) were fixed in one ship and checked by the orchestrator against
  the diff.

## Hurdles

- `process` The report re-read phases 61 to 68, which phase 60 had already swept, and the `sweep` workflow never asked for the sweeps of 60 and 65 → rule in `AGENTS.md`
- `review-loop` The new tab rule stated as fact that write follow-ups and job lookups carry their tab, which the code does not do, and round 2 found the follow-up's timer misnamed → backlog
- `review-loop` The rule that a sweep passes B + 1 to the report left the `sweep` skill running it with no argument → code
