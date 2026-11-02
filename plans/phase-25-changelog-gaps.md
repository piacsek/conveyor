# Phase 25: the gaps left by phase 24

## Context
Phase 24 (v0.20.0) shipped the `c` changelog pane and the changelog check with three known minor gaps, listed in its PR. Closing them, as a patch release v0.20.1 because the first is visible to users.

## Tasks (each starts red)
1. **A notice is shown over the changelog and help panes.** A merge, rerun or update result that lands while `c` or `?` is open sets `App::notice`, but those panes draw a fixed footer and the next key clears the notice, so it was never seen. While a notice is set, the overlay footer shows it instead of its hint (`src/ui/mod.rs`). Tests in `tests/changelog.rs`.
2. **Release dates must be calendar dates.** `changelog::dated` checks month 01-12 and the day against the month's length (leap years included). Tests feed `check` fixed text.
3. **An older tag missing from the branch says how to fix it.** When a released tag below the crate version has no entry, the message adds `merge origin/main if another branch released it`. Test on `check_released`.
4. **Release v0.20.1**: bump, `CHANGELOG.md` entry, snapshots' version.

## Verification
`scripts/gates.sh`; `conveyor-dev`, `M` on a card, `y`, `c` while it runs: the result shows in the changelog footer.
