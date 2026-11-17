# Releases and fixes outside a phase

Changes that shipped with no phase number and no plan file, moved from `PLAN.md` or read
from their fragment.

- v0.12.1: security fix. `b` and `p` only hand http(s) URLs to `open`/`xdg-open`. A check's
  `detailsUrl`/`targetUrl` belongs to whoever posted the check, and a `file://`, app-bundle or
  custom-scheme URL would have been launched as-is; now the footer says
  `refusing to open …` and nothing is spawned. Retro: the fix and its tests were first written
  in one edit; TDD is now spelled out in AGENTS.md as never skipped.

- v0.16.1: a pull request in a merge queue shows the git-merge-queue icon in yellow instead
  of the green open icon, and My PRs lists queued pull requests first (by queue position),
  then open ones, then drafts.
- v0.16.2: holding or mashing `r`/`R` no longer queues one reload per key repeat. Each
  fetcher drains queued refresh signals when it wakes (`refresh::wait`), so a burst costs at
  most one fetch beyond the running one. A 2 s window in `App` was tried first and dropped in
  review: `app.now` freezes while keys repeat, and skipping a column mid-fetch lost an `r`.

- Phase 36 never ran as a phase: its fix to `scripts/release.sh` for a release run GitHub left
  stuck in `queued` (draft #62) was closed unmerged because phase 37 replaces the script. The
  number stays unused; the stuck run (43098019853, v0.24.1) still exists and can be neither
  cancelled nor deleted (`.github/AGENTS.md`).

- v0.26.1 (#74, fragment `no-dead-code.md`, `kind: internal`): unused code fails the tests,
  and the unused code found was removed (`tests/dead_code.rs`).
- v0.27.0 (#69, fragment `feat-pr-num-link.md`): a pull request number on a card is a
  terminal hyperlink to the pull request, in all four columns.
- Toolchain pin (fix from #89, #102, no phase and no plan file; 2026-11-16): `rust-toolchain.toml`
  pins 1.98.1 with rustfmt and clippy, so `scripts/gates.sh` runs what `check` runs (the local
  default had been a newer stable). The `toolchain:` inputs of `ci.yml` and `release.yml` stay;
  `tests/toolchain.rs` keeps them, the file and `rust-version` equal, so a bump is one pull
  request. Decision: the `tag` job sets `RUSTUP_TOOLCHAIN: stable` and keeps the runner image's
  preinstalled stable, as before; without it rustup would download 1.98.1 there first, a new
  network step in the job that pushes the tag, and the merge of this pull request starts a
  release run. `version` was already immune (its cargo runs from a scratch directory with
  `RUSTUP_TOOLCHAIN` set); `build` installs rustfmt and clippy in its toolchain step, so cargo downloads nothing there
  (`tests/toolchain.rs` keeps that step's `components:` equal to the file's).
  Fragment `kind: internal`.
