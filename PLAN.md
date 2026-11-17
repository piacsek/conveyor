# conveyor plan

The index of phases and the backlog. Each phase's plan, outcome and hurdles live in its own
file under `plans/`; its time and tokens are a row of [`plans/metrics.md`](plans/metrics.md).
Releases and fixes that had no phase are in [`plans/unphased.md`](plans/unphased.md).

## Phases

| Phase | Plan | Release |
| --- | --- | --- |
| 0–4 | [the approved plan of 2026-09-10](plans/original-plan.md): setup, My PRs, Merge queue, Main builds, Deployed | v0.0.1 to v0.4.0 |
| 5 | [cards, build details, static logo](plans/phase-5-cards.md) | v0.5.0, v0.5.1 |
| 6 | [zoom, deployed cards, full-height selection](plans/phase-6-zoom-and-cards.md) | v0.6.0 |
| 7 | [keys and the deployed lookup (requested 2026-09-10)](plans/phase-7-keys-and-lookup.md) | v0.6.1, v0.7.0 |
| 8 | [one highlight, a quiet details pane, and a keymap for PR *and* build](plans/phase-8-cards-and-keys.md) | v0.8.0, v0.9.0 |
| 9 | never used as a phase number; v0.9.0 is the follow-up in phase 8 | |
| 10 | [the merge queue shows its build](plans/phase-10-queue-run-details.md) | v0.10.0 |
| 11 | [the run speaks for the queue entry, and `⊘` means cancelled](plans/phase-11-queue-run-status.md) | v0.11.0 |
| 12 | [two things the cards were getting wrong](plans/phase-12-cards.md) | v0.12.0 |
| 13 | [a filter you can keep, keys that say what they do, and the red build's log](plans/phase-13-filter-keys-log.md) | v0.13.0, v0.13.1 |
| 14 | [My PRs: authored OR assigned, deduplicated](plans/phase-14-assigned-prs.md) | v0.14.0 |
| 15 | [steady jobs (no duplicates, no blink)](plans/phase-15-steady-jobs.md) | v0.14.1 |
| 16 | [PR icon on My PRs cards](plans/phase-16-pr-icon.md) | v0.15.0 |
| 17 | [a cancelled run's leftover job does not fail a PR card](plans/phase-17-superseded-runs.md) | v0.15.1 |
| 18 | [elapsed time on every check and job line](plans/phase-18-check-times.md) | v0.16.0 |
| 19 | [`X` reruns failed jobs (Main builds and My PRs)](plans/phase-19-rerun.md) | v0.17.0 |
| 20 | [`M`: merge the selected pull request after a y/n prompt](plans/phase-20-merge.md) | v0.18.0 |
| 21 | [`U` updates the selected PR's branch with its base](plans/phase-21-update-branch.md) | v0.19.0 |
| 22 | [reruns from GitHub, and a reload after every PR write](plans/phase-22-rerun-attempts.md) | v0.19.1 |
| 23 | [a second My PRs reload after `X` and `U`](plans/phase-23-follow-up-reload.md) | v0.19.2 |
| 24 | [`c` opens the changelog in the app](plans/phase-24-changelog.md) | v0.20.0 |
| 25 | [the gaps left by phase 24](plans/phase-25-changelog-gaps.md) | v0.20.1 |
| 26 | [`S` is the one key that changes things](plans/phase-26-manage.md) | v0.21.0 |
| 27 | [the `S` popup without its hint line](plans/phase-27-manage-no-hint.md) | v0.21.1 |
| 28 | [a log file](plans/phase-28-log.md) | v0.22.0 |
| 29 | [`m` adds to the merge queue where auto-merge is off](plans/phase-29-merge-queue.md) | v0.21.2 |
| 30 | [deploy health on Deployed cards and a fuller `d` pane](plans/phase-30-deploy-health.md) | v0.23.0 |
| 31 | [a kubectl that hangs is cut off](plans/phase-31-kubectl-deadline.md) | v0.23.1 |
| 32 | [the environments are read at the same time](plans/phase-32-parallel-deploy-fetch.md) | v0.23.2 |
| 33 | [check a pull request out from the `S` popup](plans/phase-33-checkout.md) | v0.24.0 |
| 34 | [the close offer reads `close PR`](plans/phase-34-close-pr-label.md) | v0.24.1 |
| 35 | [one call resolves every build's pull request](plans/phase-35-batched-pull-lookup.md) | v0.24.2 |
| 36 | never ran; see [releases and fixes outside a phase](plans/unphased.md) | |
| 37 | [the pipeline picks the version, a pull request cannot](plans/phase-37-automatic-versions.md) | v0.24.2 |
| 38 | [manage several pull requests at once](plans/phase-38-parallel-writes.md) | v0.25.0 |
| 39 | [a failed action stays on its card](plans/phase-39-failed-write-marks.md) | v0.26.0 |
| 40 | [tabs in the My PRs column, switched with Tab or 1-9](plans/phase-40-column-tabs.md) | v0.28.0 |
| 41 | [the column titles can be renamed in the config](plans/phase-41-column-titles.md) | v0.29.0 |
| 42 | [tabs in the Merge queue column, one per repository](plans/phase-42-queue-tabs.md) | v0.30.0 |
| 43 | [resize and the tick clock](plans/phase-43-resize-and-tick.md) | v0.30.1 |
| 44 | [My PRs tabs load and refresh in the background](plans/phase-44-background-tabs.md) | v0.31.0 |
| 45 | [each release lists only its own changes](plans/phase-45-release-notes.md) | none |
| 46 | [one generic worker](plans/phase-46-generic-worker.md) | v0.31.1 |
| 47 | [a failure records what kind it is](plans/phase-47-failure-kinds.md) | v0.31.2 |
| 48 | [the help view says Esc dismisses the error on a card](plans/phase-48-esc-help-wording.md) | v0.31.4 |
| 49 | [`conveyor --demo`](plans/phase-49-demo.md) | v0.33.0 |
| 50 | [no required Nerd Font](plans/phase-50-plain-icons.md) | v0.34.0 |
| 51 | [install conveyor with Nix](plans/phase-51-nix-flake.md) | v0.32.0 |
| 52 | [the Nix flake builds with the pinned Rust](plans/phase-52-flake-pinned-rust.md) | v0.34.3 |
| 53 | [casts that can lose a value fail the gates](plans/phase-53-cast-lints.md) | v0.34.1 |
| 54 | [a crashed background worker shows up where its result would have](plans/phase-54-worker-panics.md) | v0.34.2 |
| 55 | [a place for early users to ask, report and pick up work (#99)](plans/phase-55-contributing.md) | none |
| 56 | [each phase's outcome, hurdles and numbers live with its plan](plans/phase-56-plans-split.md) | none |
| 57 | [the sweep gathers its own data and asks for itself](plans/phase-57-sweep-tooling.md) | none |
| 58 | [the privacy scrub checks what is about to be committed (#120)](plans/phase-58-scrub-staged.md) | none |
| 59 | [a `gh` call that never returns no longer freezes its column (#124)](plans/phase-59-gh-deadline.md) | |
| 61 | [two pull requests with the same number in different repositories stay apart (#121)](plans/phase-61-repo-row-key.md) | v0.34.5 |
| 62 | [Main builds reads the repository's own default branch (#123)](plans/phase-62-builds-branch.md) | v0.35.0 |
| 63 | [the flake build runs only when its inputs change, and gates the merge (#117)](plans/phase-63-flake-gate.md) | none |
| 67 | [the release scripts read `cargo metadata` with a JSON parser (#126)](plans/phase-67-cargo-metadata.md) | |

## Backlog (discussed at the sweeps and retros)

- `argocd-git` and `github-deployments` fetchers; `argocd` CLI fetcher.
- More `S` actions: `gh pr review --approve` (#113); reopen if the query ever lists closed pull requests.
- Merge follow-ups from the phase 20 review: `merge_method` per `[[repo]]` (#112) (one global value cannot
  serve a squash-only and a merge-commit-only repo at once); a write result that lands while the other prompt is open is
  hidden by that prompt (the failure half is done in phase 39: a failure stays on its card; a
  success is still hidden); the footer cuts titles by characters, not display width (#111).
- A merge sent from one My PRs tab whose result lands after the user switched tabs is recorded
  against the tab now shown (`src/app/actions/merge.rs`, `land()`): that tab's refresh lifts the
  "waiting for My PRs to catch up" guard while the first tab still shows the stale pull request.
- Follow one PR across columns: `f` on a PR highlights its sha/number in queue, builds, deployed.
- Desktop/tmux notification on queue state change or main build failure (tmux-agents `watch` pattern).
- Main builds tabs (#114), one per `[[repo]]`, following the Merge queue's active tab (phase 42 gave
  the queue its tabs; decided with the user on 2026-11-14).
- Deployed tabs, one per application of the monorepo, switched independently of the queue.
- Native GitHub client if `gh` latency becomes a problem (measure first; each call ~0.3–1 s).
