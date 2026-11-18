# Phase 73: the README shows conveyor moving

## Context

Issue #97. The README shows one still screenshot, `docs/details.png`. A still does not show
what makes a TUI worth trying: moving focus across the columns, opening the details pane, the
`S` popup. A recording of 10 to 20 seconds does, generated from a tape committed to the
repository and recorded on the synthetic `acme/*` data of `conveyor --demo`.

## Decisions (the owner's, 2026-11-17)

- The recording goes right below `docs/details.png`, which stays where it is. The rule "every
  visible change to the main interface shows in `docs/details.png`" stays as it is.
- `tests/readme.rs` allows exactly that screenshot followed by exactly one recording, and the
  Documentation rule and the Layout of the root `AGENTS.md` say so.
- vhs may be installed with `brew install vhs`.

## Verified facts (2026-11-18, `main` at 5cae050)

- `brew install vhs` installs vhs 0.12.1 with ttyd 1.7.7 and ffmpeg 9.0.2. vhs drives a
  browser to render the terminal; it found and used the installed Google Chrome (no browser
  was downloaded into `~/.cache/rod`), which `scripts/screenshots.sh` already needs.
- vhs renders the FiraCode Nerd Font from `~/Library/Fonts` with `Set FontFamily "FiraCode
  Nerd Font"`: the Nerd Font icons and `✗ ⊘ ▸` draw, so the recording uses `icons = "nerd"`
  like `docs/details.png`, and the `Catppuccin Mocha` theme `freeze` uses.
- The binary is launched from the tape the way `screenshots.sh` launches it: `exec env -u
  XDG_CONFIG_HOME -u CONVEYOR_CONFIG -u XDG_STATE_HOME -u CONVEYOR_LOG HOME=<tempdir>
  target/release/conveyor --demo`, typed inside `Hide`, so no shell prompt is recorded and the
  footer shows the `demo` marker. vhs passes its environment to the shell it starts, so the
  script hands the tape the tempdir and the binary as `CONVEYOR_DEMO_HOME` and
  `CONVEYOR_DEMO_BIN`.
- GitHub draws a GIF referenced from a repository path in a README inline, like the PNG.
  GitHub's [attaching files](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files)
  page documents MP4, MOV and WebM only as uploads (a `user-attachments` URL, not a file in the
  repository) and says nothing of a video referenced by a repository path, so the recording is
  a GIF and regenerates with the repository.
- A trial at 1440x560 pixels, 12 frames a second: 17.5 s, 438 frames, 420 KB. Small enough
  that neither the frame rate nor the size needs lowering.
- No script under `scripts/` has a test for what it draws (`screenshots.sh` has none);
  `scripts/demo.sh` holds no logic beyond the same setup, so it gets none either.
- `scripts/` and `docs/` are on the list of `scripts/feeds-binary.sh`: neither the tape, the
  script nor the GIF feeds the binary, so the branch needs no fragment and releases nothing.

## Tasks

1. This plan, its row in `PLAN.md` and in `plans/metrics.md`.
2. `tests/readme.rs` demands `docs/demo.gif` right below the screenshot and the file itself
   (red), then `scripts/demo.tape`, `scripts/demo.sh`, the recording and its README line
   (green).
3. The root `AGENTS.md`: the Layout lines for the tape and the script, and the Documentation
   rule naming the recording and the one command that regenerates it.

## Outcome

- The README shows `docs/demo.gif` right below `docs/details.png`: 17 s at 12 frames a second,
  1440x560 pixels, 424 KB, the Nerd Font icons and the Catppuccin Mocha theme of the
  screenshot. It moves down My PRs, opens the details pane, walks the focus to Deployed, closes
  the pane, walks back and opens the `S` popup.
- `scripts/demo.sh` builds the release binary, writes a tempdir `HOME` with `icons = "nerd"`
  and runs `vhs scripts/demo.tape`; the tape launches `conveyor --demo` the way
  `screenshots.sh` does, hidden from the recording.
- `tests/readme.rs` allows exactly two images besides the banner (any `![` or `<img` in the
  prose counts), the screenshot then the recording two lines below it with one blank line
  between, each alone on its line, and fails while `scripts/demo.tape` does not write `docs/demo.gif` or the GIF
  is missing. The Documentation rule and the Layout of `AGENTS.md` name both.
- Nothing under `src/` changed: no fragment, no release.
- The times on screen come from the clock of the run: the footer reads the local time of the
  refresh (`refreshed at 22:34:56` in the first trial) and the demo ages its fixture from the
  current time, so every rerun changes the GIF. A run on the minute only looks like a fixed clock.
- The tape waits for `My PRs` on screen before it shows anything, so a binary that does not
  start fails `vhs` (exit 1, no GIF written) instead of recording a bare prompt.

## Hurdles

- `api-shape` GitHub's docs describe video only as an upload and say nothing of a video
  referenced by a repository path, so MP4 and WebM were not tried and the recording is a GIF → none
- `review-loop` the README image guard counted only lines starting with `![`, so an `<img>`, an
  indented or linked image, or a second image on the screenshot's line passed → code
- `tooling` vhs records whatever the shell shows, so a binary that failed to start produced a
  GIF of a bare prompt and a clean exit until the tape waited for the demo screen → code
