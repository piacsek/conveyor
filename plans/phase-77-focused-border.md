# Phase 77: the focused column's border has the colour of the selected card's bar

## Context

The focused column is told apart only by its bold title; the other titles are dim. Every border
is drawn in the terminal's default colour, so on a wide screen it takes a second look to see
which column the keys act on. The selected card already has a clear mark: a cyan `▌` on each of
its lines. The owner asked for the focused column's border to be highlighted the same way.

Phase 75 stays free for the sweep of phases 70 to 74.

## Verified facts (2026-11-18, at `baea45b`)

- `column_block` (`src/ui/columns.rs`) builds `Block::bordered()` with the title bold when
  `stage == app.focus`, dim otherwise; it sets no border style.
- A column with tabs is drawn as two blocks: the titled block, then a second
  `Block::bordered().borders(LEFT | RIGHT | BOTTOM)` below the tab bar. Both need the colour, or
  the top edge and the sides would differ.
- `draw_column` is called from three places in `src/ui/mod.rs`: the four columns side by side,
  the zoomed column (`app.zoom`), and the narrow layout, where it draws the focused column with
  `titled == false` under the stage bar.
- The selected card's bar is `Span::styled(BAR, Style::default().fg(Color::Cyan))` in
  `src/ui/card.rs`; the colour is written there and nowhere else. `src/ui/AGENTS.md`: colours
  come from the ANSI palette, never hex, and the highlight is minimal on purpose (no
  background, no reverse video).
- Only the focused column paints its selected card, so the cyan bar and the cyan border always
  appear in the same column.

## Design

- One accent in `src/ui/style.rs` (the cyan of the bar), used by `card.rs` for the bar and by
  `columns.rs` for the border, so the two cannot drift.
- The focused column's border is drawn in the accent: the titled block and, with tabs, the
  block below the tab bar. The title keeps its bold default colour; the tab bar is unchanged.
- Unfocused columns keep today's border.
- The details, log, help and changelog panes, the `S` popup and the confirm dialog are not
  columns and keep their borders.
- Docs: every committed image that shows the columns is reshot (`scripts/screenshots.sh`), and
  `docs/demo.gif` is rerecorded since its frames change. `src/ui/AGENTS.md` gains the rule.
  Fragment `changes/phase-77-focused-border.md`, `kind: feat`.

### States

| State | Border |
| --- | --- |
| four columns, one focused | the focused column's border in the accent, the other three as today |
| focus moves with `h`/`l` | the accent moves with it on the same frame |
| the focused column has tabs | the top edge, both sides and the bottom in the accent, with no break at the tab bar |
| the focused column is empty, loading or failed | the accent all the same: it marks focus, not content |
| zoom (`z`) | the one column drawn is focused, so it is in the accent |
| narrow layout | the one column drawn is focused, so it is in the accent |
| the details or log pane is open | the focused column keeps the accent; the pane's border is unchanged |
| the `S` popup, confirm dialog, help or changelog is open | columns as before the popup; the popup's border is unchanged |
| a terminal theme that remaps cyan | follows the theme, like the bar |

## Tasks

1. The focused column's border in the accent, with and without tabs, zoomed and narrow, from
   tests that read the cell styles of a `TestBackend` frame. The accent goes into `style.rs` as
   the green step, and the bar reads it in the refactor step: on its own it changes nothing a
   test can see, so it has no red of its own.
2. Screenshots, the recording, `src/ui/AGENTS.md`, the fragment.

## Outcome

- `style::ACCENT` (cyan) is the one colour of the selection bar and of the focused column's
  border. `columns::border` gives it to the titled block and, with tabs, to the block below the
  tab bar, so the four edges run unbroken past the tab bar; the column bar of the narrow layout
  and the zoomed column take it through the same `draw_column`.
- The title sets `fg(Color::Reset)`: ratatui patches the title over the border's style, so
  without it the focused title came out cyan too.
- Tasks 1 and 2 of the first plan shipped as one cycle: the accent alone changes nothing a test
  can see, so it went in as the green and refactor steps of the border's test.
- `tests/border.rs` reads cell colours from the frame: four columns, focus moving, tabs (all
  edges, the tab bar unchanged), loading, empty and failed columns, zoom, the narrow layout,
  the details pane, the `S` popup, the confirm dialog, the help and changelog panes, and
  unfocused columns with and without tabs.
- Every screenshot that shows the columns was reshot (`help.png` did not change) and
  `docs/demo.gif` rerecorded.
- The root `AGENTS.md` gained the `gaswtr` rule beside "Work autonomously", at the owner's
  request during the build.

## Hurdles

- `test-seam` the focused title turned cyan with its border, since ratatui patches the title over the border style; a test written before the change caught it → code
- `test-seam` the `S` popup and the confirm dialog fill a 12-row terminal from its first row, so the tests that find the dialog's corner run at 24 rows → none
- `process` the first ship's output went through a pipe into a log (exit status read with `pipestatus`), against the rule of never piping `ship.sh`; later ships redirect → none
