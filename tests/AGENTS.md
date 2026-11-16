# tests

What has gone wrong in the tests before, and how the suites are set up to avoid it. Rules and working agreements are in the root `AGENTS.md`.

## Testing traps hit so far

- tmux is a test harness, not a dependency: only the `#[ignore]` e2e tests use it, they panic
  with an install hint when it is missing, and `gates.sh` skips them without tmux (CI has it).
- `tests/kube.rs` shim tests take the same kind of mutex as `tests/github/` (fork race).
- **The spawn-race tests are red only some of the time, and only on macOS.**
  `a_call_made_while_another_hangs_…` in `tests/kube.rs` and `tests/github/` start 150 rounds
  of four calls at once. Without the spawn lock the kubectl one failed in about half its runs
  at 60 rounds and the `gh` one in every run; on Linux both always pass. One green run of
  either is no proof the lock is still in place.
- **A shim's first run is slow, and a killed shell talks.** Measured on macOS 2026-11-11: the
  first exec of a freshly written script took 160 to 220 ms at the median on a busy machine
  (p99 285 to 603 ms, one of 2.4 s) against 7 ms for a file that had run before, and every
  shim test writes a new file. A 300 ms deadline was therefore often over before the shim's
  first line, and `tests/kube.rs` failed in 14 of 266 runs under load. A test that needs the
  deadline to cut a running shim goes through `time_out_once_ready`: the shim writes
  `$0.ready` before it hangs, and the call is repeated with a doubled deadline until that file
  is there. A test whose shim exits on its own takes a long deadline instead. A shim that
  hangs ends in `exec sleep`: killing the group kills `sleep` and `sh` together, and a `sh`
  that outlived its child by a moment wrote `Killed: 9 sleep 5` to stderr, which then showed
  up in the logged line.
- `tests/e2e/` runs the real binary in a scratch tmux server (`-L`, `-f /dev/null`) with
  `HOME` in a tempdir and a `gh` shell shim that prints `tests/fixtures/prs.json`. The pane
  command must be `env HOME=… PATH=… <binary>`: `respawn-pane -e PATH=…` looked right but
  the pane's login shell rebuilt `PATH` (macOS `path_helper` and rc files), the shim was
  never found and the real `gh` ran. Sockets are per test so parallel tests never share a
  server.
- **A scratch `HOME` does not isolate the config or the log.** `XDG_CONFIG_HOME`,
  `CONVEYOR_CONFIG`, `XDG_STATE_HOME` and `CONVEYOR_LOG` outrank `HOME`, and a shell that exports one made the e2e panes and `screenshots.sh` read the
  real config (real repo names on screen, three e2e failures that CI never saw). Both launch the
  binary with `env -u XDG_CONFIG_HOME -u CONVEYOR_CONFIG -u XDG_STATE_HOME -u CONVEYOR_LOG …`; any new launcher must do the same.
- **`gh` shim tests share one process, so they race on fork.** `tests/github/` writes a
  shell script per test and spawns it; on Linux CI one test's `fs::write` was still open when
  another test's `Command` forked, the child inherited the fd and `exec` failed with
  `ExecutableFileBusy` ("Text file busy"). It never showed on macOS and passed on the PR run,
  then failed on `main` after the rebase merge. Every test that creates a shim now takes the
  `SHIMS` mutex for its whole body. Any new test that writes an executable and runs it in the
  same test binary must do the same (or run it in a separate process such as tmux, as e2e
  does). Treat a green PR run as no proof against this class: it is timing-dependent.
  `tests/scripts/` needs no mutex because it writes no executable: every shim is a symlink
  to a tracked script, and the scripts under test are symlinked into the scratch directory.
- **A scratch repository's `scripts/` are symlinks to the real scripts.** `link_script` links
  them, so a test that writes to one of those paths (`committed(&work, "scripts/<name>.sh", …)`)
  writes through the link and overwrites the tracked script in the worktree, and every later
  test runs the overwritten one. A test that needs a changed file under `scripts/` uses a name
  no test links.
- A card's first line in a 40-column pane holds about 30 characters after the bar, glyph and
  the right-aligned figure; a build card's title gets a line of its own and about `width - 4`.
  Card tests read line groups, not one line per row: a build card is four lines, so row `n`
  starts at screen line `1 + 4n` and a test that wants a third card needs a terminal taller
  than the 12-row default. The helper that finds the selected card looks for `│▌`.
- **`Harness::screen()` is the text a person reads, not the cells.** A linked `#N` holds an
  OSC 8 escape in each of its cells (see `src/ui/AGENTS.md`); `screen()` drops the escapes and
  keeps the character, so text assertions and snapshots never see them. Assert on links with
  `Harness::links()`, which returns `(text, url)` per link, column by column; `h.cell(x, y)`
  still returns the raw cell, escape included.
- **Snapshots carry `vX.Y.Z`, not the crate version.** Every snapshot in `tests/snapshots.rs`
  goes through `versionless(h.screen())`, which swaps the footer logo for a placeholder and
  pads the line back to the terminal width, so a release bump touches no `.snap` file.
  `no_snapshot_carries_the_crate_version` fails for a new snapshot taken from a bare
  `h.screen()`. The real version on screen is asserted in `tests/prs/footer.rs`.
- **The `gh` shim in e2e and screenshots dispatches on the query text.** Match the queue query
  on `repository(owner`, not `mergeQueue`: the PR query also contains `mergeQueueEntry`.
  The batched pull request lookup is written `repository(name:` so it misses that route and
  falls to the default one, which answers no commit: the shims keep serving pull requests
  through the REST routes.
- **Tests build model structs through `tests/support/`, never with a full literal.** A struct
  from `src/model/`, `src/config.rs` or `src/app/` comes from a `support` builder (then mutated
  field by field) or from a literal ending in `..Default::default()`, so a new field costs one
  edit in `tests/support/mod.rs` instead of one per test binary.
