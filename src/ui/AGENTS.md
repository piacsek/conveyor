# src/ui

What is drawn: cards, glyphs and icons, dialogs, the details, help and changelog panes, the footer, colours. Rules and working agreements are in the root `AGENTS.md`.

## Cards and glyphs

- **A build card is four lines**: the number (or `run N` when no pull request is known) with
  the age on the right, then the **title on a line of its own**, then the meta line, then the
  jobs line. The title used to share the first line with the number and the age and was the
  part that got clipped — the one thing on the card you cannot reconstruct from the others.
  Row `n` therefore starts at screen line `1 + 4n`; tests that index a card's lines and any
  test asserting a third card need a terminal taller than the 12-row default.
  It costs density, deliberately: `List` drops an item that does not fully fit, so a card needs
  **four whole rows** — a Main builds pane under 6 rows draws a title over an empty body, and
  the last card of a full column vanishes whole rather than half-drawn. The bold that marks the
  selection moved with the title (`card()` only bolds its first line, which is now just the
  number), so `build_row` styles the title line itself.
  An empty `display_title` renders no title line at all rather than a blank one inside the
  card.
- **A cancelled run blames nobody.** GitHub marks the jobs a cancellation killed as failed, so
  a cancelled card used to carry a red `✗ <job> · <step>` line for work that never actually
  failed. `build_jobs`/`failed_job` drop the job line when the run is `Cancelled` and fall back
  to the sha; the details pane still lists every job, where the conclusions are stated as facts
  rather than as a headline.
- **`⊘` means cancelled, everywhere.** `build_glyph` gives `BuildStatus::Cancelled` its own
  glyph; the grey `-` stays for a *skipped* check (`conclusion_glyph`), which is a different
  fact. Never collapse the two: a cancelled run is work that was stopped, a skipped check is
  work that was never required.
- **Cards, not rows.** `ui::card(selected, glyph, title, right, rest, width)` renders a title
  line (`▌`/space, glyph, bold-when-selected title padded, right-aligned figure) plus one
  indented line per `rest` entry, each a `Vec<Span>` so a line can be dim, red or a mix. The
  cyan `▌` is drawn on **every** line of the selected card, so it reads as the card's left
  edge; that plus the bold title is the whole highlight. **Only the focused column draws it**:
  `draw_list` takes `focused` and filters `column.list.selected()` on it, so four cards never
  look selected at once. The unfocused column keeps its selection (and the list keeps it in
  view); it just does not paint it. No background, no reverse video: the
  user asked for a minimal one, twice. A
  per-stage row fn builds them (`pr_row`, `queue_row`, `build_row`, `deployed_row`) and
  `draw_list` wraps each in `ListItem::new(Text::from(lines))` with no `highlight_symbol`:
  the bar is drawn by the card from `Some(i) == column.list.selected()`. Cards carry what a
  browser trip would otherwise cost: the unhappy checks, the failed job and step, the
  deployed sha. Prefer adding a line there over adding one to the details pane. Deployed cards
  go further: one field per line rather than a joined-and-truncated `meta()`, the PR title
  before the author, wrapped with `text::wrap` over at most `DEPLOYED_TITLE_LINES` lines. Card
  height varies per row; the list widget handles it.
- **A PR card carries a pull-request icon** after its check glyph, picked by `[ui] icons`
  (`config::Icons`): `pr_glyph` gives green for an open pull request, yellow once it has a
  merge queue position and grey for a draft, so the meta line no longer says `draft`. The
  default `plain` draws `↑`, `→` and `≈`: one cell wide in Menlo, SF Mono, DejaVu Sans Mono,
  Cascadia Mono and FiraCode (checked in the font files, phase 50), and unlike any check,
  deploy or selection glyph. `nerd` draws GitHub's octicons U+F407, U+F4DB (git-merge-queue)
  and U+F4DD, the only Nerd Font glyphs in the UI: a terminal without a Nerd Font draws a box
  there, which the README's FAQ says. There is no detection, since a terminal cannot tell
  whether its font is a Nerd Font. `card()` takes the icon as an `Option<(char, Color)>`; every
  other stage passes `None`.
- **A card's `#N` is a terminal hyperlink (OSC 8).** After the list is drawn, `draw_list` asks
  `ui::link::link` to find the card's `#N` in the frame buffer, inside that card's own lines,
  and rewrites each of its cells as `ESC ] 8 ; id=<x>-<y> ; <url> ESC \ <char> ESC ] 8 ; ; ESC \`
  with `CellDiffOption::ForcedWidth(1)`, so ratatui counts the cell as one column whatever the
  escape measures. One cell per character, not the whole number squeezed into the first cell:
  every cell stays self-contained, so a partial redraw, a popup drawn over half a number and
  the `TestBackend` all see whole cells. The shared `id` is what makes a terminal underline the
  whole number on hover rather than one digit; it is the number's screen position, so two cards
  never share one. The terminal does the rest (hover, click; Cmd-click in iTerm2): conveyor
  captures no mouse, so text selection is untouched, and a terminal without OSC 8 ignores the
  sequence. The text and the style of the card do not change, so no snapshot or screenshot
  does. **The URL is written to the terminal raw, so `linkable` is the guard**: only an
  http(s) URL (`open::web_url`) made of printable ASCII without a space becomes a link; an
  escape or a control character in a URL would otherwise reach the terminal. The URL of each
  row is the one `p` opens (`Build::pull_request` for a main build, which falls back to the
  `(#N)` of a squash title). A number cut by `…` on a narrow card gets no link. Not linked:
  the `#N` in the details pane, dialogs and footer, and every `#N` in the demo (`app.demo`),
  whose `acme/*` URLs are made up: the terminal would open them past `DemoOpener`.
- **A busy PR card spins.** While a write started on a pull request is in flight
  (`App::write_on`), its card draws the footer's braille frame (`App::spinner`) in yellow in
  place of the check glyph, and its meta line starts with the action word (`Write::word`:
  `marking ready…`, `converting to draft…`, `merging…`, `updating…`, `rerunning…`,
  `checking out…`, `closing…`, and the auto-merge and merge queue ones), first so a narrow card
  keeps it. `draw_column` looks the word up per visible row before `draw_list` borrows the
  column. The card goes back to its check glyph when the result lands. Main builds cards do
  not spin; a rerun there shows its progress in the footer only (a failed one marks the card,
  see the next bullet).
- **A failed write leaves a red line on its card.** `pr_row` and `build_row` take the row's
  `Failure` (`App::failure_on`, looked up per visible row in `draw_column` like the busy word)
  and draw `! <action> failed: <reason>` in red, its whitespace runs (a multi-line gh error
  included) collapsed to one space and cut with `…`, as the line right under the
  title: the second line of a pull request card, the line after the title line of a build
  card. The card grows by that line; the check glyph and the rest are unchanged. With the
  marked card selected, the details pane starts with `<action> failed at HH:MM:SS: <reason>`
  in red, wrapped over the pane's width with nothing cut (`App::selected_failure`, `text::clock`
  with `utc_offset_secs`). A write that timed out (`Failure::unanswered`) did not fail, it may
  have happened: its line says only that (`! may be merged`, at most 25 characters, whole at
  every supported width), and its details line carries the rest, `<action>: <what may have
  happened>; gh did not answer within 60s (no answer by HH:MM:SS)`, the time being when
  conveyor gave up, never worded as when the write was sent. The column title counts marks after the row count, before the
  `⚠`: `My PRs (5) !1`. The count covers every row of the column, rows hidden by the filter
  included, like the `/total` half of the title. `column_title` feeds the column bar of the narrow layout too, so
  a mark behind it still shows. With My PRs tabs, `!N` counts the marked rows of the active tab. No mark, no suffix. What marks and what clears: see
  `src/app/AGENTS.md`.
- **A column's name comes from `[ui.titles]`, else the fixed one.** `help::column_name` is the
  one source for the column border, the narrow layout's bar and the help header; with a repo or
  system known, `column_title` puts it after the custom name, or after the short fixed one
  (`Queue`, `Main`, `Deployed`) when none is set.
- **No URLs in the details pane.** Every line there is text a person reads: the checks, the
  jobs, the sha, the pull request. A URL is 60 columns of noise nobody retypes, and both keys
  that need one (`b`, `p`) already hand it to the browser. `y`/`Y` copy them.
- **Colors** come from the ANSI palette so terminal themes apply. Do not hardcode hex.
- The Deployed row glyphs (`✓`, `◐`, `○`, `✗`): see the Deployed bullet in `src/sources/AGENTS.md`.
- The deploy health icons (`⚠`, `‼`, `?`) and the rules behind them: see `src/model/AGENTS.md`.
- A queue card whose run speaks for the entry: see `src/model/AGENTS.md`.

- **The tab bar.** A column with two tabs or more (`StageView::tab_names`: My PRs, Merge queue
  and Main builds) takes one line inside its block, above the cards: the names joined by ` │ `, the active one
  bold and reversed like the narrow layout's column bar, the others dim (`columns::tab_bar`).
  A tab `StageView::tab_struck` answers for (Merge queue: a repository GitHub answered has no
  merge queue) is also crossed out, active or not, which keeps the bar's width; the builds bar
  strikes nothing. The queue column then reads a dim `no merge queue` with no count in its title.
  Before a tab's first answer the Merge queue and Main builds titles name its configured repository.
  `draw_column` draws the block, the bar on its first inner line, and the list below it in a
  block with no top border, so the cards, widths and links work as in any column. When the
  names do not fit, tabs before the active one are dropped behind a leading `… │ ` until it
  fits, and the bar is cut after it with `…`. One tab draws no bar, so a config without
  `[[prs.tab]]` and with at most one `[[repo]]` looks as it did. In the narrow layout the column bar (`draw_column_bar`, named
  so that "tab" means only this) stays on top and the tab bar sits inside the column below it.

## Dialogs, panes and the footer

- **Every sensitive write asks in one dialog.** `ui::dialog::draw_confirm` draws over the body
  while `Mode::Confirm`, centred over the whole body (the details pane included, so an open pane
  never squeezes it), at most 56 columns inside and 2 off each edge: `Clear`, a
  bordered `Confirm` block, the question (`Pending::{Merge, UpdateBranch, Status}`) wrapped over at most 3 lines (cut with `…` before the
  `?`), a blank line and the dim `y confirms · any other key cancels`, wrapped over 2 lines on a
  narrow body rather than cut. `draw_manage` draws the `S` popup with the same geometry: each
  offer wraps over at most 2 lines with a hanging indent, and nothing else: no hint line (the
  user called it pollution; the dim reasons already explain the popup). On a short terminal the popup's bottom is clipped. The footer carries no
  prompt, only the usual notice or refresh age underneath; it used to hold the question and was
  easy to miss. A new sensitive action adds a `Pending` variant and an arm in `dialog::question`. The question
  names the pull request the way the popup title does (`PrsStage::label`): `#N`, or `infra#N`
  while the active My PRs tab lists more than one repository (`me/webapp#N` when two of them
  share a short name). Widths are terminal cells, not characters: `text::display_width`
  measures, and `truncate`, `wrap`, `pad_right` and `take_width` cut, so a CJK character or
  an emoji counts two and a wide title ends inside the dialog. A cut never splits an emoji from
  its selector (`♻️`) or a joined emoji (`👨‍👩‍👧`).
- **Overlays show what lands under them.** The help and changelog panes replace the footer
  with a hint, so while `App::notice` is set (a merge, rerun or update result that landed while
  the pane was open) the footer shows the notice instead; the next key clears it as usual.
- **The help pane does not scroll.** `draw_help` renders `KEYS` through a `Paragraph`, so a
  list longer than the pane is silently clipped from the bottom: 15 keys plus two borders and
  the footer exactly fill an 18-row terminal. The tab row shows the keys configured in
  `[ui.keys]` (`] [ 1-9`, `C-n`), not the defaults (`help::keys_for`). `("q", "quit")` is therefore first in `KEYS`, so the one
  key that gets you out is never the line that disappears. Adding a key means checking the
  smallest terminal you care about, or paginating the pane.
- **`c` shows the changelog, and every release writes one.** `CHANGELOG.md` is
  `include_str!`ed, so the pane always matches the binary and needs no network. In the pane
  `j/k`/arrows move a line, `Ctrl-d`/`Ctrl-u` half a page, any other key returns (like the
  help, so `q` closes it rather than quitting). `tests/changelog.rs` runs
  `changelog::check(CHANGELOG, CARGO_PKG_VERSION)`: the top entry is the crate version (only
  in a release commit: at `0.0.0`, the constant version of `main` and every branch, the top
  entry is the last hand-made release and is not compared), every
  heading reads `## vX.Y.Z (YYYY-MM-DD)` with a real calendar date, versions strictly descend, every entry has a `- `
  bullet with text (an indented line continues the bullet above it) and no bullet repeats an
  older one. Every `v*` git tag merged into `HEAD` must keep its entry (`check_released`; at a
  real crate version only the tags up to it; release commits are reachable only from their
  tags, so a newer release is never asked of `main`), so relabelling or
  deleting an old entry fails too; that test needs the tags, hence `fetch-depth: 0` in CI's
  `check` job, and it fails loudly in a clone without them. The pane of a build from `main`
  shows the changelog as frozen there, not the newest release.
- **Refresh feedback.** Fetchers send `Input::Fetching(stage)` before each fetch and set the
  column's `refreshing` flag; `Data` clears it. One braille spinner (frame from `app.now`, 100
  ms per frame, so the 250 ms tick advances it) is drawn at the very start of the footer while
  `App::any_refreshing()` holds, whichever column is focused; column titles and loading bodies
  carry no spinner. The footer says
  `refreshed just now` for 3 s, then `refreshed at HH:MM:SS` local time
  (`utc_offset_secs` from `chrono::Local` in `main`, 0 in tests). The bottom right holds the
  static `ui::logo()` with the crate version; nothing there changes between ticks. Under
  `--demo` (`App::demo`) a bold magenta `demo` sits in front of it, always, and alone at the
  right of the help and changelog footers: magenta is used nowhere else, so it never reads as
  a status.
- How the details pane scrolls (`draw_details` owns the clamp), the log pane and zoom: see `src/app/AGENTS.md`.
- What the `S` popup offers and when an action asks `y`: see `src/app/AGENTS.md`.
- The keymap behind `KEYS`: see `src/app/AGENTS.md`.

## Screenshots

- `scripts/screenshots.sh` renders `docs/details.png` (with the Nerd Font icons: it writes
  `icons = "nerd"` into its temporary config) and `docs/plain.png` (the default plain icons),
  plus one `shoot` per extra view. Freeze lays out an SVG with the font embedded; headless
  Chrome rasterises it, because freeze's own PNG output cannot draw Nerd Font glyphs.
- The script adds Menlo (DejaVu Sans Mono on Linux) as the SVG's fallback font family:
  FiraCode lacks `✗ ⊘ ▸`, and Chrome's default fallback is wider, so every row holding one
  shifted and the borders jogged.
- It runs `conveyor --demo`, which reaches neither `gh` nor `kubectl`, with `HOME` a tempdir
  (through `env`, see the e2e trap in `tests/AGENTS.md`), so the screen shows the demo's footer
  marker too.
