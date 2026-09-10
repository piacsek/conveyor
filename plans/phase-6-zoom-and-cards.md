# conveyor phase 6 — zoom, deployed cards, full-height selection

## Context

conveyor v0.5.1 is released and installed. Read `AGENTS.md` first: ship flow, draft PR,
screenshots in PRs, the public-repo scrub gate, the card recipe, the tmux e2e traps.
`PLAN.md` is the phase index; this file is the versioned plan for phase 6.

Four requests from the user, verbatim where they set the design:

1. **`z` zooms the current panel, works like a toggle.**
2. **"On the currently deployed versions, prefer linebreaks over single lines. It's ok to go
   over 3 lines."** The Deployed card's dim lines are cramming three fields into one
   truncated line; give each field its own line and let the card be taller.
3. **"Please show the pr title before the author."** On Deployed cards today the author comes
   first and the title, the thing you actually read, is the part that gets cut.
4. **"I want it so that the whole card is highlighted, not just the initial bit of the first
   line"**, then: **"Minimalistic highlight I must say."** So the cyan `▌` bar runs down every
   line of the selected card as its left edge, the title stays bold, and nothing else changes:
   no background fill, no reverse video (reverse video was already rejected in phase 5).

5. Mid-phase: **"you forgot to show me 3 candidates for logo. I really dislike the current
   one"**. Offered a bare wordmark, `▰▰▰▸` belt segments and `▪─▪─▪▸` rollers on a rail; the
   user picked the rollers. The old `⟦▣⟧━⟦▣⟧━⟦▣⟧━▸` drew as a boxed `[回]` in a Nerd Font,
   which is why it read badly. Keep every logo glyph single-width: no brackets, no CJK-prone
   code points.

## Design decisions

**Zoom (task 1).** `App::zoom: bool`, toggled by `z`; `KEYS` gets `("z", "zoom the column")`.
`ui::draw` draws only `app.focus`, full width, when it is set. The narrow (tabs) layout already
shows one column at a time, so zoom changes nothing visible there but still toggles. Filter,
details and the footer are untouched. No config key.

**Deployed lines (tasks 2 and 3).** One field per line, in reading order, and the
`below.truncate(2)` cap goes:

```
▌ ◐ prod                                   ↓4
▌   #4790 Cancel the pending notification
▌   job when a tour is rebooked
▌   dave
▌   6babeb79 · read 1m ago
```

- `#<n> <title>`, the title word-wrapped over at most `DEPLOYED_TITLE_LINES = 2` lines through a
  new pure `text::wrap(text, width, lines) -> Vec<String>` (greedy fill, `…` on the last line
  when content remains, a word longer than the width is hard-cut). Unit-tested in `tests/text.rs`.
- the author on its own line,
- the error in red on its own line when there is one,
- `<sha8> · read <age> ago` last.

Only the Deployed column changes. My PRs, Merge queue and Main builds keep their compact
`meta()` lines: their titles are already on line 1, so nothing important is being truncated
there.

**Full-height bar (task 4).** `ui::card` prepends the bar span to every line it builds, not
only the title line, and the indent of the lines below becomes `bar + space + 2 spaces` so text
still starts at `INDENT`. The unselected card keeps a space there, so nothing shifts.

## Execution order (one `scripts/ship.sh` per task, strict TDD)

1. `z` zoom: a test in `tests/prs.rs` that the other three column titles disappear and come
   back; `KEYS`, help snapshot, README keys line.
2. `text::wrap` unit tests, then the Deployed card lines and the title-before-author order;
   `tests/deployed.rs` line assertions; snapshots.
3. The bar down every line: assert the bar cell on each line of the selected card in
   `tests/prs.rs`; snapshots.
4. Docs (README keys and the Deployed bullet, AGENTS card recipe and glyph paragraph),
   `plans/` + `PLAN.md` outcome, v0.6.0, screenshots re-rendered, PR body with commit-pinned
   images, CI, rebase-merge, tag, release, `brew upgrade`.

## Verification

- `scripts/gates.sh` green at every ship; CI green on the PR and on `main` after the merge.
- Manual with the user's own config: `z` on each column zooms and restores; a Deployed row
  shows the PR title before the author across as many lines as it needs; the selected card
  carries the cyan bar on every one of its lines.
