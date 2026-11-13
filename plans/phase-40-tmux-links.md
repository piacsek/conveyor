# Phase 40: say why the `#N` links do nothing inside tmux

## Context
Issue #76. Since #69 the `#N` of a card is an OSC 8 hyperlink. Inside tmux the outer terminal
only sees what tmux redraws, and tmux passes a hyperlink on only when the attached client has the
`hyperlinks` terminal feature. tmux turns that feature on by itself for iTerm2 from 3.4, foot
from 3.7, WezTerm and Ghostty from 3.8 (kitty only on master). So the same conveyor links
in tmux + iTerm2 and does nothing in tmux + Ghostty, and nothing on screen says why.

## Design
At start, inside tmux (`$TMUX` set), conveyor asks tmux once for the attached client's terminal
name, the tmux version and the client's features, and when the links would be dropped it says so
in the footer notice, the place a config error already uses at start (a config error wins).
The notice is short and points to the help pane, which shows the fix under the keys; the call
log keeps the same lines:

```text
footer: tmux drops the #N links: ? says how to keep them

? pane: tmux drops the #N links for this terminal. If it shows links, run
          tmux set -as terminal-features ',xterm-ghostty:hyperlinks'
        then detach and reattach; add that line to tmux.conf to keep it.
   or:  tmux 3.3 drops the #N links: tmux 3.4 or later passes them through
```

- The line names the client's own `TERM`, so it is right for any terminal, not only Ghostty.
- No notice when: not in tmux, no client attached (the answer is empty), the client already
  has `hyperlinks`, tmux cannot be run or does not answer within a second (the whole call,
  reading included, spawned under `sources::spawning()`).
- Revised after review round 1: the first wording said to add the line to `tmux.conf` and
  reattach, which does nothing (tmux reads `tmux.conf` only when its server starts), and a
  footer line of over 100 columns was cut on narrow terminals and gone after one key.
- conveyor does not change the tmux server's options itself: tmux reads `terminal-features`
  only when a client attaches, so it would not help the running session, and it would change
  every other program's terminal behind the user's back.
- Not chosen: the DCS passthrough (`ESC Ptmux;`), which needs `allow-passthrough on` (off by
  default) and loses the link on every tmux redraw.

## Verified facts (2026-11-13, tmux 3.4 here, tmux source at 3.8)
- tmux 3.4 with defaults drops OSC 8 on the way out; with
  `set -as terminal-features ',xterm-256color:hyperlinks'` it writes `ESC]8;id=tmux1;<url>`.
  Set at runtime, it applies to the next client that attaches (`tty_term_create` reads the
  option on attach only).
- `tmux display -p '#{client_termname}\t#{version}\t#{client_termfeatures}'` from a pane:
  `xterm-256color\t3.4\tbpaste,ccolour,clipboard,hyperlinks,…` with a client attached,
  `\t3.4\t` with none (the e2e servers are detached).
- `tty-features.c` at the release tags: `hyperlinks` in the defaults of iTerm2 from 3.4, foot
  from 3.7, WezTerm and `ghostty` from 3.8 (f8674cc9), kitty on master only. `hyperlinks` itself is new in 3.4 (cdacc12c).
- The startup notice: `main.rs` sets `app.notice = config_error` before `run`; the footer
  shows `App::notice` until the next key.

## Tasks (each starts red, each shipped with `scripts/ship.sh`)
1. e2e: inside tmux, a client without `hyperlinks` gets the notice naming its `TERM`; with
   `hyperlinks` it gets none (a `tmux` shim on the pane's `PATH`, like the `gh` one).
2. Pure rules in `src/tmux.rs`: no client, an old tmux, an unparsable version, a `next-`
   build from master.
3. Docs: `AGENTS.md` layout, `src/ui/AGENTS.md` link bullet, fragment.
4. Review round 1: the fix in the help pane and the log, the footer only pointing there; a
   `TERM` that would break the quoted command is left out; the spawn lock and a deadline over
   the read (e2e: a tmux that hangs, a tmux that leaves its output open); `tests/e2e.rs`
   becomes a folder at the 700-line limit.
