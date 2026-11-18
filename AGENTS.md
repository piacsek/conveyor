# AGENTS.md

Notes for agents and humans working on `conveyor`. Keep this file short and
factual; verify against the code before treating anything here as live state. In every
`AGENTS.md`, a rule written from now on carries its reason in at most one sentence and its
history goes to the phase's plan file under `plans/`; longer paragraphs already there are not
violations to fix in passing.

## What this is

A Rust + Ratatui TUI that shows where a change is on its way to production:
my open pull requests, the merge queue, the last main builds, and what each
environment runs. One column per stage, side by side; a focused column,
actions on the selected row. `conveyor config` prints the effective config.
`PLAN.md` is the index of phases and the backlog; each phase's plan, outcome and hurdles are
in `plans/`. An outside contributor starts from `.github/CONTRIBUTING.md`.

## This repository is public

No employer data, ever: no org or repo names, PR numbers or titles, logins, hostnames, cluster
or kube-context names, ticket prefixes. Fixtures come from this repo's own public PRs;
screenshots and TUI tests use invented `acme/*` data; plans and docs use `<owner>/<repo>`
placeholders. Never screenshot real data.

`scripts/ship.sh` runs `scripts/scrub-check.sh` against the local denylist at
`~/.config/conveyor-dev/denylist` (or the file `CONVEYOR_DENYLIST` names) three times: as the
first gate and again between `git add -A` and `git commit`, over the contents and the paths of
the index and of the working tree's files that are not ignored; and between `git commit` and
`git push` (`--outgoing`), over the branch name and, for every commit no remote has yet, its
message, the paths it adds or keeps and the lines it adds (a symlink's target and a merge's
conflict resolution included), never a removed line or a deleted path, so a branch can remove a
term `main` still holds. Each match says which of the four (message, path, added line, branch
name) it was in and names its commit. `tests/scripts/scrub_check.rs` pins each case.

- A refused push leaves the commit local: `git reset --soft HEAD~1`, fix what is named
  (`git branch -m` for the branch name), ship again; an earlier commit is rewritten by hand.
- A refused merge is kept and fixed with `git commit --amend`: with nothing staged and commits
  no remote has, `ship.sh` pushes them without a new commit.
- The list is kept outside the repo on purpose, so CI does not enforce it. The check is skipped
  only where the default path is confirmed absent (the nearest existing path above it is a
  directory it can search). It fails instead, in both modes, when the path is there but is not
  a readable file, when its absence cannot be confirmed, when `CONVEYOR_DENYLIST` names one that
  is not, when the list is empty, or when `git grep` or `git log` cannot run.
- It does not read the contents of binary files, the author and committer of a commit, the PR
  body, or anything pushed without `ship.sh`.

## Layout

A map, not an inventory: what each file holds is in the code, and each directory's rules are in
its `AGENTS.md`. `tests/layout.rs` fails while a path named in the map below does not exist; it
recognises a word holding a `/` or ending in `.rs`, `.sh`, `.md`, `.toml`, `.lock`, `.nix`,
`.json` or `.yml`, and checks nothing in the prose or of any other shape.

```
src/main.rs        CLI dispatch, config load, wiring
src/cli.rs         the subcommands and flags; hand-rolled, no clap
src/config.rs      the config; src/config/values.rs holds what a loaded value must meet
src/sources/       everything that reaches out to an external service (`gh`, `kubectl`), and nothing else; `--demo` answers from src/sources/demo/
src/model/         parsing and judging what `gh` and `kubectl` answer
src/app/           App, its modes, keys and actions
src/app/stages/    one type per stage behind the StageView trait; the one match from a Stage to its field is in src/app/stages/mod.rs
src/app/actions/   the `S` popup: src/app/actions/mod.rs is the machinery, one file per action; what a column offers is its stage's `menu`
src/ui/            drawing; src/ui/help.rs holds KEYS, which the README mirrors
src/open.rs        opening a URL and the clipboard: the OS seam, not a service, so it stays out of src/sources/
src/refresh.rs     one fetcher thread per stage
src/worker.rs      one thread per kind of request
scripts/gates.sh   the quality gates; never pipe it
scripts/ship.sh    gates, dev install, scrub, commit, push, the draft PR on the first push
scripts/feeds-binary.sh the one list of what does not feed the binary
scripts/flake-inputs.sh the one list of what the flake build reads
scripts/version-check.sh the `version` check; reads the head as git objects, runs nothing from it
changes/           one fragment per pull request that feeds the binary; add-only, see Releasing
plans/             one plan per phase, plans/metrics.md, plans/unphased.md; plans/original-plan.md is history, not live state
tests/             outside-in, and the tests that hold the repository to its rules:
tests/readme.rs    the README's shape and its key table
tests/plans.rs     the plan files, their index and their metrics rows
tests/layout.rs    this map
tests/toolchain.rs one Rust version everywhere
tests/source_size.rs no Rust file over 700 lines
tests/stage_dispatch.rs no match on a Stage in src/app/ outside src/app/stages/mod.rs
tests/dead_code.rs no unused `pub` item
docs/              the screenshots and the README's recording
scripts/demo.sh    records docs/demo.gif from scripts/demo.tape
.github/           the workflows, the issue forms, the PR template; .github/CONTRIBUTING.md points here and restates none of the rules
.claude/skills/    the `sweep` and `adversarial-review` skills
```

## Where the rest lives

This file holds the rules and agreements. The facts about each directory live in an `AGENTS.md` next to the code; read the one for a directory before changing anything in it.

| File | What it holds | Read it |
| --- | --- | --- |
| `src/sources/AGENTS.md` | GitHub and kubectl: API shapes and queries, the sha → pull request lookup (`Pulls`), the spawn lock, call logging, `gh` never inheriting the terminal, the kubectl deadline | before touching `src/sources/` or anything that spawns a program |
| `src/model/AGENTS.md` | check dedup (`newest_checks`, superseded runs, `overall`), merge-group run attach rules, deploy health rules, jobs parsing, lenient parsing | before touching `src/model/` or a fixture it parses |
| `src/app/AGENTS.md` | keys, `Esc`, the filter, the `S` popup and each action (`m`, `u`, `f`, `t`/`a`/`e`/`x`, `c`), failed write marks, jobs state (`age_jobs`, `jobs_needed`), refresh and follow-up, row order, open debounce, zoom, scroll | before touching `src/app/`, `src/refresh.rs` or `src/open.rs`, or adding a key or an action |
| `src/ui/AGENTS.md` | cards, glyphs and icons, dialogs, the details, help and changelog panes, the footer, colours, how the screenshots are rendered | before touching `src/ui/`, `src/changelog.rs` or `scripts/screenshots.sh`, or changing anything visible |
| `tests/AGENTS.md` | the testing traps hit so far, the e2e `gh` shim dispatch note, how `tests/scripts/` stands in for `cargo` and `gh` | before writing or debugging a test, a shim or a launcher of the binary |
| `.github/AGENTS.md` | CI: what the workflows run and why (`check`, `audit`, the `version` check, the release queue, the tap), the `main` ruleset and its required checks, the unguarded `v*` tags, branch cleanup, the issue forms and Discussions; the release internals: every refusal of the `version` check, the release run, recovering a failed or stuck run, pins and permissions | before touching `.github/` (the issue forms and `CONTRIBUTING.md` included), a required check or a release script, or when a release run fails |

## Behaviour that is easy to break

The two bullets about `src/cli.rs` and `src/config.rs` stay here; every other one is in the `AGENTS.md` of the directory it is about.

- **`--help`/`-h`/`help` and `--version`/`-V`/`version`** print to stdout and exit 0 before
  the config is read, so a broken config never hides them.
- **Config is loaded before any subcommand runs.** Missing file = `Config::default()`; an
  unreadable file, an unknown key (`deny_unknown_fields` on every table), a value out of its
  range or a setting conveyor would ignore (`src/config/values.rs`) is an error naming the file.
  A value is refused only when something named breaks with it (a panic, an overflow, a call
  GitHub refuses, the rate limit, a row collision, a setting never read); a tidy bound is not one. `conveyor config` exits 1 on it and doubles as the reference for defaults.

## Known limits

Accepted limits a reviewer treats as decided, one line each, with where they are written down.
Pointing reviewers here is not priming, since the list is in the repo. A limit joins it when a
review finding is declined for good; a reviewer may still report one as must when it shows new
impact.

- **The footer shows only the latest notice**: a success that lands under a prompt is hidden by
  it; a failure stays on its card (`src/app/AGENTS.md`, "A failed write marks the card";
  `plans/phase-39-failed-write-marks.md`).
- **A checkout can hang on ssh**: a key that needs a passphrase and has no agent prompts on
  `/dev/tty`, and a stalled fetch has no deadline: the checkout is the one `gh` call without
  one (`src/app/AGENTS.md`, the checkout; `src/sources/AGENTS.md`).
- **A hung `gh` or `kubectl` outlives a closed terminal**: a call with a deadline runs in its own
  process group, so the terminal's hangup does not reach it and it runs until it exits by itself
  (`src/sources/AGENTS.md`, "A call in a process group of its own outlives a closed terminal").
- **Some commits cost a call on every refresh**: a sha the batch answers `null` for and REST
  answers 422, and an upper-case sha (`src/sources/AGENTS.md`, `Pulls`).
- **The `version` check judges "feeds the binary" by path only**, can be pointed by cargo at
  manifests outside its copy, and refuses a fragment name git would quote
  (`.github/AGENTS.md`, `version.yml`).
- **`check` builds and tests on Linux only**: the tests never run on macOS, and a macOS-only
  break of the release build itself fails in the release run; the flake's macOS leg catches a
  plain macOS build break (`.github/AGENTS.md`).
- **No tag ruleset guards `v*` tags**: the owner can still push or delete one by hand
  (`.github/AGENTS.md`).
- **A Nix outage blocks merges that touch the flake's inputs while the ruleset requires
  `flake-ok`**: a Nix, nixpkgs or `cache.nixos.org` outage then holds them until the ruleset is
  disabled; requiring it is the owner's step by hand (`.github/AGENTS.md`, the flake build, has
  the command that shows whether it is required).

## Documentation rule

`README.md` is minimal and holds only what a user needs, in this order and nothing else: the
banner `<img>` (it stands in for the h1; there is no `# ` heading), two sentences on what conveyor does, the one screenshot `docs/details.png` right below them (all
four columns with the details pane open), the one recording `docs/demo.gif` right below it, then `## Installation`, `## Dependencies` (what must
be installed and what each one is for, the optional Nerd Font included), `## Usage` (commands, the key
table, the default config), `## FAQ` (symptoms a user sees, each with its fix) and `## Development`. The key table mirrors `ui::help::KEYS` word
for word; `tests/readme.rs` fails when the README and the help view drift, when an image
other than the screenshot and the recording, a subsection or another section appears, or when the description grows past two
sentences (the help view is the single source: change `KEYS`, then the table). Feature depth, glyph meanings and data-source notes belong in the `AGENTS.md` files, not there.

**Every visible change to the main interface shows in `docs/details.png`**: a new or changed
card line, glyph, bar, title, column or footer element. Extend the demo fixture under
`src/sources/demo/` until the change appears on screen, rerun the screenshots, and commit the
image in the same pull request; a reviewer reports a missing or stale screenshot as a should.
Run `scripts/screenshots.sh` (needs `brew install charmbracelet/tap/freeze`, Google Chrome, and
the FiraCode Nerd Font in `~/Library/Fonts`) and commit the new images. `docs/details.png`
keeps the Nerd Font icons and `docs/plain.png` shows the default plain ones; how the script
renders them is in `src/ui/AGENTS.md`. Fixture SHAs look real (`8c1f2a7d…`), not `33333333`.
`docs/demo.gif` is recorded from `scripts/demo.tape` by `scripts/demo.sh` (needs `brew install
vhs`, Google Chrome and the same font) on the same demo data; rerun it when what it shows
changes, and a reviewer reports a stale recording as a should, like a stale screenshot.

Whenever behaviour changes, check that `README.md` is still true: a key, a default, a
subcommand, a config key. Do not add prose for a feature; if it needs explaining, the card or
the help view should carry it.

## Working agreements

All guidelines live in `AGENTS.md` files, versioned with the code (the root one for rules and
agreements, one per directory for that directory's facts); nothing binding lives only in an
agent's private memory. When the user says "update the guidelines", edit the `AGENTS.md`
nearest the code the guideline is about.

- **"Work autonomously" means the whole road to a release, without pausing**, not even for
  the retro: its outcome is written into the phase's plan file in step 4 and the user reads it there. When the user
  says it (or "autonomous", "march towards release"), assume this flow and ask only when a
  decision is genuinely theirs:
  1. Plan in `plans/phase-N-<topic>.md` (next free N in `PLAN.md`), in a fresh worktree, with
     its row in `PLAN.md` and in `plans/metrics.md`. When N is a multiple of 5, the phase opens
     with the sweep (see "The sweep"). The first ship also backfills the previous rows of
     `plans/metrics.md` (see "Time and tokens").
  2. `/my-tdd autonomous`: one task per red-green-refactor cycle, each shipped with
     `scripts/ship.sh` (gates, dev install, commit, push, draft PR on the first push).
  3. `/pr-review` the PR (see "Review before ready": what counts as must or should, when to
     redesign, where the reviewer runs the gates, who fixes). Fix every must and should
     finding, TDD like any change, ship once per round, and review again. Minor findings are
     fixed when cheap, else listed in the PR body. The loop ends only on a round with no must/should findings, and that
     round is **unprimed**: the reviewers get the PR, the repo and this file, never the list
     of earlier findings or what was decided about them. Never post the review on the PR.
  4. The plan file's `## Outcome` and `## Hurdles` are filled, its `plans/metrics.md` row
     holds what is known by now (see "Time and tokens"), and, when the branch feeds the binary,
     its fragment `changes/<branch>.md` is on it (no version bump, no `CHANGELOG.md` edit). Re-read the PR title and body, fix what
     went stale, `gh pr ready`, wait for CI green.
  5. Squash-merge, the only method `main` allows. Watch the `release` run the
     merge starts to green; with a fragment, check the release holds the six assets and the
     tap formula names the new version (see "Releasing"). Never tag by hand.
  6. Remove the worktree and report what shipped, with the release link and the times the
     branch could not record (see "Time and tokens").
  Stop and ask instead of proceeding when: a gate fails for a reason the change did not
  cause, CI fails twice on the same step, a review finding needs a product decision, or the
  next step would force-push, push to `main` or delete a release.
- **Cadence.** Outside the autonomous flow above, work runs under `/my-tdd autonomous`: one branch per phase (`phase-N-<topic>`),
  one commit per task through `scripts/ship.sh` (the fixes of a review round are the exception:
  one commit per round, see "Review before ready"), no pause between tasks, a pause only for the
  end-of-phase retro. Retro outcomes go to the plan file (`## Outcome` for facts and decisions,
  `## Hurdles` for what got in the way), its `plans/metrics.md` row, and here (rules).
- **Hurdles.** From phase 40, every plan file ends with `## Hurdles`: one bullet per thing that
  got in the way, `` - `<tag>` <what happened> → <where it went> ``. The tag is one of
  `review-loop`, `ci`, `tooling`, `test-seam`, `process`, `api-shape`, `design`; where it went
  is `code`, `backlog`, `known limit`, `none` or `` rule in `<path>` `` (a file that exists).
  A phase with nothing to say writes `- none recorded`. `tests/plans.rs` fails on any other shape.
- **The sweep.** A phase whose number is a multiple of 5 starts with a sweep before its own
  plan: read the `## Hurdles` of every phase since the last sweep and `plans/metrics.md`, and
  look for a tag that keeps coming back, a `→ none` that happened twice, and the review rounds
  and tokens drifting up. Each finding becomes a rule in the nearest `AGENTS.md`, a backlog
  item or an issue, or is declined with its reason; the sweep's outcome is the `### Sweep`
  section that opens that phase's `## Outcome` (from phase 60, `tests/plans.rs` fails without it).
  The retrospective checkpoints of `plans/original-plan.md` are its starting list.
  Its `### Sweep` opens with `Phases A to B`, the range it read; the next sweep reads from B + 1,
  and the sweep phase's own `## Hurdles` with it. A multiple of 5 that a sweep already read past
  gets no phase, and its `PLAN.md` row says which sweep covered it.
  `scripts/sweep-report.sh [<first phase>]` gathers the data (hurdles counted and grouped by tag,
  every `→ none`, the metrics rows and their empty cells). With no argument it starts at the
  phase after the last sweep phase's number, not at B + 1, so until #145 lands a sweep passes
  B + 1 as `<first phase>`;
  the `sweep` workflow opens an issue "Sweep due before phase N" with that report once phase N−1
  is on `main`. The report only counts: telling that two hurdles are the same problem, and what
  becomes of each, is the sweep's judgment, which the `sweep` skill (`/sweep`) walks through.
- **Time and tokens.** Every phase from 53 has one row in `plans/metrics.md`, written with the
  outcome, so it is on the branch before `gh pr ready`; its Notes cell carries what a number
  needs to be read right (a session shared between phases, a running total).
  The orchestrator (the main session) owns the numbers and hands them to the builder to write:
  the builder cannot see the reviewers' usage. It records only what is known by then:
  - Time: build start (the worktree's creation, the oldest entry of `git reflog show
    --date=iso <branch>`, its last line), draft PR opened, the end of the clean review
    round, the wall time from start to that end, and the number of review rounds. Absolute
    times with a timezone (`2026-11-17 04:56Z`), never "about an hour". Start is when the build
    begins, so the worktree is created then; when it already existed (a plan written ahead),
    Start is the orchestrator's time of the hand-off and the Notes say so. Every wait on the
    owner goes into the Notes with its span, from the question to the answer.
  - Tokens: one figure per agent (builder, each reviewer, the orchestrator), each marked
    measured or estimated with its source; what nobody can read is `unknown`, never a guess. A
    sub-agent's completion notice carries `subagent_tokens`, its running total so far (whether
    cached input counts like fresh input is not known), which the orchestrator copies from that
    agent's last notice; when the agent ran again after a hand-back (a builder fixing a round),
    the counter may have restarted, so the orchestrator copies the last notice of each run, in
    order, marks a restart it knows of, and never sums them or presents them as one running
    total. The Codex reviewer and the orchestrator's own session report none from the inside.
  - After the merge: a branch cannot record its own ready, merge or release, so its row leaves
    those cells empty, the report of step 6 gives them, and the first ship of the next phase
    writes them into every row still missing them. Both read them from GitHub (`gh api
    repos/<owner>/<repo>/issues/<N>/timeline --jq
    '.[]|select(.event=="ready_for_review").created_at'`, `gh pr view <N> --json mergedAt`,
    `gh release view <tag> --json publishedAt`), and the total from build start to release, or
    to merge with "no release" for a branch that feeds nothing into the binary.
- **One worktree per branch, by default.** Start every phase or fix in its own worktree off
  `origin/main`: `git worktree add -b <branch> .claude/worktrees/<branch> origin/main`, then
  `git branch --unset-upstream` so the first `ship.sh` push creates `origin/<branch>` instead of
  tracking `main`. Work, gates and ships run from inside it; the main checkout stays on its own
  branch, so two phases (and two agent sessions) never share an index or a `target/`.
  `.claude/worktrees/` is gitignored. Each worktree builds its own `target/` from cold the first
  time. `conveyor-dev` points at the worktree that last ran `dev-install.sh`, so re-run it in the
  worktree you are testing. Remove the worktree (`git worktree remove`) once its PR is merged.
  Agents that share a scratch directory put their branch in the name of every file they write
  there (a log, a PR body). Wait on your own ship by its process or its log, never by a
  `pgrep -f scripts/ship.sh`, which matches the ships of every other worktree too.
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
- **Screenshots in PRs.** A PR with a visible change embeds a screenshot that shows that
  change in its body, as a commit-pinned `raw.githubusercontent.com` URL. A change to the main
  interface goes into `docs/details.png` (see "Documentation rule"); for a view that one cannot
  show (another pane, a dialog, a narrow width, a key pressed), add a `shoot` line to
  `scripts/screenshots.sh` for it, commit the image under `docs/` and embed it too. Before
  `gh pr ready`, rerun `scripts/screenshots.sh` (and `scripts/demo.sh` when what the recording
  shows changed); when an image changed, commit it and pin its
  URL to that commit.
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
    finding, and ask the user when that changes behaviour.
  - **The gates run beside the branch, not in it.** The Claude reviewer runs
    `scripts/gates.sh` in its own detached worktree of the PR head (`git worktree add --detach
    <path> <sha>`, removed afterwards), never in the branch's worktree, so fixes do not wait
    for its run.
  - **Fixes go back to the builder.** The fixes of a round go to the agent that built the
    branch, with its context intact, not to a fresh one, and land as one `ship.sh` per round
    of findings rather than one per finding: the review-round exception to "one commit per
    task". Each fix that touches `src/` still starts from a failing test.
  - **Minor findings after the clean round.** They may be fixed without another round: one
    ship, each `src/` fix from a failing test, and the orchestrator checks the diff against the
    findings and says so in the metrics Notes; a fix that does more than its finding asks opens
    a new, unprimed round.
- **Merging and releasing.** Squash-merge the phase PR into `main` when CI is green, then
  watch the `release` run the merge starts (see Releasing): no bump, no `CHANGELOG.md` edit,
  no tag by hand. Squash is the only merge method the repository and the `main` ruleset allow
  (see `.github/AGENTS.md`). The squash commit on `main` takes GitHub's
  default here: the pull request title followed by `(#N)` (a pull request of a single commit
  gets that commit's subject instead), with the per-task commit messages listed in its body;
  the per-task commits stay readable on the pull request. Never push to `main` directly except the very first bootstrap commit of an empty
  repository, and never force-push.
- **Catching up with `main` on a pushed branch is a merge, not a rebase.** A rebase rewrites
  the pushed commits and the next `scripts/ship.sh` push is refused; the no-force-push rule
  then leaves only `git reset --hard origin/<branch>`, `git merge origin/main` and a
  cherry-pick of the new work. The squash into `main` leaves the merge commit behind anyway.
  Squash is the only merge method partly for this reason: a rebase-merge, which replays the
  branch's own commits and drops its merge commits, could land a tree the `version` check never
  judged.
- **Two binaries.** `conveyor` on PATH is the Homebrew release; `conveyor-dev` is a working
  tree (`scripts/dev-install.sh`). Never `cargo install` the crate into a PATH directory. A build from a working tree (`conveyor-dev`, CI) reads `v0.0.0` in the footer and in
  `--version`, and its `c` pane shows the changelog as frozen on `main`, not the newest
  release: that is expected, only a release commit carries a real version.
- **Plan before code, plans in the repo.** Each phase starts from a written plan, re-planned
  after the previous retro, with its "Verified facts" checked against live tools before use.
  Plans are versioned here: write each one to its own file under `plans/` (for example
  `plans/phase-5-cards.md`) and commit it with the first ship of the phase, with empty
  `## Outcome` and `## Hurdles` headings at its end that the phase fills before `gh pr ready`.
  Never leave a plan only in an agent's private plan directory or memory. `PLAN.md` stays the
  index of phases and the backlog; `tests/plans.rs` fails while a plan file is not linked from
  it, a phase plan has no `## Outcome`, or a phase from 53 has no `plans/metrics.md` row. A plan that adds
  logic over external state (pod states, check states, merge states, write results) carries a
  table of the states and what the card shows for each, and that table is reviewed before any
  code is written: review rounds otherwise find those states one at a time. A choice only the
  owner can make (a bound, how a state looks) is asked while planning, before the build starts,
  with each option checked against what the screen already draws: one asked during review stops
  the loop until it is answered.

## Development

Strict TDD: one failing test, minimal code, refactor. Outside-in: start from the binary
(`tests/cli.rs`) or from `run()` with scripted inputs on a TestBackend, drop to unit tests only
for pure helpers. No code comments; names and tests carry the intent.

Dead code is strictly forbidden. An item nothing calls is deleted in the same change that
orphans it, never kept "for later". A `pub` item that only tests call is a test seam and must
be on the `SEAMS` list in `tests/dead_code.rs`. `#[allow(dead_code)]` appears nowhere except
`tests/support/mod.rs`, where each test binary uses only some of the helpers. A `pub fn` of
`tests/support/mod.rs` that no test uses fails `tests/dead_code.rs` too, and so does a seam on
the `SEAMS` list that is no longer declared or no longer used by a test.

No Rust file under `src/` or `tests/` is longer than 700 lines (`tests/source_size.rs`). A test
file that outgrows the limit becomes a folder (`tests/<name>/main.rs` plus one file per topic),
so it stays one binary.

Gates, all required before a task is done, wrapped by `scripts/gates.sh`:

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -- --include-ignored   # one pass; the ignored ones are e2e: real binary, `gh` shim first on PATH
```

Without tmux the last gate is a plain `cargo test` and a notice that the e2e tests were skipped.

The gates and CI run the same Rust: `rust-toolchain.toml` pins it for every `cargo` that
rustup provides, and the workflows install the same version. A toolchain bump is one pull
request that changes the channel there, the `toolchain:` of `ci.yml` and `release.yml` (and, for new components, their `components:`),
`rust-version` in `Cargo.toml` (its `major.minor`), and the locked `rust-overlay` in
`flake.lock`, updated without Nix on the host by:

```bash
docker run --rm -v "$PWD":/src -w /src nixos/nix sh -c "git config --global --add safe.directory /src; nix --extra-experimental-features 'nix-command flakes' flake update rust-overlay"
```

`tests/toolchain.rs` fails while any of the first four differs, and the `version` check refuses
a channel change that keeps the locked `rust-overlay` revision, so the update must actually move
it: on a day the overlay has not moved since the lock, the check refuses the bump until it does
(the overlay commits about daily).
The `tag` job of `release.yml` stays on the runner image's `stable` on purpose (why: `.github/AGENTS.md`).
The Nix flake reads the same file: `fromRustupToolchainFile ./rust-toolchain.toml` from its
locked `rust-overlay`, so `flake.nix` names no version and a bump needs no hash, but the overlay
only knows the Rust releases up to its locked commit, hence the lock update in every bump.
`tests/toolchain.rs` fails while `flake.nix` stops reading the file or names any Rust version
itself, or the file loses `profile = "minimal"` (the default profile's `rust-docs` slows the
macOS `flake` job).

Outside CI (`CI` unset), `gates.sh` leaves out the slow `scripts` test target (`tests/scripts/`)
when `scripts/script-tests-skippable.sh` finds nothing between the merge base with
`origin/main` and the working tree, untracked files included, on the paths those tests read
from this repository (the list is in the script); it prints
`==> skipping the script tests: <reason>; CI runs them`. No repository, no `origin/main` or no
merge base runs everything. The skip is safe only because the `check` job of `ci.yml` calls
`cargo test --locked -- --include-ignored` itself, not `gates.sh`, and `check` is required to
merge into `main`. A script test that starts reading another path of this repository puts that
path on the list in the script. The script tests need `jq` on `PATH`, as `version-check.sh` and
`release-commit.sh` do (recent macOS ships it as `/usr/bin/jq`, the GitHub runners carry it).
How they stand in for `cargo` and `gh` is in `tests/AGENTS.md`.

Ship with `scripts/ship.sh "<message>"`: `cargo fmt`, gates, `dev-install.sh` (unless skipped, see Install), commit, push, all under
`set -e`, refusing to run on `main`, opening the draft PR on the first push of a branch (title: the first line of `<message>`; body: `.github/pull_request_template.md`, or empty when that file is missing).

- Do not hand-roll the chain: the interactive shell here is zsh, where `PIPESTATUS` is
  undefined and `test "" -eq 0` is true, so a `gates.sh | grep` guard passes a failure.
- Run the wrapper, not the commands, and never pipe `gates.sh` or `ship.sh` (through `tail`,
  `grep` or anything else): the pipe hides a failing gate. To keep the output, redirect it to a
  log file named for the branch. Chaining the gates with `&&` and then `;` before
  `git commit` lets a failure through the same way.
- Do not touch the worktree while a ship runs: `ship.sh` stages with `git add -A` after its
  gates, so an edit made meanwhile is committed without having passed them.

Install: `scripts/dev-install.sh` builds the release binary and symlinks it as
`~/.local/bin/conveyor-dev`; `conveyor` on PATH is always the Homebrew release. Never
`cargo install` the crate into a PATH dir. A milestone is done only after the gates pass
**and** `ship.sh` has either run `dev-install.sh` or said why it skipped it, so `conveyor-dev`
is the committed code of every branch that changed what the binary is built from.

`ship.sh` skips the install, printing `skipping dev-install: <reason>`, only when both hold
(`scripts/dev-install-skippable.sh`): `conveyor-dev` resolves to another worktree's build, and
nothing between the merge base with `origin/main` and the working tree, untracked files
included, feeds the binary. `scripts/feeds-binary.sh` is the one definition of "feeds the
binary": a path it does not list as outside (docs, tests, plans, scripts, CI, guidelines and the
like) feeds it, and a missing or dangling `conveyor-dev`, or no `origin/main`, installs too. So
a branch of docs, tests, scripts or CI never takes `conveyor-dev` over. Once `conveyor-dev` is
this worktree's build every ship runs `dev-install.sh`: cargo's freshness check makes it cheap
when nothing changed, and it rebuilds after a merge of `main` that no diff of the commit would
show. Embedding a file from a directory `scripts/feeds-binary.sh` lists into the binary means taking
that directory off the list in the script.

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
  (`scripts/feeds-binary.sh`, the same list `ship.sh` uses for the dev install). `flake.nix` and
  `flake.lock` feed the binary on purpose: they decide what a Nix user runs, and only a release
  moves `stable` to them. Such a pull
  request is always released, a refactor as an `internal` patch. One that touches only tests,
  docs, plans, scripts, CI, guidelines or agent skills (`.claude/`) needs none and releases nothing.
- **`changes/` is add-only.** A fragment is never edited, renamed or deleted once it is on
  `main`: "pending" means "not in the tree of the last tag", so a changed released fragment
  stops every release and a reused name can skip one silently. Put a correction in a new file.
- `tests/scripts/next_version.rs` runs `scripts/next-version.sh` on this repository, so a
  malformed committed fragment fails the gates (it reads `HEAD`, so locally only after the
  commit; CI always sees it). It needs the tags, like `every_released_tag_keeps_its_entry`.

**The `version` check enforces these rules on every pull request into `main`.** It is a required
check, so a pull request it refuses cannot merge. It refuses one that sets the version in
`Cargo.toml` or `Cargo.lock` or leaves either in a shape the release cannot rewrite, changes
`CHANGELOG.md`, edits, deletes or renames a fragment, adds a malformed or repeated one, changes
the Rust channel of `rust-toolchain.toml` without moving the locked `rust-overlay` of
`flake.lock`, or feeds the binary without one, naming the file and what to do instead. The full list, how it reads a
pull request without running it, the release run, and what to do when a run fails or a tag can
never build are under "Releasing internals" in [`.github/AGENTS.md`](.github/AGENTS.md).

**A revert keeps the fragment.** A pull request that reverts a merged one keeps that pull
request's fragment (GitHub's Revert button deletes it, which the check refuses, since
`changes/` is add-only) and adds a new fragment for the revert itself.

**After a merge**, watch the run (`gh run list --workflow release.yml --limit 3`, then
`gh run watch <id> --exit-status`). For a pull request with a fragment, check that the release
holds the six assets, that the tap formula names the new version, and that `Cargo.toml` on the
`stable` branch (what the Nix flake installs) names it too:

```bash
gh release view --json tagName,assets --jq '.tagName, .assets[].name'
gh api repos/piacsek/homebrew-tap/contents/Formula/conveyor.rb -H 'Accept: application/vnd.github.raw+json'
gh api 'repos/piacsek/conveyor/contents/Cargo.toml?ref=stable' -H 'Accept: application/vnd.github.raw+json'
```

Never push or move `stable` by hand: only the `stable` job of the release run moves it, after
`stable-build` built the flake at the tag.

A cancelled `release` run is normal (merges in quick succession queue up); a red one is not: see
"When a run fails or a job is stuck" under "Releasing internals" in `.github/AGENTS.md`.

Never push a `v*` tag by hand: tags come only from the `tag` job of the release run. Nothing
enforces that for the owner; why no tag ruleset does is in `.github/AGENTS.md`.

Never move or delete a tag that has a release: the next run would rebuild and republish it
and rewrite the tap formula for that old version. If a released version is wrong, release a
fix on top.

Semver: `feat` for a new column, key or config key (minor), `fix` for a fix (patch), `internal` for the rest (patch); no majors for now.
