---
name: sweep
description: Run the periodic sweep of conveyor's phase hurdles and time-and-token metrics. Gathers the data with scripts/sweep-report.sh, finds hurdles that keep coming back and trends in the metrics, proposes what each finding becomes (a rule, a backlog item, an issue, or a decline), and on the user's yes writes the `### Sweep` section of the sweep phase. Use when the user says "sweep", "run the sweep", "analyse the hurdles/metrics", or when a "Sweep due before phase N" issue is open.
---

# Sweep

The sweep reads what the phases since the last sweep wrote down (`## Hurdles` in each
`plans/phase-N-*.md`, the rows of `plans/metrics.md`) and turns what keeps going wrong into a
change. "The sweep" and "Hurdles" in the root `AGENTS.md` are the rules; read them first. The
script counts; the judgment below is yours, and every decision is the user's.

## 1. Gather

1. Run `scripts/sweep-report.sh <B + 1>` from the repository root, B the last phase of the
   `Phases A to B` line that opens the newest `### Sweep`. With no argument it starts after
   the last phase holding `### Sweep`, which can re-read phases that sweep already read; pass
   B + 1 until #145 lands. Pass another phase when the user asks.
2. Read the `## Outcome` and `## Hurdles` of every phase in the report's range in full. A
   hurdle's one line is a summary; the outcome says what it cost.
3. Read the sweep before this one (the newest `### Sweep`), if any: what it decided and
   which rules it added.
4. For every metrics row the report lists under "Empty cells" with Ready, Merged or Released
   empty, read them from GitHub with the commands under "Time and tokens" in `AGENTS.md`.
   What cannot be read stays `unknown`; never estimate a time.

## 2. Analyse

Report only what the data shows, with the phases it comes from.

- **Repeats.** Group hurdles that are the same problem, whatever their wording or tag (a
  review round finding a state the plan missed, CI differing from local, a tool missing on
  the machine). A group of two or more is a finding; a single hurdle is not, unless it cost
  a release or a revert.
- **Dead ends.** Every `→ none` in a repeat group is a finding: twice declined is a pattern.
- **Rules that did not hold.** A hurdle marked `→ rule in <path>` whose problem shows up again
  in a later phase: read the rule and say why it did not prevent it.
- **Metrics.** Review rounds, start to end, and each measured token figure, per phase. Name a
  trend only with three or more comparable values; otherwise say there is too little data.
  Read the Notes cell before comparing two rows (a shared session, a backfill, a running total).
- **Process drift.** Phases that skipped a step of the autonomous flow (no review, no
  worktree, no metrics row) and whether that recurs.

## 3. Propose

Present one table, most costly first:

| # | Finding | Phases | Proposal | Where |
| --- | --- | --- | --- | --- |

A proposal is one of:

- **Rule**: name the `AGENTS.md` nearest the code it is about and draft its wording, short
  and factual like the rest of the file. Prefer a test that enforces it over prose.
- **Backlog**: one line for the `PLAN.md` backlog.
- **Issue**: title and body for a GitHub issue (public repository: no employer data, see
  "This repository is public").
- **Decline**: why it is accepted as is; a lasting decline may belong in "Known limits".

Then ask the user which proposals to take (AskUserQuestion, multi-select, at most four per
question). Never apply a proposal the user did not take.

## 4. Write

1. Into the sweep phase's plan file (`plans/phase-N-*.md`, N the multiple of 5 the sweep
   opens; ask for its topic and create it with empty `## Outcome` and `## Hurdles` headings
   if it does not exist yet), put `### Sweep` as the first subsection of `## Outcome`: its
   opening line `Phases A to B`, the range swept, then one line per finding with what was decided, and the metrics read.
2. Apply what was taken: the rules into their `AGENTS.md`, the backlog lines into `PLAN.md`,
   the backfilled times into `plans/metrics.md`.
3. Run `cargo test --test plans` (the sweep gate and the hurdle format), and
   `scripts/scrub-check.sh`.
4. Open an issue only when the user asks for it. Commit and push only when asked, and then
   through the repository's flow (`scripts/ship.sh`).
