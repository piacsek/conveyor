# Phase 34: the close offer reads `close PR`

## Context
In the `S` popup of My PRs the last offer reads `x  close`. The user wants it to read `x  close PR`. Only that label changes: the key, the confirm question (`close #N <title>?`), the footer notices and every other offer stay as they are.

## Verified facts
- The label is the literal in `offer('x', "close", …)` in `PrsStage::offers` (`src/app/stages/prs.rs`). `Offer::label` is read in one place, `ui::dialog::draw_manage`, which draws `key  label` or `key  label · reason`.
- The confirm question is built on its own in `ui::dialog::question` (`close #N <title>`), the footer wording in `app/actions/status.rs` and the `gh` argv in `sources/github.rs`: none of them reads the label.
- `tests/manage.rs` pins `x  close` twice (the popup listing and the no-hint test), and the snapshot `manage_layout` (80 by 16) shows the popup. A `contains("x  close")` also matches the new label, so both assertions move to `x  close PR` to go red.
- The popup's inner width is the longest line, at most 56 columns and 8 less than the body. `close PR` is 8 characters, shorter than every other label, so it never sets the width and wraps only on a body under 19 columns, where every other offer already wraps.
- `README.md` and the help view do not list the popup's offers. `src/app/AGENTS.md` lists them (`x` close). `docs/details.png` shows the details pane, not the popup.

## Tasks (each starts red)
1. The offer under `x` reads `close PR`. Tests in `tests/manage.rs`, snapshot `manage_layout` updated, `src/app/AGENTS.md` follows.
2. Release v0.24.1: bump, `CHANGELOG.md` entry, outcome in `PLAN.md`.

## Verification
`scripts/gates.sh`; `conveyor-dev`, `S` on a card: the popup ends at `x  close PR`.
