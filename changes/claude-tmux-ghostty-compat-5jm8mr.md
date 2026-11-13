kind: fix

- Inside tmux, when the terminal would never receive the clickable `#N` links (tmux passes them to iTerm2, but to Ghostty only from tmux 3.8), the footer says so at start and `?` shows the `tmux set -as terminal-features ',<your TERM>:hyperlinks'` command to run before reattaching, and the line to keep in `tmux.conf`. A tmux older than 3.4 is told it cannot pass the links at all.
