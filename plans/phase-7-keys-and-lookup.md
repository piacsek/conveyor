# Phase 7: keys and the deployed lookup (requested 2026-09-10)

No plan file was written for this phase. This file holds its outcome, moved from `PLAN.md`.

## Outcome

- v0.6.1: the sha → pull request lookup froze a miss for the life of the process, so a Deployed
  row read in the seconds after a rollout showed `no pull request found for this commit` until
  a restart. `fetch::Pulls` now retries a miss on a 120 s window, falls back to the `(#N)`
  squash suffix, verifies the pull request actually contains the commit, and never treats a
  `gh` error as an answer. Two sub-agent review passes drove the design; the second confirmed
  each finding of the first was fixed and killed by a mutation of its test.
- v0.7.0: `q` is the only quit key. `Esc` leaves zoom and otherwise does nothing, so an
  accidental press no longer ends the session. `z` still toggles zoom both ways.
- New working agreement, from the user: marking a PR ready for review means a sub agent reviews
  the branch diff first, and its findings are addressed before `gh pr ready`. Written into
  AGENTS.md.
