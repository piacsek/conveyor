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

The layout names what each file or directory is for; read its types, functions and variants
from the code, and its rules from the `AGENTS.md` of its directory.

```
src/main.rs      CLI dispatch, config load, wiring
src/cli.rs       the subcommands and flags; hand-rolled, no clap
src/config.rs    the config: its tables, defaults, path and `CONVEYOR_CONFIG` override
src/sources/     everything that reaches out to an external service, and nothing else
src/sources/github.rs the Github trait and CliGh, which spawns `gh` (and `git` for the checkout guard), error mapping
src/sources/kube.rs   the Kube trait and CliKubectl, which spawns `kubectl` under a deadline that kills it
src/sources/fetch.rs  what each stage fetches, the sha → pull request cache, Deployed read per environment on parallel threads
src/sources/queries/  GraphQL documents, `include_str!`ed
src/sources/AGENTS.md GitHub and kubectl facts: API shapes and queries, the sha → pull request lookup, the spawn lock, call logging, the kubectl deadline
src/model/prs.rs My PRs: pull requests and their checks, review and merge state; lenient parse of gh JSON
src/model/queue.rs Merge queue: entries and the merge-group runs attached to them
src/model/builds.rs Main builds: runs and the pull requests they came from
src/model/deployed.rs Deployed: deployments, the sha in an image tag, pods (problems first), health
src/model/jobs.rs the jobs of a run and the failed step
src/model/log.rs  the failed step's log as gh prints it, minus job/step/timestamp prefixes and ANSI
src/model/AGENTS.md check dedup, merge-group run attach rules, deploy health rules, jobs parsing, lenient parsing
src/app/mod.rs   App, its modes, inputs, actions and requests; re-exports everything below as `app::X`
src/app/stages/  one type per stage behind the StageView trait: `mod.rs` is the trait and the dispatch, one file per stage for everything that stage does
src/app/stages/mod.rs the trait, what a stage may read from the rest of App, and the one match from a Stage to its field
src/app/stages/prs.rs My PRs: what it offers and where its rows lead
src/app/stages/queue.rs Merge queue: its merge-group runs and where its rows lead
src/app/stages/builds.rs Main builds: its runs, what it offers and where its rows lead
src/app/stages/deployed.rs Deployed: a row's own error, where its rows lead, keeping the last known sha
src/app/browser.rs where a row leads, and opening it with a debounce
src/app/navigation.rs moving around: the focused readers, refocus, the filter and the scroll helpers
src/app/column.rs the column every stage holds (state, error, list, filter), row order, what every column answers whatever its row type
src/app/jobs.rs  the jobs and log state: which runs need their jobs, aging, the log pane
src/app/actions/ the `S` popup: `mod.rs` is the machinery, one file per action for how it runs; what a column offers is its stage's `menu`
src/app/actions/mod.rs the popup machinery: menus, offers, pending confirms, writes in flight
src/app/actions/writes.rs the writes in flight, one per row, and their footer and card wording
src/app/actions/merge.rs merge
src/app/actions/update.rs update the branch, and its conflicts
src/app/actions/status.rs the status changes (ready, draft, close, auto-merge, the merge queue) and their wording
src/app/actions/checkout.rs check out locally
src/app/actions/failures.rs the mark a failed write leaves on its card, and when it clears
src/app/actions/rerun.rs rerun the failed jobs
src/app/keymap.rs the key handlers per mode, `Esc`
src/app/run.rs   run(): draw, read one Input, turn each Action into a Request
src/app/AGENTS.md keys, `Esc`, the filter, the `S` popup and each action, failed write marks, jobs state, refresh and follow-up, row order, open debounce, zoom, scroll
src/ui/mod.rs    draw: 4 columns or tabs below `4 × ui.min_column_width`
src/ui/style.rs  the shared vocabulary: bars, indents, dim, glyphs and status words, short repo and sha
src/ui/card.rs   a card: one title line plus indented span lines
src/ui/columns.rs column titles, blocks and lists
src/ui/link.rs   the `#N` of a drawn card as an OSC 8 terminal hyperlink
src/ui/rows.rs   one card builder per stage
src/ui/details.rs the `d` pane per stage, including the job list; no URLs
src/ui/footer.rs footer text, the refresh spinner and the static logo
src/ui/dialog.rs the `S` popup and the confirm dialog
src/ui/help.rs   KEYS is the generic table the README mirrors; the `p`/`b` wording swapped per column
src/ui/AGENTS.md  cards, glyphs and icons, dialogs, the details, help and changelog panes, the footer, colours
src/refresh.rs   one fetcher thread per stage, sleeping until the interval or a refresh
src/open.rs      opening a URL and copying to the clipboard: the OS seam, not a service, so it stays out of `sources/`
src/tmux.rs      inside tmux, asks once at start whether the attached client keeps the `#N` links, and the fix the help pane shows when it drops them: a local seam like `open.rs`, so it stays out of `sources/`
src/changelog.rs the bundled CHANGELOG.md and the rules a release entry must meet
src/ui/changelog.rs the `c` pane: wrapped to its width (bullets hang), headings bold without their `#`, scrolled like the details pane
src/log.rs       the call log: its path, per program name, rotated on open
src/text.rs      text helpers: truncation and wrapping with `…`, ages, durations and clock times, ANSI stripping
scripts/gates.sh              the quality gates; fails loudly, never pipe it through tail
scripts/scrub-check.sh        denylist grep over tracked files (see "This repository is public")
scripts/dev-install.sh        release build symlinked as ~/.local/bin/conveyor-dev
scripts/ship.sh               gates + dev-install (unless skippable) + commit + push, aborts on any failure; the first push of a branch opens the draft PR (commit subject as title, the template as body)
scripts/dev-install-skippable.sh  exit 0 with the reason when `ship.sh` may leave `conveyor-dev` alone, else 1 with what needs the install
scripts/script-tests-skippable.sh exit 0 with the reason when `gates.sh` may leave out the `scripts` test target, else 1 with the first path that needs it
scripts/feeds-binary.sh       reads paths on stdin, prints the ones that feed the binary; the one list of what does not (`tests/`, `docs/`, `changes/`, …)
scripts/fragments.sh          sourced, not run: reads the pending fragments and refuses a malformed or repeated one, at the commit its caller names
scripts/version-check.sh      `<base> <head>`: the `version` job of `version.yml`; reads the head as git objects, runs nothing from it
scripts/next-version.sh       prints the version the pending fragments ask for, nothing when none is pending
scripts/release-commit.sh     `<version>`: writes the release into `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md` and confirms it reads back; commits nothing
scripts/release-rewrite.sh    sourced, not run: the release rewrite both `release-commit.sh` and `version-check.sh` use
scripts/releases.sh           sourced, not run: the state of a tag's release
scripts/release-unfinished.sh `<tag>`: whether `build` and `publish` still have work, so a rerun never rebuilds a published release
scripts/release-tag.sh        the `tag` job of `release.yml` (refuses outside GitHub Actions): resume an unfinished tag, else tag what is pending, else nothing
changes/                      one fragment per pull request that feeds the binary (`kind:` and the changelog bullets); add-only, see Releasing
plans/                        one versioned plan file per phase (see Working agreements)
scripts/homebrew-formula.sh   prints the tap formula for a released version
scripts/formula-version.sh    the `tap` job's decision; never a version below the formula's
scripts/screenshots.sh        renders docs/details.png from a gh shim + synthetic fixture (truecolor.py converts ANSI)
tests/           outside-in: `tests/cli.rs` runs the real binary; TUI tests drive run() with a TestBackend;
                 `tests/readme.rs` pins the README shape and its key table to `ui::help::KEYS`;
                 `tests/source_size.rs` fails while any Rust file under `src/` or `tests/` is longer than 700 lines, naming it;
                 `tests/stage_dispatch.rs` fails while a match arm on a `Stage` variant sits in `src/app/` outside `stages/mod.rs`, naming the line;
                 `tests/dead_code.rs` fails while a `pub` item under `src/` has its name nowhere else in `src/` (a name scan, not a resolver) and is not on its `SEAMS` list, while a seam on that list is no longer declared or used by a test, while a `pub fn` of `tests/support/mod.rs` is used by no test, or while a `dead_code` attribute sits outside `tests/support/mod.rs`, naming each;
                 a test file that outgrows the limit becomes a folder (`tests/<name>/main.rs` plus one file per topic), so it stays one binary
tests/AGENTS.md  the testing traps hit so far, the e2e `gh` shim dispatch note
.github/AGENTS.md CI: what the workflows run and why (`check`, `audit`, the `version` check, the release queue, the tap), the `main` ruleset, branch cleanup; the release internals
.github/pull_request_template.md  `## Summary` and `## Changes`: the body `ship.sh` opens a draft PR with
```

## Where the rest lives

This file holds the rules and agreements. The facts about each directory live in an `AGENTS.md` next to the code; read the one for a directory before changing anything in it.

| File | What it holds | Read it |
| --- | --- | --- |
| `src/sources/AGENTS.md` | GitHub and kubectl: API shapes and queries, the sha → pull request lookup (`Pulls`), the spawn lock, call logging, `gh` never inheriting the terminal, the kubectl deadline | before touching `src/sources/` or anything that spawns a program |
| `src/model/AGENTS.md` | check dedup (`newest_checks`, superseded runs, `overall`), merge-group run attach rules, deploy health rules, jobs parsing, lenient parsing | before touching `src/model/` or a fixture it parses |
| `src/app/AGENTS.md` | keys, `Esc`, the filter, the `S` popup and each action (`m`, `u`, `f`, `t`/`a`/`e`/`x`, `c`), failed write marks, jobs state (`age_jobs`, `jobs_needed`), refresh and follow-up, row order, open debounce, zoom, scroll | before touching `src/app/`, `src/refresh.rs` or `src/open.rs`, or adding a key or an action |
| `src/ui/AGENTS.md` | cards, glyphs and icons, dialogs, the details, help and changelog panes, the footer, colours | before touching `src/ui/` or `src/changelog.rs`, or changing anything visible |
| `tests/AGENTS.md` | the testing traps hit so far, the e2e `gh` shim dispatch note | before writing or debugging a test, a shim or a launcher of the binary |
| `.github/AGENTS.md` | CI: what the workflows run and why (`check`, `audit`, the `version` check, the release queue, the tap), the `main` ruleset and its required checks, the unguarded `v*` tags, branch cleanup; the release internals: every refusal of the `version` check, the release run, recovering a failed or stuck run, pins and permissions | before touching `.github/`, a required check or a release script, or when a release run fails |

## Behaviour that is easy to break

The two bullets about `src/cli.rs` and `src/config.rs` stay here; every other one is in the `AGENTS.md` of the directory it is about.

- **`--help`/`-h`/`help` and `--version`/`-V`/`version`** print to stdout and exit 0 before
  the config is read, so a broken config never hides them.
- **Config is loaded before any subcommand runs.** Missing file = `Config::default()`; an
  unreadable file or unknown key is an error naming the file (`deny_unknown_fields` on every
  table). `conveyor config` exits 1 on it and doubles as the reference for defaults.

## Known limits

Accepted limits a reviewer treats as decided, one line each, with where they are written down.
Pointing reviewers here is not priming, since the list is in the repo. A limit joins it when a
review finding is declined for good; a reviewer may still report one as must when it shows new
impact.

- **A pull request number is the row key**: the same number in two repositories shares the busy
  spinner and the failure mark, one blocking the other (`src/app/AGENTS.md`, the `S` popup and
  "A failed write marks the card").
- **Widths are counted in characters, not display width**: wide characters (CJK, emoji) can be
  clipped at a dialog's border, and the footer cuts titles the same way (`src/ui/AGENTS.md`,
  dialogs; `PLAN.md` backlog).
- **The footer shows only the latest notice**: a success that lands under a prompt is hidden by
  it; a failure stays on its card (`src/app/AGENTS.md`, "A failed write marks the card";
  `PLAN.md` phase 39).
- **A checkout can hang on ssh**: a key that needs a passphrase and has no agent prompts on
  `/dev/tty`, and a stalled fetch has no deadline (`src/app/AGENTS.md`, the checkout).
- **Some commits cost a call on every refresh**: a sha the batch answers `null` for and REST
  answers 422, and an upper-case sha (`src/sources/AGENTS.md`, `Pulls`).
- **The `version` check judges "feeds the binary" by path only**, can be pointed by cargo at
  manifests outside its copy, and refuses a fragment name git would quote
  (`.github/AGENTS.md`, `version.yml`).
- **`check` builds and tests on Linux only**: a macOS-only break fails in the release run
  (`.github/AGENTS.md`).
- **No tag ruleset guards `v*` tags**: the owner can still push or delete one by hand
  (`.github/AGENTS.md`).

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
  3. `/pr-review` the PR (see "Review before ready": what counts as must or should, when to
     redesign, where the reviewer runs the gates, who fixes). Fix every must and should
     finding, TDD like any change, ship once per round, and review again. Minor findings are
     fixed when cheap, else listed in the PR body. The loop ends only on a round with no must/should findings, and that
     round is **unprimed**: the reviewers get the PR, the repo and this file, never the list
     of earlier findings or what was decided about them. Never post the review on the PR.
  4. The phase outcome is in `PLAN.md` and, when the branch feeds the binary, its fragment
     `changes/<branch>.md` is on it (no version bump, no `CHANGELOG.md` edit). Re-read the PR
     title and body, fix what went stale, `gh pr ready`, wait for CI green.
  5. Squash-merge, the only method `main` allows. Watch the `release` run the
     merge starts to green; with a fragment, check the release holds the six assets and the
     tap formula names the new version (see "Releasing"). Never tag by hand.
  6. Remove the worktree and report what shipped, with the release link.
  Stop and ask instead of proceeding when: a gate fails for a reason the change did not
  cause, CI fails twice on the same step, a review finding needs a product decision, or the
  next step would force-push, push to `main` or delete a release.
- **Cadence.** Outside the autonomous flow above, work runs under `/my-tdd autonomous`: one branch per phase (`phase-N-<topic>`),
  one commit per task through `scripts/ship.sh` (the fixes of a review round are the exception:
  one commit per round, see "Review before ready"), no pause between tasks, a pause only for the
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
  file and tell them to read its "Known limits" before reporting, have the Claude reviewer run `scripts/gates.sh` itself rather than trust the claim
  that the gates pass, and forbid edits, commits, and any write to GitHub or call to the
  clusters. When the skill asks whether to post the review on the PR, answer no.
  `gh pr ready` comes after the findings are addressed, not before.
  - **Severity by reachability.** A finding is must or should only when the user hits it while
    using conveyor, the release pipeline hits it in normal operation, or a pull request could
    execute code or reach a token or secret. A bypass that needs a deliberately forged file,
    which the owner would still have to merge, is minor: fixed when cheap, else listed in the
    PR body. The reviewers are given this definition.
  - **Two strikes, then redesign.** When a second round finds another hole in the same
    mechanism, stop patching it: step back and pick a simpler design that removes the class of
    finding, and ask the user when that changes behaviour. #65 took five rounds of patches and
    converged only once the check asked cargo instead of parsing lines.
  - **The gates run beside the branch, not in it.** The Claude reviewer runs
    `scripts/gates.sh` in its own detached worktree of the PR head (`git worktree add --detach
    <path> <sha>`, removed afterwards), never in the branch's worktree, so fixes do not wait
    for its run.
  - **Fixes go back to the builder.** The fixes of a round go to the agent that built the
    branch, with its context intact, not to a fresh one, and land as one `ship.sh` per round
    of findings rather than one per finding: the review-round exception to "one commit per
    task". Each fix that touches `src/` still starts from a failing test.
- **Merging and releasing.** Squash-merge the phase PR into `main` when CI is green, then
  watch the `release` run the merge starts (see Releasing): no bump, no `CHANGELOG.md` edit,
  no tag by hand. Squash is the only merge method the repository and the `main` ruleset allow
  (since 2026-11-12, see `.github/AGENTS.md`). The squash commit on `main` takes GitHub's
  default here: the pull request title followed by `(#N)` (a pull request of a single commit
  gets that commit's subject instead), with the per-task commit messages listed in its body;
  the per-task commits stay readable on the pull request. Never push to `main` directly except the very first bootstrap commit of an empty
  repository, and never force-push.
- **Catching up with `main` on a pushed branch is a merge, not a rebase.** A rebase rewrites
  the pushed commits and the next `scripts/ship.sh` push is refused; the no-force-push rule
  then leaves only `git reset --hard origin/<branch>`, `git merge origin/main` and a
  cherry-pick of the new work. The squash into `main` leaves the merge commit behind anyway.
  Squash is the only method partly for this reason: a merge that resolved conflicts once stopped
  GitHub from rebase-merging (phase 28), and a rebase-merge, which replays the branch's own
  commits and drops its merge commits, could land a tree the `version` check never judged.
- **Two binaries.** `conveyor` on PATH is the Homebrew release; `conveyor-dev` is a working
  tree (`scripts/dev-install.sh`). Never `cargo install` the crate into a PATH directory. A
  build from a working tree (`conveyor-dev`, CI) reads `v0.0.0` in the footer and in
  `--version`, and its `c` pane shows the changelog as frozen on `main`, not the newest
  release: that is expected, only a release commit carries a real version.
- **Plan before code, plans in the repo.** Each phase starts from a written plan, re-planned
  after the previous retro, with its "Verified facts" checked against live tools before use.
  Plans are versioned here: write each one to its own file under `plans/` (for example
  `plans/phase-5-cards.md`) and commit it with the first ship of the phase. Never leave a plan
  only in an agent's private plan directory or memory. `PLAN.md` stays the index of phases,
  retros and the backlog; a phase's outcome is appended there when it closes. A plan that adds
  logic over external state (pod states, check states, merge states, write results) carries a
  table of the states and what the card shows for each, and that table is reviewed before any
  code is written: on #44, review rounds 1 to 3 each found pod states such a table would have
  listed up front.

## Development

Strict TDD: one failing test, minimal code, refactor. Outside-in: start from the binary or
from `run()` with scripted inputs, drop to unit tests only for pure helpers. No code
comments; names and tests carry the intent.

Dead code is strictly forbidden. An item nothing calls is deleted in the same change that
orphans it, never kept "for later". A `pub` item that only tests call is a test seam and must
be on the `SEAMS` list in `tests/dead_code.rs`. `#[allow(dead_code)]` appears nowhere except
`tests/support/mod.rs`, where each test binary uses only some of the helpers.

Gates, all required before a task is done, wrapped by `scripts/gates.sh`:

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -- --include-ignored   # one pass; the ignored ones are e2e: real binary, `gh` shim first on PATH
```

Without tmux the last gate is a plain `cargo test` and a notice that the e2e tests were skipped.

Outside CI (`CI` unset), `gates.sh` leaves out the `scripts` test target (`tests/scripts/`,
about half of a warm run: 50 s against 23 s for the test step, measured 2026-11-12) when
`scripts/script-tests-skippable.sh` finds nothing between the merge base with `origin/main` and
the working tree, untracked files included, under `scripts/`, `tests/scripts/`,
`tests/fixtures/`, `.github/`, `changes/`, or in `Cargo.toml`, `Cargo.lock` or `CHANGELOG.md`:
the paths those tests read from this repository. It then runs `cargo test --lib --bins` with a
`--test` for every other target it finds as `tests/*.rs` or `tests/*/main.rs`, and
`cargo test --doc`, and prints `==> skipping the script tests: <reason>; CI runs them`. No
repository, no `origin/main` or no merge base runs everything. The skip is safe because the
`check` job of `ci.yml` calls `cargo test --locked -- --include-ignored` itself, not `gates.sh`,
and `check` is required to merge into `main`. A script test that starts reading another path of
this repository puts that path on the list in the script.
`tests/scripts/` runs the scripts themselves in a scratch directory and a scratch git
repository, with `cargo`, `gh` and the scripts they call replaced by symlinks to
`tests/fixtures/record-call.sh`, which appends its name and argv to `$CALLS_LOG` and exits 1 for a call whose argv starts with `$FAILING_CALL` (how a ship test says the branch has no pull request yet). Where a
script reads what `gh` prints (`release-tag.sh`, `release-unfinished.sh`, `formula-version.sh`), `gh` is a symlink to `tests/fixtures/reply-call.sh`
instead: it records the same line, exits 1 with `$FAILING_STDERR` on stderr for a call whose
argv starts with `$FAILING_CALL`, else prints every file in `$REPLIES` whose name is part of
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
included, is outside `tests/`, `docs/`, `plans/`, `scripts/`, `changes/`, `.github/`, an `AGENTS.md` at any depth (`src/` included),
`PLAN.md`, `README.md`, `LICENSE` and `.gitignore`. That list lives in `scripts/feeds-binary.sh`, the one
definition of "feeds the binary", which `dev-install-skippable.sh` calls. Everything else counts as feeding the
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

The pipeline picks the version; a pull request never does. `main` carries the constant
`version = "0.0.0"` in `Cargo.toml` and a `CHANGELOG.md` frozen at the last hand-made entry
(v0.24.1); neither is edited by a pull request. The version lives in the `v*` tags.

**A pull request says what kind of change it is in a fragment**, one added file
`changes/<branch>.md` (a branch name holding `/` uses `-`):

```markdown
kind: fix

- Main builds appears much sooner on the first load.
```

- `kind` is `fix` (patch), `feat` (minor) or `internal` (patch). The bullets are the changelog
  text, written for users, one bullet each; an indented line continues a bullet. `internal` is
  for a change nobody can observe in the binary (a refactor, a dependency bump) and may have no
  bullet; a release made only of those reads `- v<version>: internal changes only.`
- **No majors.** No kind produces one; a config-format break is a decision made by hand later.
- **A fragment is required exactly when the pull request touches what feeds the binary**
  (`scripts/feeds-binary.sh`, the same list `ship.sh` uses for the dev install). Such a pull
  request is always released, a refactor as an `internal` patch. One that touches only tests,
  docs, plans, scripts, CI or guidelines needs none and releases nothing.
- **`changes/` is add-only.** A fragment is never edited, renamed or deleted once it is on
  `main`: "pending" means "not in the tree of the last tag", so a changed released fragment
  stops every release and a reused name can skip one silently. Put a correction in a new file.
- `tests/scripts/next_version.rs` runs `scripts/next-version.sh` on this repository, so a
  malformed committed fragment fails the gates (it reads `HEAD`, so locally only after the
  commit; CI always sees it). It needs the tags, like `every_released_tag_keeps_its_entry`.

**The `version` check enforces these rules on every pull request into `main`.** It is a required
check, so a pull request it refuses cannot merge. It refuses one that sets the version in
`Cargo.toml` or `Cargo.lock` or leaves either in a shape the release cannot rewrite, changes
`CHANGELOG.md`, edits, deletes or renames a fragment, adds a malformed or repeated one, or feeds
the binary without one, naming the file and what to do instead. The full list, how it reads a
pull request without running it, the release run, and what to do when a run fails or a tag can
never build are under "Releasing internals" in [`.github/AGENTS.md`](.github/AGENTS.md).

**A revert keeps the fragment.** A pull request that reverts a merged one keeps that pull
request's fragment (GitHub's Revert button deletes it, which the check refuses, since
`changes/` is add-only) and adds a new fragment for the revert itself.

**After a merge**, watch the run (`gh run list --workflow release.yml --limit 3`, then
`gh run watch <id> --exit-status`). For a pull request with a fragment, check that the release
holds the six assets and that the tap formula names the new version:

```bash
gh release view --json tagName,assets --jq '.tagName, .assets[].name'
gh api repos/piacsek/homebrew-tap/contents/Formula/conveyor.rb -H 'Accept: application/vnd.github.raw+json'
```

A cancelled `release` run is normal (merges in quick succession queue up); a red one is not: see
"When a run fails or a job is stuck" under "Releasing internals" in `.github/AGENTS.md`.

Never push a `v*` tag by hand: tags come only from the `tag` job of the release run. Nothing
enforces that for the owner; why no tag ruleset does is in `.github/AGENTS.md`.

Never move or delete a tag that has a release: the next run would rebuild and republish it
and rewrite the tap formula for that old version. If a released version is wrong, release a
fix on top.

Semver: `feat` for a new column, key or config key (minor), `fix` for a fix (patch), `internal` for the rest (patch); no majors for now.
