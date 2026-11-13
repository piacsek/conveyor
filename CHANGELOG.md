# Changelog

## v0.26.1 (2026-11-13)
- v0.26.1: internal changes only.

## v0.26.0 (2026-11-12)
- A failed action (merge, update, status change, checkout or rerun) leaves a red `! <action> failed: <reason>` line under the title of the card it was started from, so a later result can no longer wipe it from the footer before you read it. An update that hit conflicts and a rerun where any run failed count as failures.
- `d` on a marked card starts with the whole reason and the time it landed.
- The column title counts the marked cards (`My PRs (5) !1`), in the tabs of a narrow terminal too.
- A mark goes away when you start a new action on that card, press `Esc` with the card selected (after the details pane and zoom, before the filter), or when the card leaves its column.

## v0.25.0 (2026-11-12)
- `S` works on another pull request while an action on the first one is still running; only the pull request with an action on its way refuses it.
- A pull request card with an action on its way shows a spinner in place of its check mark and names the action (`marking ready…`, `merging…`) on its second line until the result lands.

## v0.24.2 (2026-11-12)
- Main builds appears much sooner on the first load: the pull requests of all its runs are asked for in one request instead of one request per run (about 4 s instead of 27 s for 30 builds).
- The Deployed column asks for the pull requests of all its environments in one request as well.

## v0.24.1 (2026-11-11)
- In the `S` popup of My PRs the close action reads `x  close PR` instead of `x  close`.

## v0.24.0 (2026-11-10)
- `S` then `c` checks the selected pull request out locally with `gh pr checkout`. The footer says where it landed, or git's reason when it could not (uncommitted changes, a branch already open in another worktree).
- The checkout runs in the `path` set under the repository's `[[repo]]`, a new optional key, and otherwise in the directory conveyor was started in.
- A checkout is refused when that directory is not a clone of the pull request's repository, instead of fetching the pull request into it.

## v0.23.2 (2026-11-09)
- The Deployed column asks all its environments at the same time, so one that does not answer no longer makes the others wait their turn behind it.
- A `gh` or `kubectl` call that hangs can no longer hold up another call that happened to start at the same moment (macOS).

## v0.23.1 (2026-11-09)
- The Deployed column no longer sits on `fetching…` for 40 seconds when a cluster stops answering: kubectl is cut off after 15 seconds and the row says `kubectl did not answer within 15s`.
- A Deployed row that failed shows kubectl's reason (for example `error: You must be logged in to the server`) instead of an internal log line.

## v0.23.0 (2026-11-09)
- A Deployed card shows a yellow `⚠` when a pod is stuck (CrashLoopBackOff, an image that cannot be pulled, no node to run on) or the rollout stalled while the app still serves, with the reason on the card.
- A Deployed card shows a red `‼` when nothing is serving (no pod of the deployment is available), and a yellow `⚠` when it runs short of pods but still serves.
- A Deployed card shows a grey `?` when its pods could not be read, so a missing permission never looks like a healthy app.
- `d` on a Deployed card lists the kube context, namespace and deployment, the replica counts, the failing conditions and every pod with its status, restarts, age, version and last exit reason.

## v0.22.0 (2026-11-06)
- Conveyor writes a log: one line per `gh` and `kubectl` call with its arguments, how long it took and how it ended, plus when the TUI started and quit.
- `conveyor log` prints where the log is (`~/.local/state/conveyor/conveyor.log`); `conveyor-dev` keeps its own `conveyor-dev.log`.
- Each start keeps the previous session's log as `.log.1`.

## v0.21.2 (2026-11-06)
- On a pull request whose base has a merge queue, `S` then `m` reads `add to merge queue` and adds it the way GitHub's "Merge when ready" does, so it works on repositories with auto-merge turned off.

## v0.21.1 (2026-11-03)
- The `S` menu no longer ends with a hint line; it lists the actions and nothing else.

## v0.21.0 (2026-11-03)
- `S` opens one popup with everything you can do to the selected pull request or main build; actions that do not apply are listed with the reason.
- `X`, `M` and `U` are gone: rerun failed jobs is `S f`, merge is `S m`, update the branch is `S u`.
- From `S` you can also mark a pull request ready or convert it to draft (`t`), turn auto-merge on or off (`a`), take it out of the merge queue (`e`) and close it (`x`).
- Only merge, enabling auto-merge, close and a rebase ask for `y`; the other actions run as soon as you press their letter.
- A pull request card says `auto-merge` while auto-merge is on.

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
