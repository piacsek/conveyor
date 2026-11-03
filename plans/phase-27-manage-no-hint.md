# Phase 27: the `S` popup without its hint line

## Context
The user found the dim `a letter acts · any other key cancels` line under the `S` popup's actions to be noise: the letters and the dim reasons already say how it works.

## Tasks (each starts red)
1. `draw_manage` renders the offers only, no blank line and no hint; the popup is two rows shorter. Test in `tests/manage.rs`, snapshot `manage_layout` updated.
2. Release v0.21.1: bump, `CHANGELOG.md` entry, snapshots' version.

## Verification
`scripts/gates.sh`; `conveyor-dev`, `S` on a card: the popup ends at `x  close`.
