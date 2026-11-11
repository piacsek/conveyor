# AGENTS.md

Notes for agents and humans working on `conveyor`. Keep this file short and
factual; verify against the code before treating anything here as live state.

## What this is

A Rust + Ratatui TUI that shows where a change is on its way to production:
my open pull requests, the merge queue, the last main builds, and what each
environment runs. One column per stage, side by side; a focused column,
actions on the selected row. `conveyor config` prints the effective config.
`PLAN.md` holds the phase history, retros, and the backlog.

## This repository is public

No employer data, ever: no org or repo names, PR numbers or titles, logins, hostnames, cluster
or kube-context names, ticket prefixes. Fixtures come from this repo's own public PRs;
screenshots and TUI tests use invented `acme/*` data; plans and docs use `<owner>/<repo>`
placeholders. `scripts/scrub-check.sh` (first gate) fails when a tracked file matches the
local denylist at `~/.config/conveyor-dev/denylist` (kept outside the repo on purpose; the
check is skipped where the file is absent, so CI does not enforce it).

## Layout

```
src/main.rs      CLI dispatch, config load, wiring
src/cli.rs       `conveyor` (TUI) | `config` | `log` | `--help` | `--version`; hand-rolled, no clap
src/config.rs    Config (serde + toml, deny_unknown_fields), XDG path, `CONVEYOR_CONFIG` override, `expand_home`
src/sources/     everything that reaches out to an external service, and nothing else
src/sources/github.rs Github trait (`graphql`, `rest`, `current_repo`, `log_failed`, `rerun_failed`, `merge`, `update_branch`, `change_status`, `checkout`), CliGh spawns `gh api …` / `gh repo view` / `gh run view --log-failed` / `gh run rerun --failed` / `gh pr merge|update-branch|ready|close|checkout` and `git remote -v` (the checkout guard), error mapping
src/sources/kube.rs   Kube trait (`deployment(env)`, `pods(env, selector)`), CliKubectl (`kubectl --context … get deploy|pods … -o json`, a 10 s request timeout and a 15 s `DEADLINE` that kills the process)
src/sources/fetch.rs  repos() (config or cwd), fetch_prs, fetch_queue (+ merge-group runs via REST), fetch_jobs, fetch_log (LOG_TAIL lines), Pulls (sha→PR: `prefetch` asks for all the shas of a fetch in one GraphQL call, `for_sha` reads the cache and asks REST for the rest), BuildsSource (caches workflow id and sha→PR), DeploySource (kubectl per env on parallel threads + cached sha→PR)
src/sources/queries/  GraphQL documents, `include_str!`ed
src/sources/AGENTS.md GitHub and kubectl facts: API shapes and queries, the sha → pull request lookup, the spawn lock, call logging, the kubectl deadline
src/model/prs.rs PullRequest, CheckState, Check, ReviewDecision, MergeState; lenient `parse` of gh JSON
src/model/queue.rs Queue, QueueEntry, QueueState, MergeGroupRun; `parse`, `parse_merge_group_runs`, `attach_runs`
src/model/builds.rs Build, BuildStatus, PullRef, Builds; `parse_runs`, `pr_number_from_title`, `parse_pull_numbers`, `parse_commit_pulls` (the batched GraphQL answer)
src/model/deployed.rs Deployment, Deployed, Rollout, Condition, Pod, Health; `sha_from_image` (40-hex tag or `-<sha>` suffix), `parse_deployment`, `parse_pods` (problems first), `health`
src/model/jobs.rs Job, `parse_jobs` (failures first), `failed_step`
src/model/log.rs  `tail`: the failed step's log as gh prints it, minus job/step/timestamp prefixes and ANSI
src/model/AGENTS.md check dedup, merge-group run attach rules, deploy health rules, jobs parsing, lenient parsing
src/app/mod.rs   App (one stage type per stage, focus, Mode, query, jobs, logs, log_open, log_scroll/log_page), Mode::{Normal, Filter, Help, Changelog, Confirm, Manage}, Input::{Key, Fetching, Data, Jobs, Log, Merged, Rerun, BranchUpdated, StatusChanged, CheckedOut, Tick}, Action, Request::{Refresh, Jobs, Log, Rerun, Merge, UpdateBranch, ChangeStatus, Checkout}; `behind_main`, `deploy_configured`, `follow_up_*`; re-exports everything below as `app::X`
src/app/stages/  one type per stage behind the StageView trait: `mod.rs` is the trait and the dispatch, one file per stage for everything that stage does
src/app/stages/mod.rs Stage, Rows, StageView (`listing`, `receive`, `error`, `pull_target`, `build_url`, `selected_run`, `runs`, `menu`, `changed`), Context (what a stage may read from the rest of App: config, Main builds, `merge_landing`; `behind_main`, `deploy_repo`), `App::stage`/`stage_mut` (the one match from a Stage to its field), `receive`, `fetching`, `any_refreshing`, `spinner`
src/app/stages/prs.rs PrsStage: what My PRs offers (`menu`, `offers`, `merge_target`, `update_target`, `rerun_target`, `checkout_target`) and where its rows lead
src/app/stages/queue.rs QueueStage (column, `repo`): its merge-group runs and where its rows lead
src/app/stages/builds.rs BuildsStage (column, `repo`): its runs, what Main builds offers (`menu`, `rerun_target`) and where its rows lead
src/app/stages/deployed.rs DeployedStage (column, `system`): a row's own error, where its rows lead, `keep_last_known`
src/app/browser.rs where a row leads: `pull_target` and `build_url` of the focused stage, `open` and its `OPEN_DEBOUNCE`
src/app/navigation.rs moving around: the `focused_*` readers, `refocus`, `with_focused`, the filter (`filter`, `sync_filter`) and the scroll helpers
src/app/column.rs Column<T: Row> (state, error, list, filter), ColumnState, Row and its four impls, Order, `merge_keeping_order`, Navigable, Listing (what every column answers whatever its row type)
src/app/jobs.rs  JobsState, LogState, `age_jobs`, `jobs_needed` (selected, stale, then the first `EAGER_JOBS` failing builds), `selected_run`, `toggle_log`, `receive_jobs`, `receive_log`
src/app/actions/ the `S` popup: `mod.rs` is the machinery, one file per action for how it runs; what a column offers is its stage's `menu`
src/app/actions/mod.rs Menu/Offer/Choice, Pending, `ask_to_manage`, `menu` (the focused stage's), `manage`, `confirm`, `reconfirm`, `write_in_flight`, `offer`
src/app/actions/merge.rs MergeTarget, `confirm_merge`, `receive_merged`
src/app/actions/update.rs UpdateTarget, `confirm_update`, `send_update`, `receive_branch_update`, `conflicts`
src/app/actions/status.rs StatusTarget, `confirm_status`, `send_status`, `receive_status`, the `progress` and `done` wording
src/app/actions/checkout.rs CheckoutTarget, `send_checkout`, `receive_checkout`
src/app/actions/rerun.rs Rerun, `rerun_offer`, `send_rerun`, `receive_rerun`
src/app/keymap.rs `handle_key` and the Filter and Normal mode key handlers, `escape`
src/app/run.rs   run(): draw, read one Input, turn each Action into a Request
src/app/AGENTS.md keys, `Esc`, the filter, the `S` popup and each action, jobs state, refresh and follow-up, row order, open debounce, zoom, scroll
src/ui/mod.rs    draw: 4 columns or tabs below `4 × ui.min_column_width`
src/ui/style.rs  the shared vocabulary: BAR, INDENT, dim(), glyphs and status words, short_repo, sha8
src/ui/card.rs   card() and meta(): one title line plus indented span lines
src/ui/columns.rs column titles, blocks and draw_list
src/ui/rows.rs   one card builder per stage: pr_row, queue_row, build_row, deployed_row
src/ui/details.rs the `d` pane per stage, including the job list; no URLs
src/ui/footer.rs footer text, the refresh spinner and the static logo
src/ui/dialog.rs the `S` popup (`draw_manage`) and the confirm dialog (`draw_confirm`) for `m`, `x`, enabling auto-merge and a rebase
src/ui/help.rs   KEYS is the generic table the README mirrors; `keys_for(stage)` swaps the `p`/`b` wording per column
src/ui/AGENTS.md  cards, glyphs and icons, dialogs, the details, help and changelog panes, the footer, colours
src/refresh.rs   `spawn`: one fetcher thread per stage; `wait` sleeps until the interval or a refresh, draining queued signals
src/open.rs      Opener trait; SystemOpener (`open`/`xdg-open`, `pbcopy`/`xclip`): the OS seam, not a service, so it stays out of `sources/`
src/changelog.rs CHANGELOG (the bundled CHANGELOG.md) and `check`, the rules a release entry must meet
src/ui/changelog.rs the `c` pane: wrapped to its width (bullets hang), headings bold without their `#`, scrolled like the details pane
src/log.rs       `path` (CONVEYOR_LOG, else XDG_STATE_HOME/conveyor/<program>.log), `program_name` (argv[0]), Log (`open` rotates to `.1`, `line`, `call`)
src/text.rs      truncate/pad_right with `…`, age(), duration() (coarse, cards), elapsed() (seconds, details pane), strip_ansi()
scripts/gates.sh              the quality gates; fails loudly, never pipe it through tail
scripts/scrub-check.sh        denylist grep over tracked files (see "This repository is public")
scripts/dev-install.sh        release build symlinked as ~/.local/bin/conveyor-dev
scripts/ship.sh               gates + dev-install (unless skippable) + commit + push, aborts on any failure; the first push of a branch opens the draft PR (commit subject as title, the template as body)
scripts/dev-install-skippable.sh  exit 0 with the reason when `ship.sh` may leave `conveyor-dev` alone, else 1 with what needs the install
plans/                        one versioned plan file per phase (see Working agreements)
scripts/homebrew-formula.sh   prints the tap formula for a released version
scripts/release.sh            tags a merged version from `main`, pushes the tag, watches `release` and checks the tarballs and the tap formula
scripts/screenshots.sh        renders docs/details.png from a gh shim + synthetic fixture (truecolor.py converts ANSI)
tests/           outside-in: `tests/cli.rs` runs the real binary; TUI tests drive run() with a TestBackend;
                 `tests/readme.rs` pins the README shape and its key table to `ui::help::KEYS`;
                 `tests/source_size.rs` fails while any Rust file under `src/` or `tests/` is longer than 700 lines, naming it;
                 `tests/stage_dispatch.rs` fails while a match arm on a `Stage` variant sits in `src/app/` outside `stages/mod.rs`, naming the line;
                 a test file that outgrows the limit becomes a folder (`tests/<name>/main.rs` plus one file per topic), so it stays one binary
tests/AGENTS.md  the testing traps hit so far, the e2e `gh` shim dispatch note
.github/AGENTS.md CI: what the workflows run and why, the `main` ruleset, branch cleanup
.github/pull_request_template.md  `## Summary` and `## Changes`: the body `ship.sh` opens a draft PR with
```

## Where the rest lives

This file holds the rules and agreements. The facts about each directory live in an `AGENTS.md` next to the code; read the one for a directory before changing anything in it.

| File | What it holds | Read it |
| --- | --- | --- |
| `src/sources/AGENTS.md` | GitHub and kubectl: API shapes and queries, the sha → pull request lookup (`Pulls`), the spawn lock, call logging, `gh` never inheriting the terminal, the kubectl deadline | before touching `src/sources/` or anything that spawns a program |
| `src/model/AGENTS.md` | check dedup (`newest_checks`, superseded runs, `overall`), merge-group run attach rules, deploy health rules, jobs parsing, lenient parsing | before touching `src/model/` or a fixture it parses |
| `src/app/AGENTS.md` | keys, `Esc`, the filter, the `S` popup and each action (`m`, `u`, `f`, `t`/`a`/`e`/`x`, `c`), jobs state (`age_jobs`, `jobs_needed`), refresh and follow-up, row order, open debounce, zoom, scroll | before touching `src/app/`, `src/refresh.rs` or `src/open.rs`, or adding a key or an action |
| `src/ui/AGENTS.md` | cards, glyphs and icons, dialogs, the details, help and changelog panes, the footer, colours | before touching `src/ui/` or `src/changelog.rs`, or changing anything visible |
| `tests/AGENTS.md` | the testing traps hit so far, the e2e `gh` shim dispatch note | before writing or debugging a test, a shim or a launcher of the binary |
| `.github/AGENTS.md` | CI: what the workflows run and why, the `main` ruleset, branch cleanup | before touching `.github/` or a required check |

## Behaviour that is easy to break

The two bullets about `src/cli.rs` and `src/config.rs` stay here; every other one is in the `AGENTS.md` of the directory it is about.

- **`--help`/`-h`/`help` and `--version`/`-V`/`version`** print to stdout and exit 0 before
  the config is read, so a broken config never hides them.
- **Config is loaded before any subcommand runs.** Missing file = `Config::default()`; an
  unreadable file or unknown key is an error naming the file (`deny_unknown_fields` on every
  table). `conveyor config` exits 1 on it and doubles as the reference for defaults.

## Documentation rule

`README.md` is minimal and holds only what a user needs, in this order and nothing else: the
banner `<img>` (it stands in for the h1; there is no `# ` heading), two sentences on what conveyor does, the one screenshot `docs/details.png` right below them (all
four columns with the details pane open), then `## Installation`, `## Usage` (commands, the key
table, the default config) and `## Development`. The key table mirrors `ui::help::KEYS` word
for word; `tests/readme.rs` fails when the README and the help view drift, when a second
screenshot, a subsection or another section appears, or when the description grows past two
sentences (the help view is the single source: change `KEYS`, then the table). Feature depth, glyph meanings and data-source notes belong in the `AGENTS.md` files, not there. After
any visible layout change run `scripts/screenshots.sh` (needs `brew install
charmbracelet/tap/freeze`, Google Chrome, and the FiraCode Nerd Font in `~/Library/Fonts`) and
commit the new images. Freeze lays out an SVG with the font embedded; headless Chrome
rasterises it, because freeze's own PNG output cannot draw Nerd Font glyphs. The script adds
Menlo (DejaVu Sans Mono on Linux) as the SVG's fallback font family: FiraCode lacks `✗ ⊘ ▸`,
and Chrome's default fallback is wider, so every row holding one shifted and the borders jogged.
Fixture SHAs look real (`8c1f2a7d…`), not `33333333`. The script never
reads the real `gh` account: `HOME` is a tempdir and a `gh` shim printing a synthetic
`acme/*` fixture is first on PATH (through `env`, see the e2e trap in `tests/AGENTS.md`). Do not screenshot
real data: this repository is public.

Whenever behaviour changes, check that `README.md` is still true: a key, a default, a
subcommand, a config key. Do not add prose for a feature; if it needs explaining, the card or
the help view should carry it.

## Working agreements

All guidelines live in `AGENTS.md` files, versioned with the code (the root one for rules and
agreements, one per directory for that directory's facts); nothing binding lives only in an
agent's private memory. When the user says "update the guidelines", edit the `AGENTS.md`
nearest the code the guideline is about.

- **"Work autonomously" means the whole road to a release, without pausing**, not even for
  the retro: its outcome is written into `PLAN.md` in step 4 and the user reads it there. When the user
  says it (or "autonomous", "march towards release"), assume this flow and ask only when a
  decision is genuinely theirs:
  1. Plan in `plans/phase-N-<topic>.md` (next free N in `PLAN.md`), in a fresh worktree.
  2. `/my-tdd autonomous`: one task per red-green-refactor cycle, each shipped with
     `scripts/ship.sh` (gates, dev install, commit, push, draft PR on the first push).
  3. `/pr-review` the PR (see "Review before ready"). Fix every must and should finding, TDD
     like any change, ship, and review again. Minor findings are fixed when cheap, else listed
     in the PR body. The loop ends only on a round with no must/should findings, and that
     round is **unprimed**: the reviewers get the PR, the repo and this file, never the list
     of earlier findings or what was decided about them. Never post the review on the PR.
  4. The phase outcome is in `PLAN.md` and, for a behaviour change, the version bump and its
     `CHANGELOG.md` entry are on the branch. Re-read the PR title and body, fix what went
     stale, `gh pr ready`, wait for CI green.
  5. Rebase-merge (squash when GitHub cannot rebase the branch).
     For a behaviour change, tag and release per "Releasing", watch `release`
     to green and check the tap formula names the new version.
  6. Remove the worktree and report what shipped, with the release link.
  Stop and ask instead of proceeding when: a gate fails for a reason the change did not
  cause, CI fails twice on the same step, a review finding needs a product decision, or the
  next step would force-push, push to `main` or delete a release.
- **Cadence.** Outside the autonomous flow above, work runs under `/my-tdd autonomous`: one branch per phase (`phase-N-<topic>`),
  one commit per task through `scripts/ship.sh`, no pause between tasks, a pause only for the
  end-of-phase retro. Retro outcomes go to `PLAN.md` (facts, decisions) and here (rules).
- **One worktree per branch, by default.** Start every phase or fix in its own worktree off
  `origin/main`: `git worktree add -b <branch> .claude/worktrees/<branch> origin/main`, then
  `git branch --unset-upstream` so the first `ship.sh` push creates `origin/<branch>` instead of
  tracking `main`. Work, gates and ships run from inside it; the main checkout stays on its own
  branch, so two phases (and two agent sessions) never share an index or a `target/`.
  `.claude/worktrees/` is gitignored. Each worktree builds its own `target/` from cold the first
  time. `conveyor-dev` points at the worktree that last ran `dev-install.sh`, so re-run it in the
  worktree you are testing. Remove the worktree (`git worktree remove`) once its PR is merged.
- **TDD is never skipped.** Every change to `src/` starts with a failing test, run and seen
  red, before the production code is touched; then the smallest change that goes green, then
  refactor. This holds for one-line fixes, security patches, patch releases and "obvious"
  changes alike. Writing the test and the fix in the same edit is not TDD, even when the test
  is then backed out to prove it fails. Show the red run in the update.
- **Draft PR first.** The first ship on a branch opens the draft PR assigned to the user,
  with the commit subject as its title and `.github/pull_request_template.md` as its body;
  every later commit lands on that PR. The subject is a starting point: before review, rewrite
  the title in plain language that says what the change does for whoever uses it, and fill the
  body. `## Summary` holds at most 3 sentences, split across 2 lines with a blank line between
  them; `## Changes` holds the rest. Before merging, re-read the PR title and body and fix
  what went stale without rewording the user's edits.
- **Screenshots in PRs.** A PR with a visible change embeds `docs/details.png` in its body as
  a commit-pinned `raw.githubusercontent.com` URL.
- **Diagrams in PRs.** A PR that reorganizes the codebase or introduces a new flow carries a
  Mermaid diagram in its body, in a fenced `mermaid` block that GitHub renders. Show the
  shape of the change: the modules and what moved between them, or the steps of the flow and
  who calls whom. For a reorganization, a before and an after. Keep each diagram small enough
  to read without scrolling, and leave it out when the change has no structure to draw.
- **Review before ready.** Marking a PR ready for review means running the
  [`/pr-review`](https://github.com/piacsek/dotfiles/blob/main/.claude/skills/pr-review/SKILL.md)
  skill on the PR first, and acting on what it finds. It runs a Claude and a Codex reviewer
  over the diff and merges their findings; run it from the main session, because the skill
  launches Codex there. Give both reviewers the intent of the change, point them at this
  file, have the Claude reviewer run `scripts/gates.sh` itself rather than trust the claim
  that the gates pass, and forbid edits, commits, and any write to GitHub or call to the
  clusters. When the skill asks whether to post the review on the PR, answer no.
  `gh pr ready` comes after the findings are addressed, not before.
- **Merging and releasing.** Rebase-merge the phase PR into `main` when CI is green (squash
  when GitHub cannot rebase it, see the next bullet), then tag
  from `main` (see Releasing). Never push to `main` directly except the very first bootstrap
  commit of an empty repository, and never force-push.
- **Catching up with `main` on a pushed branch is a merge, not a rebase.** A rebase rewrites
  the pushed commits and the next `scripts/ship.sh` push is refused; the no-force-push rule
  then leaves only `git reset --hard origin/<branch>`, `git merge origin/main` and a
  cherry-pick of the new work. The rebase-merge into `main` drops a clean merge commit anyway.
  A merge that resolved conflicts can stop GitHub from rebase-merging: it answers `This branch
  can't be rebased`, because the resolutions live only in the merge commit and replaying the
  branch's own commits hits the same conflicts again (phase 28, after v0.21.2 shipped mid-phase
  and its snapshots and `CHANGELOG.md` conflicted). When GitHub refuses, the branch lands as a
  **squash merge** instead, subject `<type>(<scope>): <summary> (#N)`; the per-task commits stay
  readable on the PR.
- **Two binaries.** `conveyor` on PATH is the Homebrew release; `conveyor-dev` is a working
  tree (`scripts/dev-install.sh`). Never `cargo install` the crate into a PATH directory.
- **Plan before code, plans in the repo.** Each phase starts from a written plan, re-planned
  after the previous retro, with its "Verified facts" checked against live tools before use.
  Plans are versioned here: write each one to its own file under `plans/` (for example
  `plans/phase-5-cards.md`) and commit it with the first ship of the phase. Never leave a plan
  only in an agent's private plan directory or memory. `PLAN.md` stays the index of phases,
  retros and the backlog; a phase's outcome is appended there when it closes.

## Development

Strict TDD: one failing test, minimal code, refactor. Outside-in: start from the binary or
from `run()` with scripted inputs, drop to unit tests only for pure helpers. No code
comments; names and tests carry the intent.

Gates, all required before a task is done, wrapped by `scripts/gates.sh`:

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -- --include-ignored   # one pass; the ignored ones are e2e: real binary, `gh` shim first on PATH
```

Without tmux the last gate is a plain `cargo test` and a notice that the e2e tests were skipped.
`tests/scripts/` runs the scripts themselves in a scratch directory and a scratch git
repository, with `cargo`, `gh` and the scripts they call replaced by symlinks to
`tests/fixtures/record-call.sh`, which appends its name and argv to `$CALLS_LOG` and exits 1 for a call whose argv starts with `$FAILING_CALL` (how a ship test says the branch has no pull request yet). Where a
script reads what `gh` prints (`release.sh`), `gh` is a symlink to `tests/fixtures/reply-call.sh`
instead: it records the same line, then prints every file in `$REPLIES` whose name is part of
its argv.

Ship with `scripts/ship.sh "<message>"`: `cargo fmt`, gates, `dev-install.sh` (unless skipped, see Install), commit, push, all under
`set -e`, refusing to run on `main`, opening the draft PR on the first push of a branch (title: the first line of `<message>`; body: `.github/pull_request_template.md`, or empty when that file is missing). Do not hand-roll the chain: the interactive shell here is
zsh, where `PIPESTATUS` is undefined and `test "" -eq 0` is true, so a `gates.sh | grep` guard
silently passed a clippy failure into a commit on 2026-09-10.

Run the wrapper, not the commands: piping `gates.sh | tail` once hid a `cargo fmt --check`
failure in tmux-agents and an unformatted commit slipped through. Chaining gates with `&&`
and then `;` before `git commit` does the same (it happened on this repo's bootstrap commit).

Install: `scripts/dev-install.sh` builds the release binary and symlinks it as
`~/.local/bin/conveyor-dev`; `conveyor` on PATH is always the Homebrew release. Never
`cargo install` the crate into a PATH dir. A milestone is done only after the gates pass
**and** `ship.sh` has either run `dev-install.sh` or said why it skipped it, so `conveyor-dev`
is the committed code of every branch that changed what the binary is built from.

`ship.sh` skips the install, printing `skipping dev-install: <reason>`, only when both hold
(`scripts/dev-install-skippable.sh`): `conveyor-dev` resolves to another worktree's build, and
nothing between the merge base with `origin/main` and the working tree, untracked files
included, is outside `tests/`, `docs/`, `plans/`, `scripts/`, `.github/`, an `AGENTS.md` at any depth (`src/` included),
`PLAN.md`, `README.md`, `LICENSE` and `.gitignore`. Everything else counts as feeding the
binary (`src/`, `Cargo.toml`, `Cargo.lock`, the bundled `CHANGELOG.md`, any path nobody
listed), and a missing or dangling `conveyor-dev`, or no `origin/main`, installs too. So a
branch of docs, tests, scripts or CI never takes `conveyor-dev` over, and `conveyor-dev` then
stays the other worktree's build, not this branch's. Once `conveyor-dev` is this worktree's
build every ship runs `dev-install.sh`: cargo's own freshness check makes it a no-op of about
0.3 s when nothing changed (measured 2026-11-10; 17 s after a `src/` change, 23 s from a cold
`target/`), and it rebuilds after a merge of `main` that no diff of the commit would show.
Embedding a file from one of the listed directories into the binary means taking that
directory off the list in the script.

## Releasing

Releases are GitHub Releases built by `.github/workflows/release.yml` on a `v*` tag: one flat
tarball per target (`conveyor-<target>.tar.gz` holding just the binary; targets
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`) plus `.sha256`,
with generated notes. Asset names carry no version so `releases/latest/download/` links stay
valid. Both macOS targets build in **one** `macos-latest` job (`cargo build --target … --target
…`, Intel cross-compiled); `macos-13` is retired and queues forever. They were two matrix jobs
until v0.19.2: hosted macOS runners regularly failed to come online (the v0.19.1 Intel job was
cancelled after 15 min without a step), and every macOS job is one more wait on that pool. A
stuck job is rerun with `gh run rerun <id> --failed`. The workflow refuses a tag whose version differs from
`Cargo.toml`. Every action is pinned to a commit SHA with the tag in a trailing comment;
bump pins with `gh api repos/<owner>/<repo>/git/ref/tags/<tag>`. Workflows default to
`contents: read`; only `publish` gets `contents: write`, `id-token` and `attestations: write`
for `actions/attest-build-provenance` (`gh attestation verify <tarball> --repo
piacsek/conveyor`). CI also runs `cargo audit` (a prebuilt binary via `taiki-e/install-action`;
the `rustsec/audit-check` action compiled cargo-audit from source on every run, ~3 min of a
~3.5 min pipeline). The `check` job runs `cargo test -- --include-ignored` in one pass; the e2e
tests take the debug binary from `CARGO_BIN_EXE_conveyor`, so no release build there.

Only behaviour changes get packaged. A release (version bump, tag, tarballs, tap) follows a
change a user can observe in the binary: a key, a card, a default, a fix, a config key. Docs,
CI, tests, scripts, refactors, guideline edits and copy-only wording (a word in the help view,
an error message) merge to `main` without a bump and ride the next behaviour release. Do not
tag a docs-only or CI-only `main`.

1. Bump `version` in `Cargo.toml` (`Cargo.lock` follows on the next build) and add its
   `## vX.Y.Z (YYYY-MM-DD)` entry at the top of `CHANGELOG.md`: what a user sees, one bullet each.
2. Run the gates, commit `Release vX.Y.Z`, merge to `main`.
3. From a clean `main` that is exactly `origin/main`:

   ```bash
   scripts/release.sh X.Y.Z
   ```

   It refuses a dirty tree, another branch, a `main` that differs from `origin/main`, a version
   `Cargo.toml` does not carry, a tag that already exists here or on `origin`, and a `release`
   run still queued or in progress. Then it tags `vX.Y.Z`, pushes that one tag, watches the
   `release` run to its end and checks that the release holds the three tarballs and their
   `.sha256` and nothing else. The last line it prints is the release URL.
4. The script also reads `Formula/conveyor.rb` from GitHub and fails when it does not name
   the version. The `tap` job regenerates `Formula/conveyor.rb` in `github.com/piacsek/homebrew-tap`
   (local clone `~/projects/homebrew-tap`) with `scripts/homebrew-formula.sh <version>` and
   pushes with the `TAP_TOKEN` repo secret (fine-grained PAT, contents:write on
   homebrew-tap; set per repo with `gh secret set TAP_TOKEN --repo piacsek/conveyor`). If
   that job fails, run the script by hand and push the tap. Check with `brew audit --strict
   --online --formula piacsek/tap/conveyor` after changing the script.

Installed copies: Homebrew puts the release binary in `$(brew --prefix)/bin/conveyor`;
dev builds are only reachable as `conveyor-dev`. `rustsec/audit-check` v2.0.0 (latest as of
2026-09-10) still targets Node 20 and CI prints a deprecation annotation; bump the pin when a
newer release exists.

Never force-push a tag that has a release: the push re-runs `release.yml`, republishes the
assets and rewrites the tap formula for that old version (seen 2026-09-10 after a history
rewrite, when the `v0.0.1` re-run raced the `v0.1.0` run for the formula). If a released
tag must move, delete the release and the tag first, or bump the version instead.

Semver: minor for new columns/keys/config, patch for fixes, major on a config-format break.
