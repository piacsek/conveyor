# .github

The CI workflows and the branch protection around them, and the release internals (Releasing
internals, below). What a pull request author needs to release is in Releasing in the root
`AGENTS.md`.

## CI

`.github/workflows/ci.yml` runs on every push to `main` and every pull request: a `check` job
(fmt, clippy `-D warnings`, `cargo test -- --include-ignored`, both with `--locked`, with tmux installed for the e2e
suite) and an `audit` job (`cargo audit`). Learnings from the 2026-09-14 speed-up, when the
pipeline went from ~3 min 20 s to 37 s:

- **Measure before touching anything.** Per-step timings are one call away:
  `gh run view <run id> --json jobs --jq '.jobs[] | .name, (.steps[] | .name, .startedAt, .completedAt)'`.
  The whole pipeline was bound by one step: `rustsec/audit-check` ran `cargo install cargo-audit`
  from source on every run (about 3 min 10 s); everything else together was under 45 s.
- **Never `cargo install` a tool in CI.** Take the release binary instead:
  `taiki-e/install-action` with `tool: <name>` downloads the binary and checks it against the
  sha256 committed in the pinned action; `fallback: none` keeps it from quietly falling back to
  `cargo-binstall` and a source build when the download fails. `audit` is now 8 s. The same
  applies to any future tool (`cargo-insta`, `cargo-deny`).
- **Do not build what another workflow already builds.** `check` ran `cargo build --release`
  (14 s) although nothing consumed it: the e2e tests take the debug binary from
  `CARGO_BIN_EXE_conveyor`, and `release.yml` is what proves the release profile.
- **One test invocation.** `cargo test -- --include-ignored` runs the unit, TUI and e2e suites in
  one pass; two invocations rebuilt the test harness twice for nothing.
- **The floor is setup.** In the 33 s `check` job, about 16 s is job setup + checkout +
  toolchain + `Swatinem/rust-cache` restore and 6 s is `apt-get install tmux`; fmt + clippy +
  tests are about 8 s. Shaving further means caching the apt package or a container image, not touching
  cargo.
- **Trade-off taken.** `rustsec/audit-check` opened a GitHub issue when a new advisory hit
  `main`; with plain `cargo audit` a red `audit` check is the only signal. Add a `schedule`
  trigger if advisories should be caught between pushes.
- **Pins.** Every action stays pinned to a commit SHA with the tag in a trailing comment (see
  Releasing internals below for how to bump one).
- **`main` is protected by a ruleset** (`main`, id 26900132, Settings → Rules): a pull
  request, green `check`, `audit` and `version`, linear history, no force-push or deletion, and
  no bypass for anyone. The checks are matched by job name, so renaming `check` or `audit` in
  `ci.yml` blocks every merge, the rename's own PR included; renaming `version` lets the
  rename's own PR merge (the copy on `main` still reports the old name) and then blocks every
  later pull request until the ruleset names the new job; an advisory with no fixed version does the
  same through `audit`. Disable the ruleset to get out
  (`gh api -X PUT repos/<owner>/<repo>/rulesets/<id> -f enforcement=disabled`), fix, and enable
  it again. `version` (below) has been required since 2026-11-12, added by hand once
  `version.yml` was on `main`. A new required check follows the same order: a required check
  that never reports blocks every merge, and a `pull_request_target` workflow cannot report on
  the pull request that adds it.
  Since 2026-11-12 the ruleset also requires a branch to be up to date with `main` before it
  merges (`strict_required_status_checks_policy: true`): every merge to `main` is released
  unattended and the release run only builds, so a pull request's green CI must be a test of
  exactly what `main` becomes. A pull request behind `main` is updated with a merge of `main`
  (never a rebase of a pushed branch), and that push runs the checks, `version` included, again.
  Since 2026-11-12, too, squash is the only allowed merge method: the ruleset's
  `allowed_merge_methods` is `["squash"]` and the repository settings allow neither merge
  commits nor rebase merges. The `version` check judges the pull request's head tree against
  `main`, and a squash lands exactly that tree; a rebase-merge would replay the pull request's
  non-merge commits and drop its merge commits, so a rule broken in one commit and undone
  inside a merge of `main` could land although the check never saw that tree. If another merge
  method is ever allowed again, the check must first learn to judge what that method lands.
- **`version.yml` keeps the version out of pull requests.** One job, named `version`; the
  ruleset matches that name, so a rename blocks every pull request after the one that renames it, as above. It runs on
  `pull_request_target` (opened, synchronize, reopened, edited; base `main` only), not `pull_request`,
  because that event runs the workflow as it is on `main`: a pull request that edits
  `version.yml` or `scripts/version-check.sh` is still judged by the `main` copies, so it cannot
  weaken the check that judges it (proven: a pull request that bumped the version and rewrote
  the script to `exit 0` stayed red). The flip side: a change to the check is exercised only
  after it merges. The pull request that introduced it was proven on a private throwaway
  repository instead, and the same goes for any later change to it. The job checks out `main`
  (the event's default ref), fetches the `v*` tags and `refs/pull/<number>/head` (which exists
  in the base repository for a fork too), and runs `scripts/version-check.sh HEAD <head sha>`.
  The pull request is only data: the script reads it with `git diff` against the merge base
  and `git show`, never checks it out and never runs a file from it, so `.gitattributes`, hooks
  or scripts in the pull request do nothing. The one program it runs on the head is
  `cargo metadata --no-deps --offline --format-version 1`, which decides the package version,
  and it runs it on a copy: `git ls-tree -r -z <head>` lists the regular files of the head,
  each `Cargo.toml` is written with `git cat-file blob` and every other file as an empty
  placeholder (cargo only checks that a target file exists), into a fresh `mktemp -d`
  directory outside the checkout that is removed on exit. Symlinks, submodules, any path with
  a `.cargo` component and any `rust-toolchain` or `rust-toolchain.toml` (any case, any
  depth) are never written; `git archive` and `git checkout-index` are not used because they
  apply the head's `.gitattributes` (`export-subst`, filters). cargo runs from an empty
  directory of its own, so neither cargo nor rustup looks for configuration or a toolchain
  file in the copy, under `env -i` with only `PATH`, `HOME` and `CARGO_HOME` (both an empty
  directory of the scratch area), `RUSTUP_HOME` and `RUSTUP_TOOLCHAIN` (the caller's, else
  `stable`), so no wrapper, `RUSTFLAGS`, target directory or user configuration applies.
  `cargo metadata --no-deps` runs no build script and no proc macro and resolves nothing.
  It runs a second time on the same copy after the check has rehearsed the release there:
  when cargo reads `0.0.0`, the copy's `Cargo.toml` is rewritten with the functions
  `scripts/release-commit.sh` uses (`scripts/release-rewrite.sh`) to a probe version drawn
  from `/dev/urandom` on every run (a major of 100000 or more), and cargo, the `build` job's
  `grep -m1 '^version = '` line and the release script's own verification must all read the
  probe (the lock is rehearsed the same way, as text). So a manifest passes only in the plain
  form the line rewrite handles (`[package]` alone on its line, `version = "0.0.0"` as its own
  line under it, no other `version = ` line above it): a quoted key with a decoy line,
  `version="0.0.0"`, a comment after `[package]` or a `version = ` line in a table above it
  would otherwise pass and then push a tag that cannot build. The probe is not a fixed value
  because a fixed one can be planted: a `version = ` line holding it above `[package]` made
  the `build` line read the probe during the rehearsal and that planted value after the
  merge.
  `tests/scripts/version_check_head.rs` proves it: a head carrying a `build.rs`, a
  `.cargo/config.toml` naming a `rustc-wrapper`, `rustc`, aliases and `[env]`, or a toolchain
  file naming a toolchain that does not exist gets its normal verdict and nothing it names
  runs. The check reads cargo's JSON without `jq`: only output starting exactly as cargo writes
  it (`{"packages":[{"name":…,"version":…,"id":…`) is read, scanned string by string, and the
  root package is the one whose `manifest_path` is the copy's own `Cargo.toml`, compared as
  cargo writes it (the id percent-encodes the path, so a space in the temporary directory
  matched no package; a path holding `\` or `"`, which JSON escapes, still matches none and
  the check refuses). Cost, measured 2026-11-12: on
  the throwaway repository the check step took about 1 s and the whole job 6 s; on this
  repository the script takes about 1 s on a laptop.
  Workflow commands: the check step echoes `::stop-commands::<token>` before the script, with
  a token made in that step from `/dev/urandom`, folds the script's stderr into stdout, and
  echoes `::<token>::` after it, exiting with the script's code. Between the two the runner
  reads no workflow command, the legacy `##[warning]` form and one behind a carriage return
  included, so a path, a fragment line or a cargo error a pull request controls can only be
  printed. `tests/scripts/version_workflow.rs` pins that the token is made at run time and
  the script runs between the two lines. The file list comes from git, not the REST files
  API, which pages 100 at a time and stops at 3000 files; `git diff -M` also reports renames.
  The job holds `contents: read` only, no secret, and its one action is pinned. The checkout
  keeps no credentials (`persist-credentials: false`), so the job's token is not in
  `.git/config` while the script runs; only the fetch step gets it, as an `extraheader` in
  `GIT_CONFIG_*` variables of that one step, because a private repository (the throwaway one
  the check is proven on) refuses an unauthenticated fetch, and a public one takes the token
  as well. Draft pull requests are checked like any other. A pull request opened against
  another branch is not checked; retargeting it to `main` starts a run (`edited`), and so does
  an edit of its title or body, which only repeats the check. What the script refuses is in
  Releasing internals below; `tests/scripts/version_workflow.rs` pins the trigger, the
  job name, the token, where the token goes, the absence of a checkout of the pull request and
  the job's `timeout-minutes: 5`.
  Known and left: a fragment whose file name git would quote (one holding a double quote, for
  example) is refused as not a fragment, by design; and a force-push landing between the event
  and the fetch makes the superseded run fail with "is not a commit here", which the run of
  the newer push replaces. Also known and left: "feeds the binary" is judged by path only
  (`scripts/feeds-binary.sh`). Once a pull request, with its fragment, makes `src/` include or
  point at a path outside it (`include_str!`, `#[path]`, a symlink, `build =` in
  `Cargo.toml`), later edits to that path need no fragment and release nothing; and edits to
  `release.yml` or the release scripts never need one, although they change what a release
  builds. Also known and left: `cargo metadata` on the copy can be pointed at manifests
  outside it. `workspace.members` with an absolute or `..` path or a glob over `/`,
  `package.workspace` and path dependencies make cargo read files named `Cargo.toml` anywhere
  on the runner. Nothing is executed and only files named `Cargo.toml` are opened, but the
  refusal quotes cargo's first `error` line, which can reveal whether such a path exists on the
  runner, and a glob over the whole filesystem could stall the job: `timeout-minutes: 5` caps it.
- **`release.yml` is the release queue** (details in Releasing internals below). It
  starts on every push to `main` and on `workflow_dispatch`, never on a tag, under
  `concurrency: { group: release, cancel-in-progress: false }`: one run at a time, and GitHub
  keeps one waiting run per group, cancelling the older waiting one when a newer push arrives.
  That is safe only because a run never releases "the commit that started me": the `tag` job
  checks out `main` at its head and releases every fragment pending since the newest tag
  (`scripts/release-commit.sh` confirms the version it wrote with the `build` job's
  `grep -m1 '^version = '` line, and with `cargo metadata` when the runner image's cargo is on
  `PATH`, before the tag exists). The
  decisions live in `scripts/release-tag.sh` (resume an unfinished tag, else tag what is
  pending, else nothing) with its tests in `tests/scripts/release_tag.rs`; the YAML only wires
  jobs. The `tag` job is the only one besides `publish` with `contents: write`, which it uses to
  push one tag with the job's own token; a push made with that token starts no workflow, so
  the build and publish jobs run in the same run, from the tag. Both first run
  `scripts/release-unfinished.sh <tag>` and skip their other steps when the release is already
  complete (published, not a draft, six assets), so a rerun never replaces published assets.
  That check is a step, not a job, on purpose: `gh run rerun --failed` reruns the failed jobs
  and the ones that need them, never a job they need, so a separate gate job would keep its
  old answer. Nothing is ever pushed
  to `main` (the ruleset has no bypass actor, so nothing could be). Its decisions were proven
  end to end on a private throwaway repository before the cutover; there, `tap` is skipped
  (`if: github.repository == 'piacsek/conveyor'`) and so is the attestation (private
  repositories on a user account cannot attest). The attestation's subjects are the published tarballs, but
  the source commit it records is the `main` commit of the event that started the run, not
  the release commit the tag points at (what was built). Unless the run waited behind later
  merges, the two trees differ only by the version in `Cargo.toml` and `Cargo.lock` and by
  `CHANGELOG.md`.
- **The `tap` job of `release.yml` reconciles, it does not replay.** It runs in every release
  run that is not cancelled (`if: !cancelled() && github.repository == 'piacsek/conveyor'`),
  built or not, and `scripts/formula-version.sh` decides: the newest published release holding
  the six assets, written only when the formula names an older version. Concurrency alone
  could not keep the formula from going back (a rerun of an old `tap` after a newer release),
  nor repair a run cancelled between `publish` and `tap`. It still runs one at a time
  (`concurrency: tap`, `cancel-in-progress: false`), so two writes never race.
- **A pull request's branch is deleted when it merges or closes.** Merged: the repository
  setting `delete_branch_on_merge`. Closed without merging: GitHub has no setting, so
  `.github/workflows/delete-closed-branch.yml` runs on `pull_request_target: closed` and
  deletes the head branch with the job's own token (`contents: write` on that job only).
  `pull_request_target`, not `pull_request`: GitHub starts no `pull_request` workflow for a
  pull request that conflicts with its base, which is what an abandoned branch usually does.
  That event runs the workflow as it is on `main` with a write token, so a change to it is
  only exercised after it merges, and it must never check out or run the pull request's code
  (there is no checkout, so no action to pin). It leaves the branch alone when the pull
  request merged, comes from a fork, has the default branch as its head, still has another
  open pull request, or moved after the close (its tip is no longer the pull request's head
  sha). The delete is a `git push --force-with-lease=<ref>:<head sha>`, so a push landing
  between the read and the delete is refused by GitHub instead of lost; the REST delete takes
  no expected sha, and `gh api` does not escape a branch name in a path (`fix#1` read `fix`).
  A closed pull request is reopened with GitHub's "Restore branch" button first. The job is
  skipped or green for everything it leaves alone; red means a call failed or the branch
  moved or vanished in the second between the read and the delete, and nothing is lost
  either way. The open pull request check is a snapshot: closing and reopening one to
  retrigger CI loses the branch when the reopen comes after the job has looked (seconds),
  so push an empty commit or rerun the checks instead. None of the three required checks depends on it. Known and left: an open pull request whose **base** is the deleted
  branch is closed by GitHub (branches here are cut from `origin/main`, never stacked).
- **A run of the old tag-triggered workflow for v0.24.1 is stuck in `queued` and cannot be
  removed** (run 43098019853, 2026-11-11: no jobs, cancel answers "completed", force-cancel
  "not queued yet", delete 403, the web UI "Failed to cancel workflow"). It holds neither the
  `release` nor the `tap` group, so it does not block the release runs. Should GitHub ever
  start it, it runs the workflow file of that tag: it rebuilds v0.24.1, overwrites its
  tarballs and writes the formula back to v0.24.1. The way back is a release run
  (`gh workflow run release.yml`), whose `tap` job moves the formula forward again; the
  v0.24.1 tarballs then carry new checksums and stay that way.
- **No tag ruleset guards `v*` tags; the owner can still push or delete one by hand.** Whether
  a tag ruleset can let only the release workflow's `GITHUB_TOKEN` create `v*` tags is
  unproven. Tried 2026-11-12 on the private throwaway repository (user-owned, free plan):
  `POST repos/<owner>/<repo>/rulesets` with `target: tag`, `refs/tags/v*`, the rules
  `creation`, `update` and `deletion`, and as bypass actor GitHub Actions
  (`actor_type: Integration`, `actor_id: 15368`), the admin role (`RepositoryRole`, 5), a
  `DeployKey`, or none: every attempt answered 403 "Upgrade to GitHub Pro or make this
  repository public to enable this feature", and so does listing the rulesets. The legacy
  `tags/protection` endpoint answers 404. GitHub's ruleset docs list GitHub Apps as bypass
  actors without saying whether the job token counts as the GitHub Actions app, so the
  proof needs a public repository or a paid plan, not this repository: a wrong guess makes
  every release run fail at the tag push. A `deletion` rule would also stop the recovery in
  Releasing internals ("When the newest tag can never build") until the ruleset is disabled.
- **`check` runs clippy and the tests with `--locked`.** Without it cargo quietly updates a
  stale `Cargo.lock` on the runner, so a pull request that changed `Cargo.toml` dependencies
  and forgot the lock was green everywhere, and the first thing to refuse it was
  `cargo build --release --locked` in the release run, after the tag was pushed. The local
  gates (`scripts/gates.sh`) do not pass `--locked`: there cargo updating the lock is how the
  lock gets written, and the change shows up in `git status`.
- **Known and left: `check` builds and tests on Linux only.** A change that breaks only the
  macOS build passes every required check, merges, and fails in the release run's `build`
  job after its tag is pushed; that is the "newest tag can never build" case in Releasing
  internals below.

## Releasing internals

How the `version` check judges a pull request and how a release run tags, builds, publishes and
recovers. What a pull request author needs (the fragment, its `kind`, the tag rules) stays in
Releasing in the root `AGENTS.md`.

**The `version` check enforces the fragment rules of Releasing in the root `AGENTS.md` on
every pull request into `main`** (`.github/workflows/version.yml`, details in the
`version.yml` bullet under CI). `scripts/version-check.sh <base> <head>` refuses the pull
request, naming the file and what to do instead, when:

- a path it changes holds a control character (a newline, a tab, any byte below 0x20, or
  0x7f): the check stops there, before anything else, and prints only how many such paths
  there are, since a newline would split one path into two and could inject a workflow
  command through its output;
- cargo, asked about the `Cargo.toml` at its head, reads a version of the root package other
  than `0.0.0`, cannot load the manifest at all (the refusal quotes cargo's first `error` line),
  finds no package at the root (a virtual manifest) or one not named `conveyor`; a workspace
  member keeps its own version. The check never reads the version itself: a decoy line in a
  multi-line string or inline table, or a key spelled with an escape, fooled every line reader
  tried, while cargo is what decides. How the head is kept from running anything is in
  the `version.yml` bullet under CI. Without `cargo` on `PATH` the check fails rather than passes;
- cargo reads `0.0.0`, but the release could not set the version: the check rehearses the
  release on its copy of the head, applying to `Cargo.toml` and `Cargo.lock` the very rewrite
  `scripts/release-commit.sh` applies (the functions of `scripts/release-rewrite.sh`, shared by
  both) with a probe version drawn from `/dev/urandom` on every run (a major of 100000 or more,
  which no release carries), and refuses unless the release script's own
  verification passes, cargo (isolated as before) reads the probe, the line the `build` job of
  `release.yml` compares with the tag (`grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2`,
  the same spelling there, here and in `release-commit.sh`, pinned by
  `tests/scripts/version_check_release.rs`) reads
  it, and the lock's `conveyor` entry reads it by the lock rules below. The rewrite works line
  by line, so `Cargo.toml` keeps one plain form: `[package]` alone on its line (no comment, no
  spaces), `version = "0.0.0"` as its own line under it (not `version="0.0.0"`, not a quoted
  key, not `version.workspace = true`), LF line endings, and no other line starting with
  `version = ` above it. Anything else passed cargo yet left the release commit with the new
  version in a decoy line or nowhere: the run would then push its tag and fail to build it,
  and a released tag never moves, or `release-commit.sh` would refuse on every run;
- `Cargo.lock` holds a `conveyor` entry other than `0.0.0`, none, or more than one entry of
  that name (the check reads the lock line by line, so anything that could hide the real line
  is refused);
- `Cargo.lock` at its head is not in the one shape cargo writes it in: it holds a multi-line
  string, a line whose key is `name` or quoted is not exactly `name = "<crate>"` (letters,
  digits, `_` and `-`), or a `name = "conveyor"` line is not right after a `[[package]]` line
  and right before its `version = "..."` line. A lock no tool would write could hide the real
  entry behind a decoy the check reads, pass, and then fail the release's
  `cargo build --locked`; the refusal says to regenerate it with cargo
  (`cargo update --workspace`);
- it changes `CHANGELOG.md` at all;
- it edits, deletes or renames a file under `changes/`, or renames a file into it;
- a fragment it adds is not a direct child `changes/<name>.md`, its name is already on the tip
  of `main` (a branch behind `main` re-adding a fragment `main` gained later; it does not count
  as the pull request's fragment), or `fragment_kind` or `refuse_repeats` of
  `scripts/fragments.sh` refuses it (the same functions the release run uses, read at the pull
  request's head through `fragments_at`; `fragment_kind` refuses a carriage return anywhere
  but at the very end of a line, by name, in the release run too), or it repeats a bullet of
  a fragment `main` holds that the last tag does not (compared as `refuse_repeats` compares,
  naming both files: once the two land in different releases the later release run would
  refuse on every run, and with `changes/` add-only nobody could repair it);
- a path it changes feeds the binary (`scripts/feeds-binary.sh`, a rename counting both of its
  paths, `CHANGELOG.md` left out since it is refused anyway) and it adds no fragment.

Past the control character check it reports every refusal, not only the first. Every line it
writes to stderr starts with `version check: `, the lines of the `scripts/fragments.sh`
functions included (with a carriage return turned into a space), so a path or a fragment line
starting with `::` is never the first thing on a line; the release scripts calling those
functions print them as before. The prefix alone does not keep a pull request from issuing a
workflow command: the runner also reads the legacy `##[warning]...` form anywhere in a line.
What does is the workflow: it stops command processing with `::stop-commands::<token>` before
the script runs, a token made in that step from `/dev/urandom` that the pull request cannot
know, folds the script's stderr into stdout so nothing overtakes that line, and turns
processing back on with `::<token>::` afterwards, keeping the script's exit code. The check
judges the head's tree, which is exactly what a squash lands, so squash stays the only merge
method `main` allows (see the ruleset bullet under CI) until the check learns to judge what another
method lands. `version` is a required check of the `main` ruleset since 2026-11-12, next to
`check` and `audit`, so a pull request it refuses cannot merge.

**The release run.** `.github/workflows/release.yml` starts on every push to `main` (and by
hand: `gh workflow run release.yml`) and runs one at a time (`concurrency: release`). Its `tag`
job checks out `main` at its current head, not the commit that started it, fetches every `v*`
tag (release commits are reachable only from their tags) and runs `scripts/release-tag.sh`:

1. The newest `vX.Y.Z` tag, when its release is not `complete` (published, not a draft, holding
   exactly the three tarballs and their `.sha256`; `scripts/releases.sh`), is unfinished (only
   the newest tag is looked at, an older unfinished one below it is never resumed): the
   run builds and publishes that tag and computes nothing new. A draft counts as unfinished:
   `softprops/action-gh-release` creates the release as a draft, uploads, then publishes, so a
   run that dies in between leaves a draft that may already hold every asset. A release
   holding any other asset stops the run; remove that asset by hand. Fragments merged since
   that tag wait for the next run, and the run says so in a `::warning::`.
2. Otherwise `scripts/next-version.sh` reads the fragments added since that tag; none means
   nothing to release: `build` and `publish` are skipped, `tap` still runs and only reconciles
   the formula, and the run is green.
3. Otherwise `scripts/release-commit.sh` writes the version into `Cargo.toml` and
   `Cargo.lock` and a `CHANGELOG.md` with the new section above the last tag's. Before the
   changelog it reads `Cargo.toml` with the `build` job's own line
   (`grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2`) and asks
   `cargo metadata --no-deps --offline` whether the package version now reads the release;
   on any other answer it puts `Cargo.toml` and `Cargo.lock` back and refuses, so no tag is
   pushed. The `tag` job installs no toolchain: it relies on the cargo that GitHub's
   `ubuntu-latest` image carries, which the image lists but nothing pins, so the cargo
   confirmation is skipped silently when cargo is not on `PATH` (the `build` line is read
   either way) and the rehearsal in the `version` check is then the only guard on what cargo
   reads. The job commits the release
   as `github-actions[bot]` (`Release vX.Y.Z`) on top of `main` and pushes only the tag. `main`
   never moves; the tag push starts no other run.

Then `build` and `publish` each start with `scripts/release-unfinished.sh <tag>` and skip
every other step when the tag's release is already `complete`, so no run, a rerun of an old
one included, rebuilds a published version or replaces its assets (their checksums never
change once published). Otherwise `build` (one `macos-latest` job for both Apple targets, Intel cross-compiled, and one
Linux job; `macos-13` is retired and queues forever) checks out the tag, refuses it when
`Cargo.toml` there reads another version, and packages one flat tarball per target
(`conveyor-<target>.tar.gz` holding just the binary) plus `.sha256`; `publish` attests them
(`gh attestation verify <tarball> --repo piacsek/conveyor`) and creates the release with
generated notes. **What the attestation proves:** its subjects are the digests of the
published tarballs, but the source commit it records is the commit of the event that started
the run (a commit on `main`), not the release commit the tag points at, which is what was
built. When the run was not queued behind later merges, the two trees differ only by the
version in `Cargo.toml` and `Cargo.lock` and by `CHANGELOG.md`; a run that waited in the queue
or resumed an older tag records a `main` commit that can be further from what was built.
`tap` runs in every release run that is not cancelled, whether or not this run built anything:
`scripts/formula-version.sh` picks the newest published `complete` release, and only when
`Formula/conveyor.rb` in `github.com/piacsek/homebrew-tap` (local clone
`~/projects/homebrew-tap`) names an older version (or the file is missing) does the job
regenerate it with `scripts/homebrew-formula.sh <version>` and push it with the `TAP_TOKEN`
repo secret (fine-grained PAT, contents:write on homebrew-tap; set with
`gh secret set TAP_TOKEN --repo piacsek/conveyor`). `tap` runs only in `piacsek/conveyor` and
the attestation only in a public repository, so the same file can be proven in a private
throwaway one. The formula never goes backwards: a run cancelled between `publish` and `tap`
is repaired by the next run, and an old `tap` job run again finds a formula already at or past
its release and writes nothing. Asset names carry no version so `releases/latest/download/` links stay valid.

Merges in quick succession queue up: GitHub keeps one waiting run per group and cancels the
older waiting one, which is harmless because every run releases everything pending at the
head of `main`. A cancelled `release` run is therefore normal; a red one is not.

**When a run fails or a job is stuck** (a macOS job left in `queued`), cancel it if it is
still going and **start a new run** (`gh workflow run release.yml`) rather than rerunning the
old one. A new run reads the head of `main` with the current workflow file and, when the
newest tag is unfinished (no release, a draft, or an asset missing), finishes it and
releases nothing new in that run, so fragments merged meanwhile wait for the run after it
(dispatch one more if nothing else is coming). A rerun (`gh run rerun <id> --failed`) still
works but replays the old run: the workflow file of its commit, and the tag its `tag` job chose
then. Its `build` and `publish` check that tag again: they finish it when its release is still
unfinished and do nothing when it is already `complete`, and its `tap` only moves the formula
forward, so rerunning an old run never rewrites a published release or the formula backwards. If `tap` fails, the next release run repairs the formula; to repair it
now, start a new run, or run `scripts/homebrew-formula.sh <version>` by hand and push the tap.
Check with `brew audit --strict --online --formula piacsek/tap/conveyor` after changing the
script.

**When the newest tag can never build** (its commit does not compile, or its `Cargo.lock` is
stale, so every run resumes it and fails again): the tag has no published release, so the
rule about released tags (Releasing in the root `AGENTS.md`) does not cover it. Land the fix
on `main` through a pull request, delete the unfinished tag and its draft release if one
exists, then start a new run, which tags the same version again from the fixed `main`. Deleting a tag is the owner's decision:
an agent asks first.

```bash
gh release delete vX.Y.Z --yes
git push origin :refs/tags/vX.Y.Z
gh workflow run release.yml
```

The first command only applies when a draft exists.

Every action is pinned to a commit SHA with the tag in a trailing comment; bump pins with
`gh api repos/<owner>/<repo>/git/ref/tags/<tag>`. Workflows default to `contents: read`; only
the `tag` job (pushes the tag) and `publish` (`contents: write`, `id-token` and
`attestations: write` for `actions/attest-build-provenance`) get more. CI also runs
`cargo audit` (a prebuilt binary via `taiki-e/install-action`; the `rustsec/audit-check` action
compiled cargo-audit from source on every run, ~3 min of a ~3.5 min pipeline). The `check` job
runs `cargo test -- --include-ignored` in one pass; the e2e tests take the debug binary from
`CARGO_BIN_EXE_conveyor`, so no release build there.

Installed copies: Homebrew puts the release binary in `$(brew --prefix)/bin/conveyor`;
dev builds are only reachable as `conveyor-dev`. `rustsec/audit-check` v2.0.0 (latest as of
2026-09-10) still targets Node 20 and CI prints a deprecation annotation; bump the pin when a
newer release exists.
