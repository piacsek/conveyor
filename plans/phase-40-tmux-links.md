# Phase 40: say why the `#N` links do nothing inside tmux

## Context
Issue #76. Since #69 the `#N` of a card is an OSC 8 hyperlink. Inside tmux the outer terminal
only sees what tmux redraws, and tmux passes a hyperlink on only when the attached client has the
`hyperlinks` terminal feature. tmux 3.4 to 3.7 turn that feature on by themselves for iTerm2
(and foot, WezTerm, kitty), not for Ghostty; tmux 3.8 adds Ghostty. So the same conveyor links
in tmux + iTerm2 and does nothing in tmux + Ghostty, and nothing on screen says why.

## Design
At start, inside tmux (`$TMUX` set), conveyor asks tmux once for the attached client's terminal
name, the tmux version and the client's features, and when the links would be dropped it says so
in the footer notice, the place a config error already uses at start (a config error wins):

```text
tmux drops the #N links: add set -as terminal-features ',xterm-ghostty:hyperlinks' to tmux.conf and reattach
tmux 3.3 drops the #N links: tmux 3.4 or later passes them through
```

- The line names the client's own `TERM`, so it is right for any terminal, not only Ghostty.
- No notice when: not in tmux, no client attached (the answer is empty), the client already
  has `hyperlinks`, tmux cannot be run or does not answer within a second.
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
- `tty-features.c`: the `iTerm2` default features carry `hyperlinks` since 3.4; `ghostty` is
  listed from 3.8 (f8674cc9). `hyperlinks` itself is new in 3.4 (cdacc12c).
- The startup notice: `main.rs` sets `app.notice = config_error` before `run`; the footer
  shows `App::notice` until the next key.

## Tasks (each starts red, each shipped with `scripts/ship.sh`)
1. e2e: inside tmux, a client without `hyperlinks` gets the notice naming its `TERM`; with
   `hyperlinks` it gets none (a `tmux` shim on the pane's `PATH`, like the `gh` one).
2. Pure rules in `src/tmux.rs`: no client, an old tmux, an unparsable version, a config error
   winning.
3. Docs: `AGENTS.md` layout, `src/ui/AGENTS.md` link bullet, fragment.
