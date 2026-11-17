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
| default path under a directory without search permission, or under a file | any | names the path, exit 1 |
| readable file | no listed term in index, tracked or untracked files | `clean`, exit 0 |
| readable file | a listed term in a tracked or untracked, not ignored, file | the matches, exit 1 |
| readable file | a listed term only in the index | the matches, exit 1 |
| readable file | a listed term only in an ignored file | `clean`, exit 0 |
| readable file | a listed term in the path of a tracked, staged or untracked, not ignored, file | the paths, exit 1 |
| readable file | a listed term only in the path of an ignored file | `clean`, exit 0 |
| empty file | any | says it is empty, exit 1, in both modes |
| only blank lines | any | `git grep` fails, exit 1 |
| readable file | a non-ASCII listed name, renamed away before HEAD by an outgoing commit | `--outgoing` names the unquoted path, exit 1 |
| readable file | a listed term in the message of an outgoing merge | not pushed; `ship.sh` says to keep the merge and `git commit --amend` |
| any | nothing staged, commits no remote has | `ship.sh` makes no commit and pushes them as they are |
| readable file | a listed term in a file written after the gates ran, before `git add -A` | `ship.sh` scrubs again after `git add -A`: nothing committed or pushed, the files stay staged |
| readable file | a listed term only in the commit message | committed, not pushed, no PR opened; `--outgoing` names the commit and `message`; `ship.sh` says to `git reset --soft HEAD~1` and ship again |
| readable file | a listed term in the branch name | committed, not pushed; `--outgoing` says `branch name`; `ship.sh` says to reset and `git branch -m` |
| readable file | a listed term in an earlier outgoing commit, not in HEAD | not pushed; `--outgoing` names that commit without `(HEAD)`; `ship.sh` says it has to be rewritten by hand |
| readable file | a listed term added and removed again by commits no remote has | committed, not pushed |
| readable file | a symlink whose target holds a listed term, in an outgoing commit | `--outgoing` names the target line, exit 1 |
| readable file | a listed term only in commits a remote already has | `--outgoing` reads `clean`, exit 0 |
| readable file | an outgoing commit deletes a file, or a line, holding a listed term a remote has | `clean`, exit 0: only added lines are read |
| readable file | an outgoing commit deletes a file whose name holds a listed term a remote has | `clean`, exit 0: deleted paths are not read |
| readable file | an outgoing added line, added path or rename to a listed name | names the commit and `added line` or `path`, exit 1 |
| readable file | a merge commit whose conflict resolution adds a listed term | names the merge and `added line`, exit 1 |
| readable file | a listed term that equals part of a blob hash in an `index` line | `clean`, exit 0: `index` lines are not read |
| readable file | `git log` cannot run (no commit yet) | `--outgoing` says `git log failed`, exit 1 |

## Tasks

1. This plan, its rows in `PLAN.md` and `plans/metrics.md`, and phase 57's row backfilled.
2. The scrub reads untracked and staged content: red first in `tests/scripts/ship.rs` (a ship
   with an untracked file holding a listed term commits and pushes nothing) and in a new
   `tests/scripts/scrub_check.rs`.
3. The scrub fails closed on a denylist that is there but unreadable, red first in
   `tests/scripts/scrub_check.rs`; root `AGENTS.md` says what the check covers.

## Outcome

- `scripts/scrub-check.sh` scrubs what `ship.sh` publishes three times: as the first gate and
  again after `git add -A` it reads the contents and paths of the index and of the tracked and
  untracked files that are not ignored; between `git commit` and `git push`, `--outgoing` reads
  the branch name and, per commit no remote has, its message, the paths it adds or keeps and the
  lines it adds (merge resolutions and symlink targets included), never removed lines or
  deleted paths. Each match names the stream and the commit.
- Redesigned after round 2 (two strikes): scrubbing the tree before the commit could not see the
  message, so the check moved to what the push adds, read with `git diff-tree --cc`, not
  `git log` (which walks past a deletion-only HEAD).
- The denylist resolution fails closed: the default path is skipped only when its absence is
  confirmed by walking up to a searchable directory; an empty list fails in both modes.
- A refused push keeps the commit local with advice that works: `git reset --soft HEAD~1` for a
  plain commit, `git commit --amend` for a merge, after which `ship.sh` pushes the unpushed
  commits without a new one when nothing is staged.
- Not covered, by decision: binary contents, author and committer, the PR body, pushes without
  `ship.sh`. A rename is read as all lines added, so moving a file whose contents hold a term
  `main` already has is refused.

## Hurdles

- `review-loop` round 1 found the gap between the gates and the commit, and paths the scrub never read → code
- `review-loop` round 2 found the commit message unread, which led to the redesign that scrubs what is pushed → code
- `review-loop` round 3 found the redesign reading removed lines and its reset advice not working → code
- `review-loop` round 4 was clean; its four minor findings were fixed after it → code
- `review-loop` the unsearchable parent of the default denylist was declined in three rounds and fixed on the fourth → code
- `tooling` the first ship was piped through tail, against the rule → none
- `process` code was written before its test and backed out to see the test red → none
- `tooling` a sibling agent overwrote the PR body file in the shared scratchpad; files there now carry the phase in their name → none
