# Phase 16: PR icon on My PRs cards

## Context
User wants GitHub's pull-request icon on each PR card: grey for draft, green for open. `is_draft` is already fetched and parsed (`src/model/prs.rs:88`), so this is render-only. Decisions taken: Nerd Font octicons (U+F407 `nf-oct-git_pull_request`, U+F4DD `nf-oct-git_pull_request_draft`); drop the `draft` word from the meta line.

## Change
1. `src/ui/style.rs`: add `pr_glyph(is_draft: bool) -> (char, Color)`: `('\u{f4dd}', Color::DarkGray)` for draft, `('\u{f407}', Color::Green)` otherwise. ANSI palette only.
2. `src/ui/rows.rs` `pr_row` (line 28): prepend the icon span before `#N title` on the first line; remove the `draft` entry from `parts` (lines 42-46).
   - `card()` takes `title: String`, so a coloured icon needs either an optional second glyph param on `card()` or a `title_prefix: Option<(char, Color)>`. Prefer the smallest: an extra `Option<(char, Color)>` arg, `None` from the other three row builders. Title width shrinks by 2.
3. Tests, red first:
   - TUI test in `tests/prs.rs`: a draft PR's card line contains `\u{f4dd} #N`, an open one `\u{f407} #N`, and the meta line no longer contains `draft`.
   - Colour asserted via the TestBackend buffer cell fg (DarkGray / Green), one assert per line.
   - Update `tests/snapshots.rs` snapshots and any `"#1 a"` substring asserts still hold (icon sits before `#`, so they should).
4. AGENTS.md: one bullet under the cards section: icon is a Nerd Font glyph, tofu without one; the only Nerd Font glyph in the UI.
5. `scripts/screenshots.sh`: regenerate `docs/details.png`. Check FiraCode Nerd Font in `~/Library/Fonts` has U+F4DD (added in Nerd Fonts v3); if not, Chrome falls back and the row shifts.

## Risks
- Users without a Nerd Font see a box. Accepted.
- Card first line loses 2 columns of title.

## Verification
- `scripts/gates.sh` (fmt, clippy, tests, e2e).
- `scripts/dev-install.sh`, run `conveyor-dev` against own PRs, confirm a draft shows grey icon and an open PR green.
- Ship via `scripts/ship.sh`; minor-ish visible change, so a patch/minor release per Releasing.
