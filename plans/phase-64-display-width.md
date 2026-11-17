# Phase 64: titles with emoji or CJK characters keep inside their dialog and card

## Context

Issue #111. conveyor measures text in characters, not in terminal cells. A title holding emoji
or CJK characters takes two cells per such character, so the confirm dialog and the `S` popup
are sized too narrow for it and its end is cut at the right border, and a card pads such a
title too far, pushing its right-aligned figure out of the card. Listed under "Known limits" in
`AGENTS.md` and in the `PLAN.md` backlog.

## Verified facts (2026-11-17, `main` at 5dd801f)

- `src/text.rs`: `truncate`, `wrap` and `pad_right` count `chars()`.
- `src/ui/dialog.rs`: `draw_confirm` and `draw_manage` size the dialog from
  `line.chars().count()` of the wrapped lines and of the menu title.
- `src/ui/card.rs`: `card` pads the title with `pad_right` and subtracts
  `right.chars().count()`; `meta` cuts with `truncate`.
- `src/ui/rows.rs`: the unhappy checks line measures each check with `chars().count()`.
- `src/ui/columns.rs`: `tab_bar` measures tab names (from the user's config) with
  `chars().count()` and cuts with `chars().take`.
- The footer carries no pull request title: `footer_text` is a notice, an error, the filter or
  the refresh age, drawn by a `Paragraph`, which ratatui clips by cell width. Nothing to change
  there; the backlog's "the footer cuts titles" line is stale.
- `unicode-width` 0.2.2 is in `Cargo.lock` through ratatui; ratatui's `Line::width` uses it.
- The other `chars().count()` in `src/ui/` measure fixed ASCII text (the `#N` label, the help
  keys, the changelog bullet indent).

## Decisions

- `unicode-width` becomes a direct dependency; `text::width` is the one measure, and
  `truncate`, `wrap` and `pad_right` count with it.
- A character wider than the whole width is still taken on its own line by `wrap`, so the
  hard cut always moves forward.

## Tasks

1. `truncate` cuts by display width (`tests/text.rs`).
2. `wrap` fills and hard-cuts by display width.
3. `pad_right` pads to the display width.
4. The confirm dialog and the `S` popup are sized by display width: a wide title ends with
   `?` before the border (TUI test).
5. A card's unhappy checks line and the tab bar measure by display width.
6. Docs: the Known limits line and the `src/ui/AGENTS.md` sentence go, the backlog item is
   marked done, the fragment.

## Outcome

## Hurdles
