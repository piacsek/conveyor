# `M`: merge the selected pull request after a y/n prompt

## Context

conveyor is read-only today; merging a PR still costs a browser trip. `M` on a My PRs card asks
`merge #N <title>? y/n` in the footer and, on `y`, runs `gh pr merge`. It is the first write
action, so the prompt, the target it captures and the failure path matter more than the key.

Verified: `gh pr merge --help` (gh 2.103.3) says a branch behind a merge queue needs no strategy
(gh enqueues, or arms auto-merge while checks are pending); without a queue a strategy flag is
required non-interactively. `--match-head-commit SHA` refuses the merge if the head moved.

Decisions (taken with the user):
- Strategy comes from a new config key `prs.merge_method` = `squash` | `merge` | `rebase`,
  default `squash`, always passed (gh ignores it under a queue).
- Help gets its own line `("M", "merge the pull request (y/n)")`: `KEYS` goes 13 → 14, so the help
  pane needs a 17-row terminal; `q` stays first so it is never the clipped line.

## Behaviour

- `M` acts only in **My PRs**. Elsewhere: notice `M merges from My PRs` (queue entries are
  already queued; builds and deployments are already merged).
- Refused with a notice, no prompt: nothing selected; a draft (`#N is a draft`); a queued PR
  (`#N is already queued`); a merge already in flight (`merging #N…`).
- Prompt = `Mode::Confirm(MergeTarget { number, repo, title, head_oid })`, captured when `M` is
  pressed. `y` merges **that** target even if a refresh moved the selection meanwhile. Any other
  key (including `Esc`, `n`, `M`) cancels with notice `merge cancelled`. `Ctrl-C` still quits.
  The Confirm arm sits **before** the `Ctrl-d`/`Ctrl-u` arms in `handle_key`, so scrolling cannot
  slip past the prompt.
- On `y`: `Action::Merge(target)` → `Request::Merge(target)` → merge thread runs
  `gh pr merge <N> -R <repo> --<method> --match-head-commit <oid>` (flag omitted if oid empty) →
  `Input::Merged(number, Result<(), String>)`.
- Result: ok → notice `merge sent for #N`, then `Request::Refresh` for PRs and Queue (not through the key debounce: one result, not a key repeat)
  (the queue icon on the card tells whether it merged, queued or armed auto-merge). Err → gh's
  stderr in the footer, `merging` cleared so `M` can retry.

## Files

| File | Change |
|---|---|
| `src/sources/queries/prs.graphql`, `src/model/prs.rs` | add `headRefOid` → `PullRequest::head_oid` (lenient: missing = empty) |
| `tests/fixtures/prs.json` | add `headRefOid` (real sha of this repo's own PR) |
| `src/config.rs` | `MergeMethod` enum (serde lowercase, default `Squash`), `Prs::merge_method` |
| `src/sources/github.rs` | `Github::merge(repo, number, method, head_oid)`; pure `CliGh::merge_args` like `log_failed_args` |
| `src/app.rs` | `Mode::Confirm`, `MergeTarget`, `Action::Merge`, `Request::Merge` (drop `Copy` from `Request`, it now holds `String`s), `Input::Merged`, `App::merging`, `handle_confirm_key`, `M` arm next to `L` |
| `src/ui/footer.rs` | `footer_text` shows the prompt first while `Mode::Confirm` |
| `src/main.rs` | `spawn_merge_fetcher` modelled on `spawn_log_fetcher`; `Request::Merge` arm |
| `src/ui/help.rs`, `README.md` | 14th key; README key table + default config (`merge_method`) |
| `AGENTS.md` | "13 keys / 16 rows" → 14 / 17; one bullet: `M` is the only write action, target captured at prompt time, `--match-head-commit` |
| `Cargo.toml` | minor bump → v0.17.0 (new key + config key) |

Reuse: `App::notice`, `pull_target`'s PR lookup (`self.prs.selected()`),
the log-fetcher thread shape, `CliGh::run` (null stdin, stderr → error).

## Tasks (one `scripts/ship.sh` commit each, red test first)

Worktree `.claude/worktrees/phase-20-merge`, branch `phase-20-merge` off `origin/main`.
This file ships with the first commit.

1. `headRefOid` parsed into `PullRequest::head_oid` (`tests/parse_prs.rs`).
2. `prs.merge_method`: default squash, `rebase` parses, `fast` rejected (`tests/config.rs`); `conveyor config` prints it (`tests/cli.rs`).
3. `CliGh::merge_args` builds the exact argv, with and without oid (`tests/github.rs`, takes `SHIMS`).
4. App: `M` on a PR → Confirm + footer prompt; `y` → `Action::Merge` with the captured target; `n`/`Esc`/other → cancelled; refusals (draft, queued, other column, in flight). TUI tests through `run()`.
5. `run()`: `Action::Merge` sends `Request::Merge`; `Input::Merged` ok → notice + refresh requests; err → notice, retry allowed.
6. Wiring in `main.rs` + e2e: the `gh` shim answers `pr merge` by writing its argv to a file; press `M`, `y`; assert the file holds `pr merge <N> -R … --squash --match-head-commit <oid>`.
7. Help `KEYS` + README table/config + AGENTS.md; `tests/readme.rs` green.
8. Release v0.17.0 after `/pr-review` and merge.

## Risks

- `M` then `y` is two keystrokes from merging. Mitigation: target shown in the prompt by number
  **and** title, any non-`y` key cancels, `--match-head-commit` blocks a push you did not see.
- gh may print its success line to stderr; we do not parse it, so the notice stays generic.
- A repo that forbids squash fails with gh's error until the user sets `merge_method`.

## Verification

1. `scripts/gates.sh` (fmt, clippy, tests, e2e).
2. `scripts/dev-install.sh`, then in a scratch repo of your own: `conveyor-dev`, select a test PR,
   `M`, `n` → `merge cancelled`; `M`, `y` → `merge sent for #N`, card disappears after refresh.
3. Push a commit to that PR between `M` and `y` → footer shows gh's head-mismatch error.

Estimate: about half a day of TDD cycles, mostly tasks 4–6.
