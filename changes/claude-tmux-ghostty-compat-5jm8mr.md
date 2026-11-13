kind: fix

- Inside tmux, when the terminal would never receive the clickable `#N` links (tmux turns them on for iTerm2 but, before tmux 3.8, not for Ghostty), the footer says so at start and gives the `set -as terminal-features ',<your TERM>:hyperlinks'` line to add to `tmux.conf`. A tmux older than 3.4 is told it cannot pass the links at all.
