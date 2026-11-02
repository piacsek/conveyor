# Phase 21: `U` updates the selected PR's branch with its base

## Context

A PR that is `behind` its base costs a browser trip to click "Update branch". `U` does it from
My PRs, after a y/n prompt like `X` and `M`. When the base cannot be merged in cleanly, the
footer says the PR has conflicts instead of a raw `gh` error.

User decisions (2026-11-02): key is `U` (capital, like `X`/`M`); always asks y/n; how the base
comes in is a config key `prs.update_method` = `merge` (default, no force-push) | `rebase`
(`--rebase`, force-pushes the head branch).

Builds on phase 20 (`M`, PR #31): `Mode::Confirm(Pending)` with `Pending::{Rerun, Merge}`,
`PullRequest::head_oid`, `KEYS` at 14.

## Verified facts

- `gh pr update-branch [<number>] -R <owner/repo>` (gh 2.103.3): merges the base in by default.
- Its messages (strings in the gh binary): `%s PR branch updated`,
  `%s PR branch already up-to-date`, `%s Cannot update PR branch due to conflicts`.
  **To verify in task 1:** which stream each lands on and the exit code on conflict
  (expected: stderr, exit 1). `CliGh::run` drops stderr on success, so "already up to date" is
  invisible unless the method reads it.
- `MergeState::Dirty` (`src/model/prs.rs`) = GitHub already knows the PR conflicts; often
  `Unknown` instead (computed lazily), so the gh error is still the main conflict signal.
- Pushing to a queued PR's branch drops it from the merge queue.

## Tasks (one commit each via `scripts/ship.sh`, red test first)

0a. **Confirm becomes a centred dialog, for every write action (`X`, `M`, `U`).** New
   `src/ui/dialog.rs`: `draw_confirm(frame, area, &Pending)` renders over the body with
   ratatui `Clear` + `Block::bordered()`, centred (`Layout` with `Flex::Center`, about 50
   columns wide, clamped to the terminal), title `Confirm`, the question on one wrapped line
   (`rerun failed jobs of run 431?`, `merge #812 <title>?`, `update branch of #812 <title>?`),
   then `y confirm · any other key cancels` in `dim()`. `ui::draw` calls it last while
   `Mode::Confirm`; `footer_text` drops its Confirm branch (footer shows the normal text
   underneath). Keys and `confirm()` logic are unchanged. Tests: a `TestBackend` snapshot in
   `tests/snapshots.rs` (dialog centred at 80×24 and at 40×12, title truncated with `…`),
   and update the existing rerun/merge TUI and e2e tests that read the prompt from the footer
   (`wait_for_screen` still finds the text, the footer assertions move to the dialog).
   Re-run `scripts/screenshots.sh` only if `docs/details.png` changes (it should not).
0. **Config `prs.update_method`.** `UpdateMethod::{Merge, Rebase}` in `src/config.rs` (serde
   lowercase, `#[default] Merge`), `Prs::update_method`, shaped like #31's `MergeMethod`.
   Tests in `tests/config.rs`: missing key → `Merge`, `"rebase"` parses, `"squash"` is an error
   naming the file. `tests/cli.rs`: `conveyor config` prints `update_method = "merge"`.
1. **`Github::update_branch(repo, number, method) -> io::Result<UpdateOutcome>`** in
   `src/sources/github.rs`; `CliGh::update_branch_args` mirrors `rerun_failed_args`
   (`pr update-branch <N> -R <repo>`, plus `--rebase` for `Rebase`). `UpdateOutcome::{Updated, AlreadyUpToDate}` from gh's
   output; an error whose text contains `due to conflicts` maps to a distinct conflict error.
   `fetch::update_branch` turns it into `Result<UpdateOutcome, String>` like
   `fetch::rerun_failed`. Tests in `tests/github.rs` (take `SHIMS`): argv pinned; shim printing
   each of the three messages, conflict with exit 1.
2. **Confirm.** `Pending::UpdateBranch(UpdateTarget { repo, number })` next to
   `Pending::Merge`; `update_target()` refuses, each with a footer notice:
   - not My PRs: `U updates a pull request's branch; focus My PRs`
   - last PRs refresh failed: same message `X` uses
   - `queue_position.is_some()`: `#N is in the merge queue; updating it would drop it`
   - `MergeState::Dirty`: `#N has conflicts with its base; resolve them locally`
   - an update already in flight (`App::updating`)
   Dialog question: `update branch of #N <title>?` (`rebase branch of …` under `rebase`). `y` re-derives the target and
   goes on only if unchanged (as `confirm_merge` does); any other key: `update cancelled`.
   TUI tests through `run()` in the existing merge/rerun test file.
3. **Wire it.** `Action::UpdateBranch`, `Request::UpdateBranch`, `Input::BranchUpdated`, a
   `spawn_update_worker` in `src/main.rs` shaped like `spawn_rerun_worker`. Notices:
   `branch of #N updated`, `#N is already up to date`,
   `#N has conflicts with its base; resolve them locally`, else `update failed: <gh message>`.
   On `Updated`, refresh My PRs. e2e: shim appends `pr update-branch …` argv to a log; `U` then
   `y` writes it, `U` then `n` does not; a shim run printing the conflict message shows the
   conflict notice.
4. **Help, docs, release.** Share #31's line: `("M/U", "merge / update the branch (y/n)")`, so
   `KEYS` stays 14 and the help still fits 17 rows. README key table and default config
   (`update_method = "merge"`, pinned by `tests/readme.rs`),
   AGENTS.md write-action bullet for `U`, `plans/phase-21-update-branch.md`, PLAN.md outcome.
   Bump the minor version (new key).

## Files

- `src/config.rs`, `src/sources/github.rs`, `src/sources/fetch.rs`, `src/main.rs`
- `src/app.rs`, `src/ui/mod.rs`, `src/ui/dialog.rs` (new), `src/ui/footer.rs`, `src/ui/help.rs`
- `tests/github.rs`, the merge/rerun TUI tests, `tests/e2e.rs`, `README.md`, `AGENTS.md`,
  `PLAN.md`, `plans/phase-21-update-branch.md`, `Cargo.toml`

## Verification

- `scripts/gates.sh` (fmt, clippy, tests, e2e).
- Live after `scripts/dev-install.sh`: `conveyor-dev` on a throwaway PR of this repo that is
  behind main → `U`, `y` → `branch of #N updated`, GitHub shows the merge commit. Again →
  `already up to date`. A throwaway PR editing the same line as main → conflict notice.

## Caveats

- `U` pushes a commit to the PR branch and restarts its CI; the y/n prompt is the guard.
- `update_method = "rebase"` force-pushes: a local checkout of that branch diverges. The
  dialog reads `rebase branch of #N <title>?` in that mode so it is never a surprise.
- Assumption: the dialog replaces the footer prompt for `X` and `M` too (one shared Confirm
  mode). AGENTS.md's `X`/`M` bullets that say "the footer reads …" get rewritten in task 4.
- Needs push access to the head branch; a fork PR without "allow edits" fails with gh's message.
- Task 1's stream/exit-code check may change how `UpdateOutcome` is read (a `run` variant that
  keeps stderr on success).
