# .github

The CI workflows and the branch protection around them. Releasing is in the root `AGENTS.md`.

## CI

`.github/workflows/ci.yml` runs on every push to `main` and every pull request: a `check` job
(fmt, clippy `-D warnings`, `cargo test -- --include-ignored` with tmux installed for the e2e
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
  Releasing in the root `AGENTS.md` for how to bump one).
- **`main` is protected by a ruleset** (`main`, Settings → Rules): a pull request, green `check`
  and `audit`, linear history, no force-push or deletion, and no bypass for anyone. The checks
  are matched by job name, so renaming either job in `ci.yml` blocks every merge, the rename's
  own PR included; an advisory with no fixed version does the same through `audit`. Disable the
  ruleset to get out (`gh api -X PUT repos/<owner>/<repo>/rulesets/<id> -f enforcement=disabled`),
  fix, and enable it again. `version` (below) joins the required checks by hand, in the
  ruleset, only after `version.yml` is on `main`: a required check that never reports blocks
  every merge, and `pull_request_target` cannot report on the pull request that adds it.
  Since 2026-11-12 the ruleset also requires a branch to be up to date with `main` before it
  merges (`strict_required_status_checks_policy: true`): every merge to `main` is released
  unattended and the release run only builds, so a pull request's green CI must be a test of
  exactly what `main` becomes. A pull request behind `main` is updated with a merge of `main`
  (never a rebase of a pushed branch), and that push runs the checks, `version` included, again.
- **`version.yml` keeps the version out of pull requests.** One job, named `version`; the
  ruleset matches that name, so renaming the job blocks every merge as above. It runs on
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
  `scripts/release-commit.sh` uses (`scripts/release-rewrite.sh`) to the probe version
  `987.654.321`, and cargo, the `build` job's `grep -m1 '^version = '` line and the release
  script's own verification must all read the probe (the lock is rehearsed the same way, as
  text). So a manifest passes only in the plain form the line rewrite handles (`[package]`
  alone on its line, `version = "0.0.0"` as its own line under it, no other `version = ` line
  above it): a quoted key with a decoy line, `version="0.0.0"`, a comment after `[package]`
  or a `version = ` line in a table above it would otherwise pass and then push a tag that
  cannot build.
  `tests/scripts/version_check_head.rs` proves it: a head carrying a `build.rs`, a
  `.cargo/config.toml` naming a `rustc-wrapper`, `rustc`, aliases and `[env]`, or a toolchain
  file naming a toolchain that does not exist gets its normal verdict and nothing it names
  runs. The check reads cargo's JSON without `jq`: only output starting exactly as cargo writes
  it (`{"packages":[{"name":…,"version":…,"id":…`) is read, scanned string by string, and the
  root package is the one whose id is the copy's own directory. Cost, measured 2026-11-12: on
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
  Releasing in the root `AGENTS.md`; `tests/scripts/version_workflow.rs` pins the trigger, the
  job name, the token, where the token goes and the absence of a checkout of the pull request.
  Known and left: a fragment whose file name git would quote (one holding a double quote, for
  example) is refused as not a fragment, by design; and a force-push landing between the event
  and the fetch makes the superseded run fail with "is not a commit here", which the run of
  the newer push replaces. Also known and left: "feeds the binary" is judged by path only
  (`scripts/feeds-binary.sh`). Once a pull request, with its fragment, makes `src/` include or
  point at a path outside it (`include_str!`, `#[path]`, a symlink, `build =` in
  `Cargo.toml`), later edits to that path need no fragment and release nothing; and edits to
  `release.yml` or the release scripts never need one, although they change what a release
  builds.
- **`release.yml` is the release queue** (details in Releasing in the root `AGENTS.md`). It
  starts on every push to `main` and on `workflow_dispatch`, never on a tag, under
  `concurrency: { group: release, cancel-in-progress: false }`: one run at a time, and GitHub
  keeps one waiting run per group, cancelling the older waiting one when a newer push arrives.
  That is safe only because a run never releases "the commit that started me": the `tag` job
  checks out `main` at its head and releases every fragment pending since the newest tag
  (`scripts/release-commit.sh` confirms the version it wrote with `cargo metadata` before the
  tag exists, when the runner image's cargo is on `PATH`). The
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
  so push an empty commit or rerun the checks instead. Neither required check depends on it. Known and left: an open pull request whose **base** is the deleted
  branch is closed by GitHub (branches here are cut from `origin/main`, never stacked).
- **A run of the old tag-triggered workflow for v0.24.1 is stuck in `queued` and cannot be
  removed** (run 43098019853, 2026-11-11: no jobs, cancel answers "completed", force-cancel
  "not queued yet", delete 403, the web UI "Failed to cancel workflow"). It holds neither the
  `release` nor the `tap` group, so it does not block the release runs. Should GitHub ever
  start it, it runs the workflow file of that tag: it rebuilds v0.24.1, overwrites its
  tarballs and writes the formula back to v0.24.1. The way back is a release run
  (`gh workflow run release.yml`), whose `tap` job moves the formula forward again; the
  v0.24.1 tarballs then carry new checksums and stay that way.
