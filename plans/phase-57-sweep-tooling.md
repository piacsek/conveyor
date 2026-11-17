# Phase 57: the sweep gathers its own data and asks for itself

## Context

Phase 56 made every phase record its hurdles (`## Hurdles`, tagged) and its time and tokens
(`plans/metrics.md`), and added a sweep every fifth phase that reads them. The sweep was
manual: nothing gathered the data, nothing reminded anyone, nothing failed when it was skipped.
The user asked whether CI could do it, and then for this phase plus a Claude Code skill in the
repository that analyses the data, to be run by hand.

What a script can and cannot do decides the split. Counting hurdles per tag, listing every
`→ none`, picking the metrics rows of the phases since the last sweep and naming their empty
cells are mechanical. Telling that two hurdles worded differently are the same problem, and
deciding what each becomes (a rule, a backlog item, an issue, a decline), is judgment: the
skill's job, not CI's.

## Verified facts (2026-11-17, `main` at bc6c32c)

- The `version` check runs `main`'s `scripts/feeds-binary.sh` on a pull request, not the head's
  (phase 55's hurdle), and that list has no `.claude/`, so a pull request that adds
  `.claude/skills/` counts as feeding the binary and needs a fragment, which releases a patch.
- Claude Code finds a project skill only at `.claude/skills/<name>/SKILL.md`; `.gitignore`
  ignores `/.claude/worktrees/` only.
- The runner's `awk` and this container's are `mawk`: no `asort`, so sorting goes through `sort`.
- `GITHUB_TOKEN` with `issues: write` can open an issue; nothing in this repository writes to
  `main` except the release job, so CI cannot backfill `plans/metrics.md`.

## Decisions

- The user chose two pull requests over one with an `internal` fragment: the first adds
  `.claude/` to `scripts/feeds-binary.sh` with the tooling; the second, after it merges, adds
  the skill and needs no fragment. Nothing is released.
- The report names nothing as a repeat and recommends nothing; the skill does.
- The issue is opened once per sweep: an issue of the exact title, open or closed, stops a
  second one.

## Tasks

PR 1:

1. `scripts/feeds-binary.sh` lists `.claude/` (red first in `tests/scripts/feeds_binary.rs`).
2. `scripts/sweep-report.sh [<first phase>]`: by tag, went nowhere, every hurdle by tag, plans
   with no `## Hurdles`, the metrics rows and their empty cells; from the phase after the last
   `### Sweep`, else 40 (red first in `tests/scripts/sweep.rs`).
3. `scripts/sweep-issue.sh` and `.github/workflows/sweep.yml` (red first, same file).
4. `tests/plans.rs`: a phase whose number is a multiple of 5, from 60, holds `### Sweep` in its
   `## Outcome`.
5. `AGENTS.md`, `.github/AGENTS.md`, phase 56's metrics row backfilled, this plan.

PR 2:

6. `.claude/skills/sweep/SKILL.md`: runs the report, reads the hurdles in full, fills the missing
   times from GitHub, groups repeats, proposes a disposition for each, and writes the
   `### Sweep` section only when the user agrees. `AGENTS.md` names it.

## Outcome

PR 1 shipped tasks 1 to 5. The report on `main` at the time covers phases 40 to 56: 33 hurdles,
`review-loop` the most frequent (8), 14 of them `→ none`. Phase 56's row now holds its merge
time; its ready time stays `unknown`, since the session that built it had no way to read the
pull request timeline. PR 2, the skill, follows once PR 1 is on `main`.

## Hurdles

- `ci` The `version` check judges a pull request with `main`'s `feeds-binary.sh`, so a new path outside the binary takes a pull request of its own before anything can live there → none
- `tooling` The tools of this cloud session cannot read a pull request's `ready_for_review` time, so phase 56's row keeps `unknown` → none
- `ci` `sweep-report.sh` measured `" → "` in bytes, which `mawk` here counts and the runner's `gawk` in a UTF-8 locale does not, so `check` failed on #127 → rule in `tests/AGENTS.md`
