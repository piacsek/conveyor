# Phase 49: `conveyor --demo`

## Context
Issue #96, one of three phases (49 demo, 50 plain icons, 51 Nix flake). Anyone should be able
to try conveyor with no `gh` auth, no `kubectl` and no config: `conveyor --demo` draws all four
columns from an invented `acme/*` fixture built into the binary. Today the only such fixture
lives in `scripts/screenshots.sh` as shell-written JSON behind `gh` and `kubectl` shims.

## Verified facts (2026-11-16, read in the code at `2d78aa0`)
- `src/main.rs` builds `CliGh::default().with_log(..)` once per fetcher and once per worker
  (eleven places) and `CliKubectl` once; every consumer takes `impl Github` / `impl Kube + Sync`.
  `run()` takes the opener as `&impl Opener`.
- `cli::parse` reads the first argument only; `--help` and `--version` return before the
  config is read.
- Writes go through `Github::{merge, update_branch, change_status, checkout, rerun_failed}`;
  an `Err(Failure)` from any of them already marks the card (`app::actions::failures`) and
  puts the message in the footer. `Failure::from(String)` is kind `Other`.
- `run()` turns an `Err` from `Opener::open` or `Opener::copy` into the footer notice
  (`attempt`).
- The footer's right side is the static `ui::logo()`; `tests/snapshots.rs` swaps `logo()` out.
- `fetch::Pulls::prefetch` asks a batched `repository(name:` GraphQL query; an answer without
  the commits leaves each sha to REST `commits/{sha}/pulls`, which is how the screenshot shim
  serves pull requests today.
- `log::path` returning `None` leaves the log off (`Log::default()`), and `Log::open` rotates
  the previous file.
- `scripts/screenshots.sh` fixture: PRs 4821/4790 (`acme/webapp`) and 4830 (`acme/auth`), the
  queue from `tests/fixtures/queue.json`, main runs 309 to 312, merge-group runs 313/314, jobs
  for 309 to 313, pull requests by sha, Deployments `dev`/`staging`/`prod` (`staging` behind,
  with a crash-looping pod) and `uat` failing.

## Design
- `cli::Command::Demo` for `--demo`, in `USAGE`.
- `src/sources/demo.rs`: `Demo` implements `Github` and `Kube`. It answers reads from JSON
  under `src/sources/demo/` (`include_str!`), where a string `ago:<secs>` becomes the time that
  many seconds before the call, so ages stay alive across refreshes. It spawns nothing and logs
  nothing. Every write answers `Err("demo: nothing was changed")`.
- `demo::config(config)` keeps the user's config and replaces its `[prs]` and `[[repo]]` with
  the demo's (two tabs, `acme/webapp` with four environments, `acme/infra`).
- `main::tui` takes the sources as factories, so the demo and the real run share one path.
  The demo runs with `Log::default()`: no file is opened or rotated.
- `open::DemoOpener`: `open` refuses with a footer notice, `copy` copies as usual.
- `App::demo` makes the footer show `demo` in front of the logo.
- `scripts/screenshots.sh` runs `--demo` instead of its shims.

## States

| Action in demo | What happens | What the user sees |
| --- | --- | --- |
| refresh (timer or `r`/`R`) | fixture answered again, times relative to now | same rows, fresh ages |
| jobs of a run (`d`) | fixture: two jobs, the run's own status | job list |
| failed step's log (`L`) | short invented log | log pane |
| merge, update branch, ready/draft, close, auto-merge, leave queue, rerun | no call; `Err("demo: nothing was changed")` | the failed-write mark on the card, the message in the footer |
| check out (`c` in `S`) | no call; same error | same mark |
| open in browser (`p`, `b`, `o`-less keys) | nothing spawned; `Opener::open` errs | footer `demo: acme/* is made up, so nothing was opened` |
| a card's `#N` | the demo draws no OSC 8 link | the number as plain text; nothing to click |
| copy (`y`/`Y`) | copies as usual | as usual |
| `review` tab | its own two pull requests, by other authors | two cards, none of them mine |
| help or changelog pane open | the footer keeps the marker | `demo` at the bottom right |
| a sha with no pull request | REST `[]`, then the `(#N)` route answers nothing | the row without a pull request, as today |
| `uat` environment | `Kube` errs like an expired session | `✗` row with the message |
| config file missing | default config, then the demo tables | demo |
| config file broken | default config, error in the footer, as without `--demo` | demo with the notice |
| `[ui]` set in the config | kept | the user's titles, widths and keys |

## Tasks
1. `conveyor --demo` draws My PRs from the built-in fixture with no `gh` on `PATH` (e2e), via
   `Command::Demo`, `Demo` as `Github`, and `tui` taking its sources.
2. Merge queue and Main builds from the fixture (runs, jobs, pull requests by sha).
3. Deployed from the fixture (`Demo` as `Kube`), `uat` failing.
4. Writes answer `demo: nothing was changed`; the failed step's log has text.
5. The demo replaces `[prs]` and `[[repo]]` and keeps the rest of the config.
6. Opening a URL in the demo refuses with a notice; the footer says `demo`.
7. `scripts/screenshots.sh` runs `--demo`; `docs/details.png` rerendered.
8. README, `AGENTS.md` files, fragment.
