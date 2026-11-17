# Phase 58: the privacy scrub checks what is about to be committed (#120)

## Context

This repository is public, and `scripts/scrub-check.sh` is the guard of its first rule: no
employer data. It greps `git ls-files`, the files git already tracks, and `scripts/ship.sh`
runs it (through `scripts/gates.sh`) before `git add -A`. So a new file holding a listed term is
never seen by the scrub, then staged, committed and pushed. The check also fails open: a
denylist path that exists but is a directory or cannot be read prints `skipped` and exits 0,
exactly like a machine with no denylist.

## Verified facts (2026-11-17, `main` at 6512665)

- `scrub-check.sh` builds its file list with `git ls-files -z` and decides on `grep`'s exit
  status after `2>/dev/null`; it skips on `[ -f "$list" ]`, false for a missing path, a
  directory and a dangling symlink alike.
- `ship.sh` runs `cargo fmt`, `gates.sh`, the dev install, then `git add -A`, `git commit`,
  `git push` and, on the first push, `gh pr create`.
- `git add -A` stages every change of the working tree, deletions included, plus every untracked
  file that is not ignored.
- `git grep --untracked` searches the working tree's tracked and untracked files and leaves out
  the ignored ones; `git grep --cached` searches the index. Both take `-n -i -I -F -f <file>`,
  exit 0 on a match, 1 on none, and 128 on an error.
- `tests/scripts/ship.rs` drives `ship.sh` in a scratch repository with `gates.sh` replaced by
  the recorder; nothing tests `scrub-check.sh` itself, and `tests/scripts/scrub_check.rs` does
  not exist.
- CI does not run the scrub: the denylist lives only on the owner's machine.

## Decisions

- The scrub reads what `git add -A` and `git commit` would take: the working tree's tracked
  and untracked, not ignored, files, and the index. The gate run and the ship then see the same
  set, so `ship.sh` keeps `git add -A` and its order.
- Fail closed when a list was meant to be there: `CONVEYOR_DENYLIST` set to a path that is not
  a readable file, or the default path present but not a readable file (a directory, an
  unreadable file, a dangling symlink), exits 1 naming the path. A default path that does not
  exist at all still prints `skipped` and exits 0, for CI and contributors.
- An empty `CONVEYOR_DENYLIST` counts as unset, as before.
- A `git grep` that errors fails the check instead of reading as clean.

## States

| Denylist | Content | Result |
| --- | --- | --- |
| default path absent, variable unset | any | `skipped`, exit 0 |
| variable set, path absent or not a readable file | any | names the path, exit 1 |
| default path present, not a readable file | any | names the path, exit 1 |
| readable file | no listed term in index, tracked or untracked files | `clean`, exit 0 |
| readable file | a listed term in a tracked or untracked, not ignored, file | the matches, exit 1 |
| readable file | a listed term only in the index | the matches, exit 1 |
| readable file | a listed term only in an ignored file | `clean`, exit 0 |

## Tasks

1. This plan, its rows in `PLAN.md` and `plans/metrics.md`, and phase 57's row backfilled.
2. The scrub reads untracked and staged content: red first in `tests/scripts/ship.rs` (a ship
   with an untracked file holding a listed term commits and pushes nothing) and in a new
   `tests/scripts/scrub_check.rs`.
3. The scrub fails closed on a denylist that is there but unreadable, red first in
   `tests/scripts/scrub_check.rs`; root `AGENTS.md` says what the check covers.

## Outcome

## Hurdles
