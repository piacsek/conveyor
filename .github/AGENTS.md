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
  fix, and enable it again.
- **`release.yml` is the release queue** (details in Releasing in the root `AGENTS.md`). It
  starts on every push to `main` and on `workflow_dispatch`, never on a tag, under
  `concurrency: { group: release, cancel-in-progress: false }`: one run at a time, and GitHub
  keeps one waiting run per group, cancelling the older waiting one when a newer push arrives.
  That is safe only because a run never releases "the commit that started me": the `tag` job
  checks out `main` at its head and releases every fragment pending since the newest tag. The
  decisions live in `scripts/release-tag.sh` (resume an unfinished tag, else tag what is
  pending, else nothing) with its tests in `tests/scripts/release_tag.rs`; the YAML only wires
  jobs. The `tag` job is the only one besides `publish` with `contents: write`, which it uses to
  push one tag with the job's own token; a push made with that token starts no workflow, so
  the build, publish and tap jobs run in the same run, from the tag. Nothing is ever pushed
  to `main` (the ruleset has no bypass actor, so nothing could be). Its decisions were proven
  end to end on a private throwaway repository before the cutover; there, `tap` is skipped
  (`if: github.repository == 'piacsek/conveyor'`) and so is the attestation (private
  repositories on a user account cannot attest).
- **The `tap` job of `release.yml` runs one at a time** (`concurrency: tap`,
  `cancel-in-progress: false`), so two releases never write the formula at the same moment.
  With the whole workflow in the `release` group two runs no longer overlap, so the formula is
  written in release order.
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
