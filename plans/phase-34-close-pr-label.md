# Phase 34: the close offer reads `close PR`

## Context
In the `S` popup of My PRs the last offer reads `x  close`. The user wants it to read `x  close PR`. Only that label changes: the key, the confirm question (`close #N <title>?`), the footer notices and every other offer stay as they are.

## Verified facts
- The label is the literal in `offer('x', "close", …)` in `PrsStage::offers` (`src/app/stages/prs.rs`). `Offer::label` is read in one place, `ui::dialog::draw_manage`, which draws `key  label` or `key  label · reason`.
- The confirm question is built on its own in `ui::dialog::question` (`close #N <title>`), the footer wording in `app/actions/status.rs` and the `gh` argv in `sources/github.rs`: none of them reads the label.
- `tests/manage.rs` pins `x  close` twice (the popup listing and the no-hint test), and the snapshot `manage_layout` (80 by 16) shows the popup. A `contains("x  close")` also matches the new label, so both assertions move to `x  close PR` to go red.
- The popup's inner width is the longest line, at most 56 columns and 8 less than the body. `close PR` is 8 characters and `leave the merge queue` and `check out locally` are always longer, so it never sets the width. It wraps only on a body under 19 columns; there `merge` (5 characters) still fits on one line.
- `README.md` and the help view do not list the popup's offers. `src/app/AGENTS.md` lists them (`x` close). `docs/details.png` shows the details pane, not the popup.

## Tasks (each starts red)
1. The offer under `x` reads `close PR`. Tests in `tests/manage.rs`, snapshot `manage_layout` updated, `src/app/AGENTS.md` follows.
2. Release v0.24.1: bump, `CHANGELOG.md` entry, outcome in `PLAN.md`.

## Verification
`scripts/gates.sh`; `conveyor-dev`, `S` on a card: the popup ends at `x  close PR`.

## Outcome

v0.24.1 (phase 34, `plans/phase-34-close-pr-label.md`): the close offer in the `S` popup
reads `close PR`. The user asked for the wording and for a release of its own, although
copy-only wording normally rides the next behaviour release. The label is one literal in
`PrsStage::offers` and only `draw_manage` reads it, so the key, the confirm question and the
footer notices are untouched. The existing `contains("x  close")` assertions also matched
the new label, so they were moved to `x  close PR` to see them red first.
