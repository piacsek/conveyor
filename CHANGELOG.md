# Changelog

## v0.20.1 (2026-11-02)
- A merge, rerun or branch update that finishes while the changelog or help is open shows its result in that pane's footer.

## v0.20.0 (2026-11-02)
- `c` shows this changelog; `j`/`k` and `Ctrl-d`/`Ctrl-u` scroll it, any other key returns.

## v0.19.2 (2026-11-02)
- My PRs reloads once more 10 s after a rerun (`X`) from My PRs or a branch update (`U`) that changed the branch, so the card catches up with GitHub.

## v0.19.1 (2026-11-02)
- A run rerun from GitHub drops the previous attempt's jobs instead of mixing both.

## v0.19.0 (2026-11-02)
- `U` updates the selected pull request's branch after a y/n dialog; `prs.update_method` picks merge (default) or rebase.
- Confirm prompts open as a centred dialog over the whole body.
- `U` waits while a merge is in flight or about to refresh; `M` waits while an update is in flight.

## v0.18.0 (2026-11-02)
- `M` merges the selected pull request after a y/n prompt, pinned to the head commit on the card.

## v0.17.0 (2026-11-02)
- `X` reruns failed jobs after a y/n prompt: the newest main build, or the finished runs behind a pull request's failing checks.

## v0.16.2 (2026-11-02)
- Holding `r` or `R` no longer queues a fetch per key repeat; refreshes coalesce.

## v0.16.1 (2026-11-02)
- My PRs lists queued pull requests first, then open ones, then drafts.
- A queued pull request shows the merge queue icon.

## v0.16.0 (2026-11-01)
- Each check says how long it took, or how long it has been running.
- Jobs and runs in the details pane tick while running and keep their seconds.

## v0.15.1 (2026-11-01)
- A cancelled run's leftover job no longer fails the card once a newer run exists.

## v0.15.0 (2026-11-01)
- PR cards show the pull-request icon: green when open, grey when draft.

## v0.14.1 (2026-11-01)
- A PR's checks show once each, from the newest workflow run, across every page of checks.
- The card's check state follows those checks, not GitHub's rollup.
- A refresh keeps the jobs on screen, and a run that finishes unselected ends on its final jobs.

## v0.14.0 (2026-11-01)
- My PRs shows pull requests you authored or are assigned to; queries may use `OR`.

## v0.13.1 (2026-09-14)
- `L` opens the log in its own pane beside the details, resting at its last line.

## v0.13.0 (2026-09-14)
- `L` shows the failed step's log.
- `/` filters every column; `Enter` keeps the filter, `Esc` clears it, titles show visible/total.
- `Esc` closes details, then zoom, then the filter; `1`-`4` focus a column.
- The help names the focused column and says what `p` and `b` do there.
- Deployed `b` and `Y` fall back to the commit page when no fetched main build matches.

## v0.12.1 (2026-09-14)
- Only http(s) URLs are opened in the browser.

## v0.12.0 (2026-09-11)
- A build card's title gets a line of its own.
- A cancelled run no longer blames the jobs it killed.

## v0.11.0 (2026-09-11)
- A queue entry's merge-group run speaks for the entry; cancelled runs get the `⊘` glyph.
- The newest merge-group run wins.

## v0.10.0 (2026-09-11)
- Queue cards show their merge-group run and its jobs.

## v0.9.0 (2026-09-11)
- `Ctrl-d` and `Ctrl-u` scroll the details pane.

## v0.8.0 (2026-09-11)
- `p` opens the pull request and `b` the build; `Enter` does nothing.
- `d` opens the details pane (it was `p`); `Y` copies the build URL next to `y` for the pull request.
- Only the focused column highlights its selected card.
- The details pane carries no URLs.

## v0.7.0 (2026-09-10)
- `q` is the only quit key; `Esc` leaves zoom.

## v0.6.1 (2026-09-10)
- A missing pull request for a commit is retried after a while instead of being remembered forever.

## v0.6.0 (2026-09-10)
- `z` zooms the focused column to the full width.
- Deployed cards put one field per line, with the pull request title wrapped.
- Main builds stay newest first and the queue in position order on every refresh.

## v0.5.1 (2026-09-10)
- A Deployed row behind main shows a half circle, not the running dot.

## v0.5.0 (2026-09-10)
- Cards with a cyan selection bar replace numbered rows.
- Main builds show the failed job and step.
- `R` refreshes every column; `b` opens the build behind the selected card.

## v0.4.0 (2026-09-10)
- Deployed column: what each environment runs, read through kubectl.

## v0.3.0 (2026-09-10)
- Main builds column, with the pull request behind each run.
- The footer shows when the data was refreshed.

## v0.2.0 (2026-09-10)
- Merge queue column, with each entry's merge-group run.
- `[[repo]]` tables in the config.

## v0.1.0 (2026-09-10)
- My PRs column: checks, review and merge state, filter, details, open and copy.

## v0.0.1 (2026-09-10)
- `conveyor config` prints the effective config; `--help` and `--version`.
