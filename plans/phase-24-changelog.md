# Phase 24: `c` opens the changelog in the app

## Context
No way to see what changed between versions from inside conveyor. There is no CHANGELOG file; GitHub release notes are auto-generated one-liners. Decision (user): bundle a hand-written `CHANGELOG.md` into the binary with `include_str!`, open it with `c`, sharing the `?` line in the help (`?/c  help / changelog`) so the help still fits 17 rows.

## Setup
- Worktree: `git worktree add -b phase-22-changelog .claude/worktrees/phase-22-changelog origin/main`, then `git branch --unset-upstream`.
- Plan copied to `plans/phase-24-changelog.md` (phases 22 and 23 were taken; the branch kept its `phase-22-changelog` name), committed with the first ship.
- `/my-tdd autonomous`, one commit per task via `scripts/ship.sh`.

## Tasks (each starts red)

1. **`CHANGELOG.md` at repo root, backfilled.** One `## vX.Y.Z (YYYY-MM-DD)` section per tag (30 tags, newest first), 1 to 4 user-visible bullets each, drawn from `PLAN.md` phase outcomes, `git log vA..vB`, and tag dates (`git log -1 --format=%as <tag>`). No employer data (public repo); run `scripts/scrub-check.sh`.
   - Test (new `tests/changelog.rs`): the top section's version equals `env!("CARGO_PKG_VERSION")`. This also enforces "update the changelog on every release".
2. **`Mode::Changelog` + `c` key.** `src/app.rs`: new `Mode::Changelog`, `KeyCode::Char('c')` in Normal sets it and `changelog_scroll = 0`. In Changelog mode: `j/↓`/`k/↑` move 1 line, `Ctrl-d`/`Ctrl-u` half a page, any other key returns, `q` included (like the help pane). Arm sits next to `Mode::Help` (before the scroll arms, like Confirm).
   - Tests in `tests/changelog.rs` (TestBackend via `run()`): `c` shows `Changelog` title + current version line; `Esc` and `q` return to columns; `Ctrl-d` scrolls; `Ctrl-c` quits.
3. **Draw it.** New `src/ui/changelog.rs` with `draw_changelog(frame, area, app)`: bordered block titled `Changelog`, `## ` lines rendered bold without the `## ` prefix, bullets as-is (dim not needed), `Paragraph::scroll`. Reuse the details-pane pattern: the draw owns the clamp (`scroll.min(overflow)`), writes `changelog_page`, title grows `↑`/`↓` (see `draw_details` in `src/ui/details.rs` and the scroll handler around `src/app.rs:885`). Footer: `j/k C-d/C-u scroll · any other key returns`. Hook in `ui::draw` (`src/ui/mod.rs:30`) next to the Help branch. Text comes from `CHANGELOG` in `src/changelog.rs` (`include_str!("../CHANGELOG.md")`), wrapped to the pane width with `text::wrap` before counting lines for the scroll clamp.
   - No snapshot: the pane shows the live changelog, so it would change on every release. Pane tests set `App::changelog` to a fixed text.
4. **Help + docs.** `KEYS` line `("?", "help")` → `("?/c", "help / changelog")` in `src/ui/help.rs`; README key table mirrors it (`tests/readme.rs` enforces). AGENTS.md: one bullet under "Behaviour that is easy to break" (changelog mode keys, include_str, top version must match Cargo.toml) and a `src/ui/changelog.rs` line in Layout; Releasing step 1 gains "add the `CHANGELOG.md` section". PLAN.md: phase 24 entry.
5. **Release v0.20.0** (new key = minor): bump `Cargo.toml`, changelog top section `v0.20.0`, screenshot unchanged (no layout change to the main view).

## Gotchas
- `include_str!` path is relative to the source file.
- Changelog test pinning the top version fails on every bump until the section is written: intended.
- `c` vs `Ctrl-C`: the `control && 'c'` quit check at `src/app.rs:923` runs first, so plain `c` is safe.
- Filter mode types `c` into the query; the new arm must only match in Normal.

## Verification
- `scripts/gates.sh` green (fmt, clippy, tests, e2e).
- `scripts/dev-install.sh`, run `conveyor-dev`, press `c`: newest version on top, `C-d` scrolls, `Esc` returns, `?` shows `?/c`.
- Before ready: `/pr-review` per AGENTS.md.

## Estimate
About 2 to 3 hours; the backfill (task 1) is roughly half of it.
