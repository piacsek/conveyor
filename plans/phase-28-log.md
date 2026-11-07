# Phase 28: a log file

## Context
Conveyor keeps no record of what it did. An error in the footer is gone once the next notice
replaces it, and nothing says which `gh` or `kubectl` call failed, how long it took, or what it
printed. A log file answers "why did that column go red at 14:02" after the fact.

## Decisions
- **Always on, no config key.** A log nobody enabled is the one that is missing when it is needed.
- **Path:** `$XDG_STATE_HOME/conveyor/<program>.log`, else `~/.local/state/conveyor/<program>.log`,
  where `<program>` is the file name the binary was started as (`argv[0]`): `conveyor` and
  `conveyor-dev` running side by side each keep their own log, and neither one's start rotates
  the other's file away. `CONVEYOR_LOG` overrides both (tests, and anyone who wants it
  elsewhere). The directory is created on start. Two copies of the *same* program do not share
  cleanly: the second start rotates the first one's live file to `.log.1`, and a third start
  replaces it, so the first copy's later lines are lost. Accepted for a diagnostics file.
- **Retention:** on start, an existing `conveyor.log` is renamed to `conveyor.log.1` (replacing
  the old one). Two sessions on disk, never unbounded growth, and a restart after a bad session
  keeps the evidence.
- **What one line holds:** local date and time; the program (`gh`/`kubectl`), its argv, the
  duration, and `ok` or `exit <code>: <stderr>`. Stdout is never logged (megabytes of JSON). A
  GraphQL `query=` argument is logged as `query=<N chars>`: the documents are anonymous and
  multi-line, the variables beside them already say what was asked.
  Example:
  `2026-11-06 14:02:11 gh api graphql -f query=<812 chars> -F owner=acme -F name=api 412ms exit 1: HTTP 502`
- **Also logged:** start (version, config path, config error if any) and quit.
- **Never breaks the TUI.** A log that cannot be opened or written is a no-op; the app runs as
  before. No footer notice: the log is a diagnostic, not a feature.
- **`conveyor log` prints the path,** so `tail -f "$(conveyor log)"` works without knowing XDG.
  It exits 0 before the TUI starts, like `config`.
- **The repo stays public:** the log lives under `$HOME`, never in the tree. Tests write it to a
  tempdir through `CONVEYOR_LOG`, and the e2e/screenshot launchers add `-u CONVEYOR_LOG` to their
  `env` line so a developer's export cannot leak into them.

## Seam
A `Log` value (`src/log.rs`): `Log::open(path) -> Log` (a disabled `Log` on any error), cheap to
clone (`Arc<Mutex<File>>` inside, one `write_all` per line), and `line(&self, &str)`. `CliGh` and
`CliKubectl` take one and time each `Command::output`. `main` opens it once and hands clones to
every fetcher and worker thread.

## Tasks (each starts red)
1. `log::path(CONVEYOR_LOG, XDG_STATE_HOME, HOME, program)` and `log::program_name(argv[0])`: precedence, unit tests next to `config::path`'s.
2. `Log::open` rotates `.log` to `.log.1` and creates the directory; an unwritable path yields a
   disabled `Log` that drops lines. Tests in a tempdir.
3. `conveyor log` prints the path and exits 0; `--help` lists it. `tests/cli.rs`.
4. `CliGh::run` logs one line per call: argv with `query=<N chars>`, duration, `ok` or the exit
   and stderr. `tests/github.rs` with the shim (takes `SHIMS`).
5. `CliKubectl::image` logs the same way. `tests/kube.rs` (takes its mutex).
6. `main` opens the log, writes start/quit lines and wires it into every `CliGh`/`CliKubectl`.
   e2e: run the binary with `CONVEYOR_LOG` in a tempdir, quit, assert the start line and one
   `gh` line. Add `-u CONVEYOR_LOG` to the e2e and `screenshots.sh` launchers.
7. Docs and release: README `## Usage` gets `conveyor log`; AGENTS.md layout gains `src/log.rs`
   and a bullet on the log; `PLAN.md` outcome; v0.22.0 with its `CHANGELOG.md` entry.

## Verification
`scripts/gates.sh`; `conveyor-dev`, press `r` a few times, `q`, then `tail "$(conveyor-dev log)"`
shows the start line, one line per `gh` call with a duration, and the quit line. A second launch
leaves the first session in `conveyor.log.1`.

## Risks
- A slow disk adds latency to each fetch: one short append per call, measured in the e2e run.
- `query=<N chars>` hides which document ran when two share variables; acceptable until it bites.
