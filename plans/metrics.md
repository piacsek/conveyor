# Time and tokens per phase

One row per phase from phase 53, the first to record them; earlier phases have none and are
not backfilled. Times are UTC. Each token figure says where it came from: `measured` is the
`subagent_tokens` of the agent's last completion notice (a running total; whether cached input
counts like fresh input is not known), `estimated` names its source, and what nobody can read is
`unknown`. The builder writes its row before `gh pr ready`; the ready, merged and released cells
cannot be known on the branch, so the next phase's first ship fills them in from GitHub (see
"Time and tokens" in `AGENTS.md`). `End` is the release, or the merge with "no release" for a
branch that feeds nothing into the binary.

| Phase | PR | Start | Draft PR | Clean review | Ready | Merged | Released | Start → end | Rounds | Builder | Claude reviewer | Codex reviewer | Orchestrator | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 53 | #108 | 2026-11-17 04:56:47Z | 05:02:19Z | unknown | 05:19:27Z | 05:19:31Z | v0.34.1 05:21:50Z | 25m03s | 1 (clean) | 115,897 measured | 93,701 measured | unknown | unknown | Backfilled after the release. One orchestrator session shared with phases 54 and 55. |
| 54 | #115 | 2026-11-17 04:56:47Z | 05:06:49Z | unknown | 05:36:13Z | 05:36:16Z | v0.34.2 05:37:58Z | 41m11s | 1 | 138,798 measured | 121,972 measured | unknown | unknown | Backfilled. Round 1: one test tightened, one finding declined. Part of the time went to catching up with `main` after phases 53 and 55 merged. |
| 55 | #109 | 2026-11-17 04:56:47Z | 05:02:22Z | unknown | 05:17:12Z | 05:26:54Z | no release | 30m07s | 1 | 155,003 measured | 90,368 measured | unknown | unknown | Backfilled. Docs only. Round 1: three wording fixes. The builder's figure rose across its notices (126,609, then 134,457, then 155,003). |
| 56 | not opened | 2026-11-17 07:47:02Z | not opened | not run | | | no release | | 0 | none (no sub-agent) | not run | not run | unknown | Built by the main session alone in a cloud session, with no `/pr-review` and no draft PR; see the phase outcome. |
