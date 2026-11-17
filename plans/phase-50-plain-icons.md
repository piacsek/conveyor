# Phase 50: no required Nerd Font

## Context
Issue #96, the last of three phases (49 demo, 51 Nix flake, 50 plain icons). Done when
conveyor renders legibly without a Nerd Font. A terminal cannot tell whether its font is a Nerd
Font, so there is no detection: a config key picks the icons, and its default needs no Nerd
Font, so the first render (and `conveyor --demo`) never shows a box.

## Verified facts (2026-11-16, read in the code at `8cf33df`)
- The only private-use code points under `src/` are the three of `pr_glyph` in
  `src/ui/style.rs`: `\u{f4dd}` draft (DarkGray), `\u{f4db}` in the merge queue (Yellow),
  `\u{f407}` open (Green). A scan of `src/`, `tests/` and `scripts/` for U+E000 to U+F8FF and
  above U+F0000 finds no other, the footer logo and the queue position mark `⇥` included;
  only the `.snap` files carry the three.
- `pr_row` in `src/ui/rows.rs` is the one caller of `pr_glyph`; `draw_column` in
  `src/ui/columns.rs` calls `pr_row` and holds `app.config`.
- The card says the queue position in words (`⇥N` before the age); a draft is told by its
  icon and colour only (the meta line dropped `draft` when the icons came in).
- `MergeMethod` is a lowercase serde enum: an unknown value fails `config::load` with the file
  named, `conveyor config` exits 1, and the TUI starts with `Config::default()` and the error
  as its notice (`src/main.rs`).
- `demo::config` keeps the user's `[ui]`.
- `scripts/screenshots.sh` runs `--demo` with `HOME` a tempdir and `XDG_CONFIG_HOME` and
  `CONVEYOR_CONFIG` unset, so the config it reads is `$home/.config/conveyor/config.toml`.
- Font coverage, read from the font files with `fontTools` (cmap, advance equal to `M`):
  `↑` U+2191, `→` U+2192 and `≈` U+2248 are in Menlo, SF Mono, DejaVu Sans Mono 2.37,
  Cascadia Mono 2404.23 and FiraCode Nerd Font Mono, all one cell. Monaco lacks `↑` (and every
  check glyph). Consolas not verified (no file here); all three are in WGL4, which it covers.

## Design
- `[ui] icons`, `"plain"` (default) or `"nerd"`: `config::Icons`, a lowercase serde enum like
  `MergeMethod`, printed by `conveyor config`.
- `pr_glyph(icons, is_draft, queued)`: `nerd` gives today's octicons; `plain` gives
  `↑` open, `→` in the merge queue, `≈` draft, in the same colours.
- Why this trio: arrows and `≈` are in every common monospace font and one cell wide; none
  looks like a check glyph (`✓ ✗ ● ○ ? - ⊘`), a deploy icon (`⚠ ‼ ◐`), the selection bar `▌`
  or `▸`. `→` echoes the `⇥N` queue position on the same line, `↑` reads as a change on its
  way up, `≈` as a rough version.
- `scripts/screenshots.sh` writes `icons = "nerd"` into its temporary config for
  `docs/details.png` and `docs/help.png`, and shoots `docs/plain.png` (all four columns) with
  no config.

## States

| `[ui] icons` | open | in the merge queue | draft |
| --- | --- | --- | --- |
| absent (default) or `"plain"` | `↑`, green | `→`, yellow | `≈`, dark gray |
| `"nerd"` | `\u{f407}`, green | `\u{f4db}`, yellow | `\u{f4dd}`, dark gray |
| anything else | config error naming the file; `conveyor config` exits 1; the TUI starts with the default config (plain) and the error as its notice | | |

The busy spinner, the failure line, the OSC 8 link and `⇥N` do not change.

## Tasks
1. `conveyor config` prints `icons = "plain"` under `[ui]`; `"nerd"` loads; any other value
   fails naming the file.
2. PR cards draw the plain icons by default and the octicons under `icons = "nerd"`;
   snapshots reviewed.
3. `scripts/screenshots.sh`: `nerd` for `docs/details.png`, a new `docs/plain.png`.
4. README, `src/ui/AGENTS.md`, root `AGENTS.md`, fragment, `PLAN.md` outcome.
