# Phase 60: the guidelines say only the rules (#125)

## Context

Issue #125. Root `AGENTS.md` opens with "Keep this file short" and is 50 KB. Its rules are
mixed with the incidents that produced them, its `## Layout` restates about 110 files by hand,
and an outside contributor cannot tell which rules apply to them. Phase 60 is also a sweep
phase, so it opens with the sweep of phases 40 to 68.

The one hard requirement: no rule is lost or weakened. The rule inventory below lists every
binding rule of each file this phase touches and where it lives after the change.

## Verified facts (2026-11-17, `main` at d5578ca)

- Sizes before (lines, bytes): `AGENTS.md` 583 / 50,159; `.github/AGENTS.md` 576 / 47,518;
  `src/app/AGENTS.md` 422 / 37,633; `src/sources/AGENTS.md` 285 / 24,285; `src/ui/AGENTS.md`
  177 / 15,062; `src/model/AGENTS.md` 114 / 9,872; `tests/AGENTS.md` 83 / 6,881.
- What points into the root file by name, and must keep pointing at something real:
  `.github/CONTRIBUTING.md` links the anchors `#this-repository-is-public`,
  `#where-the-rest-lives` and `#releasing`; `scripts/version-check.sh` prints "see Releasing in
  AGENTS.md" (pinned by `tests/scripts/version_check.rs`); `scripts/sweep-issue.sh` names "The
  sweep"; `.claude/skills/sweep/SKILL.md` names "The sweep", "Hurdles" and "Time and tokens";
  `.claude/skills/adversarial-review/SKILL.md` names "This repository is public", "Known limits"
  and "Review before ready" (severity by reachability); `plans/metrics.md` names "Time and
  tokens"; `.github/AGENTS.md` names Releasing three times. No test reads the text of an
  `AGENTS.md`; `tests/plans.rs` reads `plans/`, `tests/readme.rs` reads `README.md`.
- #118 is open and proposes dropping the frozen changelog; nothing has been removed.
  `CHANGELOG.md` on `main` still tops at `## v0.24.1 (2026-11-11)` and `src/changelog.rs` has
  `UNRELEASED = "0.0.0"`. So "a `CHANGELOG.md` frozen at the last hand-made entry (v0.24.1)" is
  true today; #125 read #118's title as a fact.
- Comments in the code: `src/` and `tests/` carry `//`, `//!` and `///` lines (for example
  `src/sources/github.rs`, `src/open.rs`, `src/text.rs`, `tests/readme.rs`), against "No code
  comments".
- Backfill, read from GitHub: #135 ready 19:34:13Z, merged 19:37:20Z, v0.35.2 19:39:26Z; #134
  ready 19:26:57Z, merged 19:47:34Z, v0.35.3 19:54:53Z; #136 ready 18:52:51Z, merged 18:54:32Z,
  no fragment so no release; #137 ready 23:41:54Z, merged 23:46:05Z, v0.35.4 23:48:12Z.

## Decisions on the issue's points

| Point of #125 | Decision |
| --- | --- |
| Rules mixed with history | Fixed here: each rule keeps at most one sentence of reason; the incidents, dates, PR numbers and timings move to "History moved out of `AGENTS.md`" below. |
| The `## Layout` file list drifts | Fixed here: a directory-level map that keeps only what carries a rule, and `tests/layout.rs`, which fails while any path the map names, in a description too, does not exist. |
| The CHANGELOG sentence is false | Left: it is true today (see Verified facts). |
| Outsiders cannot tell which rules apply | Left to the owner: saying which rules bind a fork changes the meaning of the working agreements either way. Every rule stays in the section it had (see "Scope at base and now"), and "What this is" points an outside contributor at `.github/CONTRIBUTING.md`, which already links the sections they need. |
| "No code comments" is broken by the code | Rule kept; enforcing it is a backlog item, because a test now would fail on today's code, and whether docblocks count is the owner's call (see the hand-back). Existing comments stay. |
| The same pass for `.github/AGENTS.md` and the per-directory files | Backlog item: each needs its own inventory. Only the facts moved out of the root file land in them here (`tests/AGENTS.md`, `src/ui/AGENTS.md`). |

## Tasks

1. The sweep, this plan, the `PLAN.md` and `plans/metrics.md` rows, and the backfill of phases
   64, 66, 67 and 68.
2. `tests/layout.rs`, seen red on today's map (`flake.nix, flake.lock` is not one path), then the
   trimmed root `AGENTS.md`, with the facts it sheds moved to `tests/AGENTS.md` and
   `src/ui/AGENTS.md`.
3. The PR title and body.

## Rule inventory

Every binding rule of root `AGENTS.md` at d5578ca, one line each, with where it lives after
this phase. "Root" means root `AGENTS.md`; a section name follows.

| # | Rule (before) | After |
| --- | --- | --- |
| 1 | Keep this file short and factual; verify against the code before treating it as live state | Root, opening |
| 2 | No employer data: org or repo names, PR numbers or titles, logins, hostnames, cluster or kube-context names, ticket prefixes | Root, This repository is public |
| 3 | Fixtures from this repo's own public PRs; screenshots and TUI tests use `acme/*`; plans and docs use `<owner>/<repo>` | Root, This repository is public |
| 4 | Do not screenshot real data (was in Documentation rule) | Root, This repository is public |
| 5 | `ship.sh` runs `scrub-check.sh` against the denylist at `~/.config/conveyor-dev/denylist` or `CONVEYOR_DENYLIST`, three times | Root, This repository is public, with what each run reads (a symlink's target and a merge's conflict resolution included, never a removed line, so a branch can remove a term `main` still holds); each case: `tests/scripts/scrub_check.rs` |
| 6 | A match names which of the four places it was in and its commit | Root, This repository is public (the four named: message, path, added line, branch name) |
| 7 | A refused push: `git reset --soft HEAD~1`, fix what is named (`git branch -m` for the branch), ship again; an earlier commit is rewritten by hand | Root, This repository is public |
| 8 | A refused merge is kept and fixed with `git commit --amend`; `ship.sh` then pushes it without a new commit | Root, This repository is public |
| 9 | The list stays outside the repo; skipped only where the default path is confirmed absent, so CI does not enforce it | Root, This repository is public |
| 10 | It fails when the path is there but unreadable, its absence cannot be confirmed, `CONVEYOR_DENYLIST` names a missing file, the list is empty, or git cannot run | Root, This repository is public (every case, as at the base); each case: `tests/scripts/scrub_check.rs` |
| 11 | It never reads binary contents, commit authors, the PR body, or anything pushed without `ship.sh` | Root, This repository is public |
| 12 | The layout names what each part is for; types from the code, rules from the directory's `AGENTS.md` | Root, Layout |
| 13 | `src/sources/` is everything that reaches an external service, and nothing else | Root, Layout |
| 14 | `src/cli.rs` is hand-rolled, no clap | Root, Layout |
| 15 | `src/open.rs` is the OS seam, not a service, so it stays out of `sources/` | Root, Layout |
| 16 | One type per stage behind `StageView`; the one match from a `Stage` to its field is in `stages/mod.rs` | Root, Layout (`tests/stage_dispatch.rs` pins it) |
| 17 | What a column offers in the `S` popup is its stage's `menu`; one file per action | Root, Layout |
| 18 | `scripts/feeds-binary.sh` is the one list of what does not feed the binary; `scripts/flake-inputs.sh` the one list of what the flake build reads | Root, Layout |
| 19 | `scripts/version-check.sh` reads the head as git objects and runs nothing from it | Root, Layout (and `.github/AGENTS.md`) |
| 20 | `plans/original-plan.md` is history, not live state | Root, Layout |
| 21 | `.github/CONTRIBUTING.md` points here and restates none of the rules | Root, Layout |
| 22 | The per-file descriptions of the other ~90 paths | Dropped: the code and each directory's `AGENTS.md`; `tests/layout.rs` keeps the map's paths real |
| 23 | Tests are outside-in: `tests/cli.rs` runs the binary, TUI tests drive `run()` with a TestBackend | Root, Development |
| 24 | What each repository test fails on (`readme`, `source_size`, `toolchain`, `plans`, `stage_dispatch`, `dead_code`) | Root, Layout names each test with a one-line purpose; the unused `pub fn` of `tests/support/mod.rs` case: Root, Development (dead code); the rest: the test and its failure message |
| 25 | No Rust file under `src/` or `tests/` is longer than 700 lines; a test file that outgrows it becomes `tests/<name>/main.rs` plus one file per topic | Root, Development |
| 26 | Read a directory's `AGENTS.md` before changing anything in it (the table) | Root, Where the rest lives (unchanged) |
| 27 | `--help`/`--version` print to stdout and exit 0 before the config is read | Root, Behaviour that is easy to break (unchanged) |
| 28 | Config loaded before any subcommand; what is refused; refuse only what something named breaks with; `conveyor config` exits 1 and is the reference for defaults | Root, Behaviour that is easy to break (unchanged) |
| 29 | Every other easy-to-break bullet lives in its directory's `AGENTS.md` | Root, Behaviour that is easy to break |
| 30 | Known limits: treated as decided; one joins when declined for good; may still be reported as must with new impact | Root, Known limits (a sweep finding may join too, as the macOS runner did; nothing else loosened) |
| 31 | The eight known limits | Root, Known limits (unchanged), plus one from the sweep: unassigned macOS runners |
| 32 | README shape and order, nothing else | Root, Documentation rule (unchanged) |
| 33 | The key table mirrors `ui::help::KEYS`; change `KEYS` then the table; `tests/readme.rs` | Root, Documentation rule |
| 34 | Feature depth, glyph meanings and data-source notes go in the `AGENTS.md` files, not the README | Root, Documentation rule |
| 35 | Every visible change to the main interface shows in `docs/details.png`: extend the demo fixture, rerun, commit in the same PR; missing or stale is a should | Root, Documentation rule |
| 36 | Run `scripts/screenshots.sh`; it needs freeze, Google Chrome and the FiraCode Nerd Font | Root, Documentation rule |
| 37 | `docs/details.png` keeps the Nerd Font icons; `docs/plain.png` shows the plain ones | Root, Documentation rule |
| 38 | Why the script goes through an SVG and Chrome, and the Menlo fallback | `src/ui/AGENTS.md`, Screenshots |
| 39 | The script runs `conveyor --demo` with `HOME` a tempdir through `env`, so the demo marker shows | `src/ui/AGENTS.md`, Screenshots |
| 40 | Fixture SHAs look real, not `33333333` | Root, Documentation rule |
| 41 | When behaviour changes, check `README.md` is still true; no prose for a feature, the card or help carries it | Root, Documentation rule |
| 42 | Every pull request with a visible change embeds a commit-pinned screenshot; views `details.png` cannot show get a `shoot` line; rerun before `gh pr ready` | Root, Working agreements (as at the base) |
| 43 | A PR that reorganizes the code or adds a flow carries a small Mermaid diagram, before and after for a reorganization | Root, Working agreements (as at the base) |
| 44 | Draft PR first: the first ship opens it, assigned, from the template; rewrite the title in plain language; Summary at most 3 sentences on 2 lines; re-read before merging without rewording the user's edits | Root, Working agreements (as at the base) |
| 45 | Catching up with `main` on a pushed branch is a merge, not a rebase | Root, Working agreements (as at the base) |
| 46 | Never push to `main` (except an empty repository's bootstrap commit); never force-push | Root, Working agreements (as at the base) |
| 47 | TDD is never skipped: every change to `src/` starts from a failing test seen red; same edit is not TDD; show the red run | Root, Working agreements (as at the base) |
| 48 | All guidelines live in `AGENTS.md` files; nothing binding only in private memory; "update the guidelines" edits the nearest one | Root, Working agreements |
| 49 | "Work autonomously": steps 1 to 6 and the four stop conditions | Root, Working agreements (unchanged) |
| 50 | Cadence: one branch per phase, one commit per task through `ship.sh`, one per review round, pause only for the retro; where retro outcomes go | Root, Working agreements |
| 51 | Hurdles: shape, tags, destinations, `- none recorded`; `tests/plans.rs` | Root, Working agreements (unchanged) |
| 52 | The sweep: when, what to look for, what each finding becomes, the `### Sweep` section, the report, the workflow, the skill | Root, Working agreements |
| 53 | Time and tokens: the row, who owns the numbers, the time and token fields, the backfill after the merge | Root, Working agreements; plus the counter-restart rule from the sweep |
| 54 | One worktree per branch: the commands, `--unset-upstream`, remove it after the merge; `conveyor-dev` follows the last `dev-install.sh` | Root, Working agreements; plus the scratch-file rule from the sweep |
| 55 | Review before ready: `/pr-review` first, both reviewers briefed, Claude runs the gates, no writes, never post, `gh pr ready` after | Root, Working agreements |
| 56 | Severity by reachability | Root, Working agreements (unchanged) |
| 57 | Two strikes, then redesign; ask when behaviour changes | Root, Working agreements (the #65 story: History below) |
| 58 | The gates run beside the branch, in a detached worktree | Root, Working agreements (unchanged) |
| 59 | Fixes go back to the builder, one ship per round, `src/` fixes red first | Root, Working agreements (unchanged) |
| 60 | Squash-merge when CI is green, the only method; watch the release run; squash commit takes GitHub's default | Root, Working agreements (the date: History below) |
| 61 | Two binaries: Homebrew `conveyor`, `conveyor-dev` from a worktree; never `cargo install` into PATH; `v0.0.0` and the frozen changelog are expected in a working tree | Root, Working agreements (as at the base) |
| 62 | Plan before code, plans in `plans/`, committed with the first ship with empty `## Outcome` and `## Hurdles`; `tests/plans.rs`; a state table for logic over external state, reviewed before code | Root, Working agreements (the #44 story: History below) |
| 63 | Strict TDD, outside-in, unit tests only for pure helpers | Root, Development |
| 64 | No code comments; names and tests carry the intent | Root, Development (enforcement: backlog) |
| 65 | No dead code; test-only `pub` items on `SEAMS`; `allow(dead_code)` only in `tests/support/mod.rs` | Root, Development (unchanged, plus the unused `pub fn` of `tests/support/mod.rs`) |
| 66 | The three gates, all required, through `scripts/gates.sh`; without tmux the e2e tests are skipped | Root, Development (unchanged) |
| 67 | A toolchain bump: the channel, `toolchain:`, `components:`, `rust-version`, the locked `rust-overlay` (the docker command); `tests/toolchain.rs`; the `version` check refuses an unmoved overlay | Root, Development |
| 68 | The `tag` job stays on the image's `stable` | Root, Development (unchanged) |
| 69 | The flake reads `rust-toolchain.toml`, names no version, `profile = "minimal"` | Root, Development |
| 70 | `gates.sh` may skip the script tests outside CI; the paths that need them | Root, Development (the path list: `scripts/script-tests-skippable.sh`) |
| 71 | `check` calls `cargo test` itself, not `gates.sh`, which makes the skip safe | Root, Development |
| 72 | A script test that reads another path puts it on the script's list | Root, Development |
| 73 | The script tests need `jq` on `PATH` | Root, Development |
| 74 | How `tests/scripts/` shims `cargo` and `gh` (`record-call.sh`, `reply-call.sh`), and what `gates.sh` runs when it skips them | `tests/AGENTS.md` |
| 75 | Ship with `scripts/ship.sh "<message>"`: what it runs, refuses `main`, opens the draft PR | Root, Development |
| 76 | Do not hand-roll the chain, never pipe the gates, never chain them with `&&` and `;` before a commit | Root, Development (the incidents: History below) |
| 77 | Install: `dev-install.sh`; a milestone is done only after the gates and `ship.sh` ran or explained skipping `dev-install.sh` | Root, Development |
| 78 | When `ship.sh` skips the install; the list is `scripts/feeds-binary.sh`; embedding a listed file means taking it off the list | Root, Development (the path list: the script; timings: History below) |
| 79 | Releasing: the whole section | Root, Releasing (unchanged) |

### Scope at base and now

Every rule above keeps the section it had at the base, so it binds whom it bound. The rules that
changed section, and why their audience did not move:

| # | Section at base | Section now | Scope |
| --- | --- | --- | --- |
| 4 | Documentation rule | This repository is public | same: both bind every change |
| 12 to 21 | Layout (a file list) | Layout (a map) | same: descriptions, binding nobody new |
| 23, 25 | Layout (`tests/` lines) | Development | same: both describe how this repository is tested |
| 38, 39 | Documentation rule | `src/ui/AGENTS.md`, Screenshots | facts, not rules; the rule to run the script stays in the Documentation rule |
| 74 | Development | `tests/AGENTS.md` | facts, not rules |

A first draft moved rows 42 to 47 and 61 into new "Pull requests" and Development sections and
added a "Who follows what" section; review round 1 found that it narrowed or widened who the
owner's flow binds, so they are back in Working agreements and the audience question is left to
the owner.

## History moved out of `AGENTS.md`

The stories behind the rules, as the root file told them at d5578ca.

- Two strikes, then redesign: #65 took five rounds of patches and converged only once the check
  asked cargo instead of parsing lines.
- State tables: on #44, review rounds 1 to 3 each found pod states such a table would have
  listed up front.
- Squash is the only merge method since 2026-11-12. A merge that resolved conflicts once stopped
  GitHub from rebase-merging (phase 28).
- The My PRs tabs of phase 40 shipped without a screenshot.
- Hand-rolled gates: in zsh `PIPESTATUS` is undefined and `test "" -eq 0` is true, so a
  `gates.sh | grep` guard passed a clippy failure into a commit on 2026-09-10. Piping
  `gates.sh | tail` once hid a `cargo fmt --check` failure in tmux-agents. Chaining the gates
  with `&&` and then `;` before `git commit` let an unformatted bootstrap commit through.
- Measured: the `scripts` test target was about half of a warm run, 50 s against 23 s for the
  test step (2026-11-12); a no-op `dev-install.sh` took about 0.3 s, 17 s after a `src/` change
  and 23 s from a cold `target/` (2026-11-10). The default rustup profile's `rust-docs` doubled
  the macOS `flake` job.

## Outcome

### Sweep

Phases 40 to 68 (the first sweep; no earlier `### Sweep`, so every phase with a `## Hurdles`).
Read: the report of `scripts/sweep-report.sh` (hurdles by tag: review-loop 28, tooling 16,
process 12, test-seam 10, ci 8, api-shape 4, design 4), every bullet of every `## Hurdles` from
phase 40 to 68, and `plans/metrics.md`.

- An edit made while `ship.sh` ran was committed by its `git add -A` (62, 68; 68 shipped a red
  test) → rule in root `AGENTS.md` (Development: do not touch the worktree while a ship runs),
  and backlog: `ship.sh` refuses to commit when the tree changed since its gates started.
- `ship.sh` piped through `tail` against the rule (58, 59, 61) → rule in root `AGENTS.md`
  (Development: keep the output in a log file named for the branch instead). Enforcement
  declined: agents run without a terminal, so `ship.sh` cannot tell a pipe from a capture.
- Sibling agents shared one scratchpad: a PR body overwritten, ship logs mixed, `pgrep -f
  scripts/ship.sh` matched other worktrees (58, 59, 62) → rule in root `AGENTS.md` (Working
  agreements: scratch files carry the branch in their name; wait on your own ship's process).
- Builders' token counters restart, so "running total" was not one number (58, 59, 61, 64, 66,
  68) → rule in root `AGENTS.md` (Time and tokens: list every figure in order and mark the
  restart, never sum or pick one silently).
- The macOS runner was not assigned in time (62's release run, 68's `flake` job; 63's leg slow)
  → known limit in root `AGENTS.md`, pointing at the recovery in `.github/AGENTS.md`.
- Parallel phases conflict when catching up with `main` (47, 54, 59), and on the `PLAN.md` and
  `plans/metrics.md` rows at every merge → backlog: rows that do not collide (one file per row,
  or rows a phase never edits after its own).
- A test written after its code (58, 66) → declined: "TDD is never skipped" already says so,
  and both were caught and redone; a new rule would repeat it.
- "Bounds stricter than what breaks" twice in 68 → declined: 68 itself made it a rule.
- No Codex or `/pr-review` in the session (41, 56) → declined: the cloud session's tools, not
  the process; step 3 already requires the review.
- Clippy refused a ship after only a targeted `cargo test` (67) → declined: "done" already
  means the gates, and `ship.sh` caught it.
- A test binary died once with exit 101 and no failing test (67) → declined: seen once; a
  second time makes it an issue.
- Tab labels that cannot be told apart, both left `→ none`: a long custom title pushes later
  columns off the narrow layout's bar (41), two `[[repo]]` with the same short name draw two
  identical tabs (42) → backlog: tab labels stay distinct and on screen.
- Review rounds found states the plan had not listed (40, 42, 44, 48, 49, 51, 52), each fixed in
  code → declined: "Plan before code" already asks for the state table up front, a rule born of
  phase 44; the later ones are fixes, not a missing rule.
- A phase closed without its outcome (43, found in 56) → declined: already a rule, and
  `tests/plans.rs` fails on a plan without `## Outcome` since 56.
- Tests that slept on the real clock or raced a deadline (44, 47), both fixed in code →
  declined: no third case, and each fix removed its own cause.
- The `version` check judges with `main`'s `scripts/feeds-binary.sh`, so a new path outside the
  binary needs a pull request of its own first (55, 57) → declined: by design, since
  `version.yml` runs `main`'s scripts and reads the pull request only as data
  (`.github/AGENTS.md`, `version.yml`); 55 moved its file under `.github/` instead.
- A tool missing from the machine or session: `freeze` (40), `fontTools` work (50), what a cloud
  session cannot read (57) → declined: 40 became the screenshot rule, the rest are one-offs of
  the environment.
- Error kinds never seen live (47, twice) → declined: one phase; the kinds rest on what `gh` and
  `kubectl` print, and a live case becomes an issue.
- The notes of v0.24.2 to v0.30.1 still list too much (45, `→ none`) → declined: seen once, and
  rewriting them waits for the owner's yes, as phase 45 records.
- Review rounds per phase (58: 4, 59: 4, 61: 2, 62: 3, 63: 1, 64: 2, 66: 2, 67: 1, 68: 4) and
  reviewer tokens per round (about 100k to 150k) show no upward drift; the four-round phases
  are the two redesigns (58, 59) and 68's product decision. Builder tokens stay at about 140k
  to 170k per hand-back. Declined: nothing to act on. The Codex reviewer and the orchestrator
  are `unknown` in every row, which no tool here can read.

## Hurdles
