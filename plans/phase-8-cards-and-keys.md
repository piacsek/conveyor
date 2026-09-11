# Phase 8 — one highlight, a quiet details pane, and a keymap for PR *and* build

## Why

Four things the user asked for after living in v0.7.0:

1. Every column drew a highlight on its own selected row, so four cards looked
   selected at once. Only the focused column's selection should read as selected.
2. The details pane repeated a full URL on nearly every line; they are unreadable
   at column width and nobody types them.
3. `Enter` opening a browser tab was surprising. It becomes a no-op until the
   open UX is revisited.
4. Opening the build and opening the pull request are both wanted from any card;
   `o` alone did not say which one it meant.

## Keymap

Mnemonic pairs, PR and build side by side:

| key | does |
| --- | --- |
| `p` | open the pull request |
| `b` | open the build |
| `y` | copy the pull request URL |
| `Y` | copy the build URL |
| `d` | details pane (was `p`) |
| `Enter`, `o` | nothing |

`KEYS` stays at 13 lines (`y/Y` share one) so the help pane still fits a 16-row
terminal.

## Tasks

1. Highlight only the focused column's selected card.
2. Drop every URL from the details pane (checks, queue entry and run, build and
   its pull, deployment, jobs).
3. `Enter` is a no-op.
4. The keymap above, plus help, README, AGENTS and screenshots.
