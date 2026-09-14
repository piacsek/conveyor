# Phase 13 — a filter you can keep, keys that say what they do, and the red build's log

Requested 2026-09-14 for v0.13.0. Six improvements, one branch (`phase-13-filter-keys-log`),
one commit per task through `scripts/ship.sh`, TDD red first on every task.

## Why

1. **The filter cannot be committed.** In `Mode::Filter` every character types into the query,
   `Enter` does nothing and `Esc` is the only exit, which also clears the filter. You can
   filter or navigate, never both.
2. **The filter is global but only the focused column says so.** `sync_filter` sets the query
   on all four columns; the footer count and the column titles are focused-only, so rows
   vanish from three columns for no visible reason.
3. **`Esc` is overloaded and incomplete.** It leaves zoom and cancels the filter but does not
   close the details pane. In tab mode there is no `1`–`4` jump.
4. **Key semantics shift per column and the help does not say so.** `b` is the first failing
   check (PRs), the merge-group run (Queue), the run (Builds), the main build with this sha
   (Deployed); the help says `b  open build`.
5. **Deployed `b` dead-ends** when the deployed sha is older than the `builds` runs fetched:
   `no main build found for abc12345`, no fallback.
6. **A failed build dead-ends at the browser.** The pane shows the failed job and step, then
   `p`/`b` leave for Chrome. The most common reason to open conveyor is "CI is red"; the log
   of the failed step should be one key away.

## Design

- **Filter state.** `App.query: Option<String>` is the live filter applied to all four
  columns. `Mode::Filter(String)` means *editing* it; typing previews live as today. `Enter`
  commits: back to `Mode::Normal`, query kept, `j`/`k`/`g`/`G` move again. `Esc` while
  editing cancels: query cleared. `/` with a live query reopens it for editing. The footer
  shows `/query▏  visible/total` while editing and `/query  visible/total` once committed.
- **Counts everywhere.** While a query is live every column title reads `Name (visible/total)`
  instead of `Name (total)`; the tab bar inherits it.
- **`Esc` closes the topmost thing**, one per press: editing → cancel the filter; details pane
  open → close it; zoomed → leave zoom; live filter → clear it; otherwise nothing. `1`–`4`
  focus a column in every layout.
- **Context-sensitive help.** `KEYS` stays the generic table the README mirrors. The help view
  takes the focused stage and swaps the `p` and `b` descriptions for that column's
  meaning; its title names the column (`Keys — Main builds`).
- **Deployed `b` fallback.** When no fetched main build carries the deployed sha,
  `https://github.com/<repo>/commit/<sha>` for the deploy repo; the footer no longer errors.
- **`L` shows the failed step's log.** For the selected run of Main builds or Merge queue with
  a failed job known, `L` opens the details pane and requests
  `gh run view <run> --job <job> --log-failed -R <repo>`; the pane gains a `log` section with
  the last 40 lines (ANSI stripped, `job\tstep\t` prefixes dropped), scrollable with
  `Ctrl-d`/`Ctrl-u`. `L` again hides it. No failed job → footer `no failed job to show`.
  Read-only on purpose: `gh run rerun --failed` stays in the backlog (write action, confirm UX).
- **Keymap** (help fits a 16-row terminal: 13 rows + border + footer, so `d` and `C-d/C-u`
  share a row):

  | key | does |
  | --- | --- |
  | `q` | quit |
  | `j/k ↓/↑` | move |
  | `h/l Tab 1-4` | focus column |
  | `p` | open pull request |
  | `b` | open build |
  | `y/Y` | copy pull request / build URL |
  | `d C-d/C-u` | details / scroll them |
  | `L` | log of the failed step |
  | `z` | zoom the column |
  | `Esc` | close details / zoom / filter |
  | `/` | filter (Enter keeps, Esc clears) |
  | `r/R` | refresh focused / all |
  | `?` | help |

## Tasks

1. `Enter` commits the filter, `Esc` while editing cancels it, `/` reopens a live one; cursor
   in the footer while editing.
2. Every column title carries `visible/total` while a query is live.
3. `Esc` closes details, then zoom, then the live filter; `1`–`4` focus a column.
4. Help is context-sensitive to the focused column.
5. Deployed `b`/`Y` fall back to the commit URL.
6. `L`: the failed step's log in the details pane (`Github::log_failed`, `Request::Log`,
   `Input::Log`, `App.logs`), `text::strip_ansi`.
7. Version 0.13.0, README key table, AGENTS.md, PLAN.md outcome, `docs/details.png`.

## Verified facts (2026-09-14)

- `gh run view --job <id> --log-failed -R <repo>` exists in gh 2.100.0; lines are
  `<job>\t<step>\t<text>`.
- `KEYS` is 13 rows; the help view is `KEYS.len() + 2` rows tall.
- `tests/readme.rs` pins the README table to `KEYS`, so task 7 must touch both together.
