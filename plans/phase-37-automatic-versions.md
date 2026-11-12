# Phase 37: the pipeline picks the version, a pull request cannot

## Context
Today the author of a pull request picks the version: the branch bumps `Cargo.toml`, adds the `CHANGELOG.md` entry, and after the merge someone runs `scripts/release.sh X.Y.Z` by hand. Pieces of work running in parallel do not know about each other, so they pick the same number and all edit the top of `CHANGELOG.md` (phase 28 could not be rebase-merged for that reason), and a release run that GitHub left stuck in `queued` blocked the local script on 2026-11-11.

The user wants pull requests to be version agnostic: a pull request says only what kind of change it is, and an automated workflow, the only thing able to set a version, decides the number and releases. Releases run one at a time through a queue.

## Verified facts (checked 2026-11-11)
- The repository belongs to a user account (`owner.type: User`), public. GitHub's merge queue is offered for organization repositories, so it is not part of this design. Not checked on the settings page.
- The `main` ruleset (id 26900132) requires a pull request, the checks `check` and `audit`, linear history, no force-push or deletion, and has **no bypass actor**. So nothing, a workflow included, can commit a version bump straight to `main`.
- `release.yml` starts on a pushed `v*` tag: `check-version` (tag equals `Cargo.toml`), `build` (macOS job for both Apple targets, Linux job), `publish` (`softprops/action-gh-release`, attestation), `tap` (`concurrency: tap`, pushes `Formula/conveyor.rb` with `TAP_TOKEN`).
- GitHub keeps one waiting run per concurrency group and replaces it when a newer one arrives (`.github/AGENTS.md`, the `tap` note). A queue built on it must release "everything unreleased", never "the commit that started me".
- A tag pushed with the workflow's own `GITHUB_TOKEN` starts no other workflow, so the build, publish and tap jobs have to run in the workflow that creates the tag.
- A `pull_request` workflow runs the pull request's own copy of the workflow file, so a pull request can weaken a check defined there. `pull_request_target` runs the copy on `main` (`delete-closed-branch.yml` already relies on this) and must never check out the pull request's code.
- The version is read from `CARGO_PKG_VERSION` in `src/main.rs` (`--version`, the log's first line) and `src/ui/footer.rs`. `src/changelog.rs` bundles `CHANGELOG.md` with `include_str!`; `tests/changelog.rs` requires its top entry to be the crate version and every `v*` tag up to it to keep its entry. Snapshots are already versionless.
- `scripts/homebrew-formula.sh <version>` builds the formula from the published assets and needs nothing from the working tree.
- Verified 2026-11-11 on a private throwaway repository: the workflow's own token pushed a tag for a release commit that is on no branch, `main` did not move, and the tag push started no other workflow run.
- Release commits are reachable only from their tags, so the workflow must fetch every `v*` tag explicitly. A checkout that only brings the tags reachable from `main` would see the same last tag forever.

Checked later (2026-11-12), for the optional hardening in task 8:
- Which bypass actors a **tag** ruleset accepts on a user-owned repository is still open: the private throwaway repository refuses every ruleset on a free plan (403), whatever the bypass actor. See Outcome.

## Design

### The version lives in tags, not on `main`
- `Cargo.toml` on `main` carries the constant `version = "0.0.0"` and never changes. A build from `main` or a branch reads `v0.0.0`.
- `CHANGELOG.md` on `main` is frozen at the last hand-made entry and is no longer edited by pull requests.
- The release workflow computes the next version from the newest `v*` tag, writes it into `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md` in a **release commit that is only reachable from the tag**, and builds from that commit. Nothing is written to `main`, so the ruleset keeps its "no bypass for anyone".
- Rejected: a bot commit on `main` (needs a bypass actor) and a bot-opened release pull request (needs auto-merge and a second token so CI runs on it). Both put the number back into a diff that a pull request could also write.

### A pull request declares its kind in a fragment
- A behaviour change adds one file `changes/<branch>.md`:

  ```markdown
  kind: fix

  - Main builds appears much sooner on the first load.
  ```

  `kind` is `fix` (patch), `feat` (minor) or `internal` (patch). The bullets are the changelog text, written for users as today. An `internal` fragment is for a change the user cannot observe, a refactor or a dependency bump, and may leave the bullets out; a release made only of such fragments gets the one line `- v<version>: internal changes only.`, naming the version because the changelog rules refuse a bullet repeated from an older release.
- **Majors are out of scope.** No kind produces one and the workflow never bumps the major number. A config-format break stays a decision made by hand, in a later phase.
- **A fragment is required exactly when the pull request touches what ends up in the binary**, and that decides whether anything is released:
  - Touches the binary: a fragment is mandatory, so the change is always released. A refactor is released as a patch (`internal`).
  - Touches only what does not feed the binary (`tests/`, `docs/`, `plans/`, `scripts/`, `.github/`, `changes/`, an `AGENTS.md` at any depth, `PLAN.md`, `README.md`, `LICENSE`, `.gitignore`): no fragment is needed and nothing is released.
  - "Feeds the binary" has one definition, the list `scripts/dev-install-skippable.sh` already holds. It moves into one script both that script and the new check call, so the two can never disagree.
- One file per branch, so parallel pull requests never conflict.

### The release queue
- `release.yml` starts on every push to `main` (and by hand through `workflow_dispatch`), under `concurrency: { group: release, cancel-in-progress: false }`.
- Each run:
  1. `last` = the highest `v*` tag.
  2. `pending` = the fragments in `changes/` at `HEAD` that are not in the tree of `last`.
  3. None: stop, green.
  4. `next` = `last` bumped by the highest `kind` among them.
  5. Release commit on top of `HEAD`: `Cargo.toml` and `Cargo.lock` at `next`; `CHANGELOG.md` = the new `## vNEXT (date)` section built from the pending fragments, above the `CHANGELOG.md` of `last`.
  6. Push the tag `vNEXT` at that commit, then build, publish and update the tap in the same run.
- Four merges in quick succession: the first run releases the first one; the runs behind it collapse into one that releases the other three as a single version.
- A run that died after pushing its tag is finished by the next run: when `last` has no release holding the six assets, that tag is built and published before anything new is computed.
- Fragments are never deleted by the workflow (it cannot write to `main`). "Pending" is defined against the last tag's tree, so old fragments cost nothing. **A fragment is never deleted, edited or renamed once it is on `main`:** `changes/` only grows. Deleting a released fragment and later reusing its name would either stop every release (the content differs from the released one) or skip a release without a word (the content is the same), so the check of task 7 refuses a pull request that deletes, edits or renames a file under `changes/`.

### A pull request cannot set the version
Two layers, the first being the one that matters:
1. **The workflow never reads a version from `main`.** The number comes from the newest tag and the fragment kinds. Whatever a pull request writes into `Cargo.toml` or `CHANGELOG.md` is overwritten in the release commit.
2. **A required check refuses the attempt.** A new job `version` on `pull_request_target` (the copy on `main`; as built it checks out `main`, fetches `refs/pull/<n>/head` as data and reads it with git, with no API call and nothing from the pull request executed) fails when the pull request changes the `version` of the package in `Cargo.toml`, the `conveyor` entry in `Cargo.lock`, or `CHANGELOG.md`; when a fragment it adds is malformed (a `kind` other than `fix`, `feat` or `internal`, or a `fix`/`feat` without a bullet); and when it touches a path that feeds the binary without adding a fragment. It may check out `main` to run the path script, never the pull request. It is added to the ruleset's required checks after it exists on `main`.

Known limit: the owner of the repository can still push a `v*` tag by hand or edit the ruleset. Task 8 would have narrowed the first; it stayed unproven (see Outcome).

## What changes for whoever works on a phase
- No version bump and no `CHANGELOG.md` edit in a pull request; a fragment instead.
- No `scripts/release.sh`. After the merge, watch the `release` run.
- `conveyor-dev` and CI builds read `v0.0.0`; the `c` pane of a dev build shows the changelog as frozen on `main`. Both are written down in the root `AGENTS.md` ("Two binaries" and "Releasing"), so a `v0.0.0` in the footer is read as "a build from a working tree", not as a bug.
- A refactor needs an `internal` fragment, or its pull request cannot merge.

## Tasks (each starts red)
1. `scripts/next-version.sh`, with `scripts/fragments.sh` (the last tag, the pending fragments, the fragment format): prints the next version from the newest tag and the pending fragments, nothing when none is pending. Tests in `tests/scripts/` on a scratch repository: patch, minor, the highest kind wins, fragments already in the last tag are ignored.
2. `scripts/release-commit.sh`, sourcing `scripts/fragments.sh`: writes `Cargo.toml`, `Cargo.lock` and the `CHANGELOG.md` section for a given version from the pending fragments. Tests: the section's shape passes `changelog::check`, bullets keep their order, an older changelog is kept below.
3. `changelog::check` and `tests/changelog.rs` accept the constant `0.0.0` on `main` (the top entry must equal the crate version only in a release commit) and gain a test that every fragment in `changes/` parses.
4. One script answers "does this path feed the binary"; `scripts/dev-install-skippable.sh` calls it and its tests stay green.
5. `Cargo.toml` goes to `0.0.0`; the first fragment-based release continues from the newest tag (see Rollout for v0.24.2).
6. `release.yml`: trigger on `main`, the concurrency group, the compute, release commit and tag steps, the existing build, publish and tap jobs fed from the release commit, the resume of an unfinished tag.
7. The `version` check on `pull_request_target` (the version is untouched, fragments are well formed, a binary change carries one), then the ruleset gains it as a required check (after the workflow is on `main`, or every merge blocks). A fragment must be an added file, never a modified or renamed one, and its name a direct child `changes/<name>.md` (a branch name holding `/` uses `-` instead). It refuses a pull request that deletes, edits or renames a file under `changes/`.
8. Optional hardening: a tag ruleset so `v*` tags are created only by the workflow. Only if the bypass actors available to a user-owned repository allow it.
9. Remove `scripts/release.sh` and its tests; rewrite "Releasing" (the kinds, no majors, `v0.0.0` in dev builds), "Two binaries", the autonomous flow (steps 4 and 5) and `.github/AGENTS.md`; `PLAN.md` outcome.

## Pull requests
One phase, four pull requests, so the only risky step is small and the rest can merge on their own:

1. **Groundwork** (tasks 1, 2 and 4): the scripts and their tests. Nothing calls them yet, so releases work as before.
2. **Cutover** (tasks 3, 5 and 6): `0.0.0` on `main`, the v0.24.2 entry as a fragment, the new `release.yml`, the guidelines for the new flow. Merging it makes the first automated release. Needs 1 and the proof on a throwaway repository.
3. **Enforcement** (task 7): the required check, then the ruleset. After one real release has worked.
4. **Cleanup** (tasks 8 and 9): the optional tag ruleset, the phase outcome.

The removal of `scripts/release.sh` and its tests moved from pull request 4 into pull request 2.

3 and 4 both need 2 and can run side by side. Between 2 and 3 a pull request can still edit the version; the workflow ignores it.

## Rollout
- Land tasks 1 to 4 first: they change nothing about how a release is made.
- **v0.24.2 becomes the first automated release.** `main` already carries `version = "0.24.2"` and its `CHANGELOG.md` entry (phase 35), but no tag: a release run GitHub left stuck in `queued` blocked `scripts/release.sh`. The pull request of task 5 takes that entry back out of `CHANGELOG.md`, sets the version to `0.0.0` and adds the same two bullets as a `fix` fragment, so the workflow computes v0.24.1 plus a patch and releases v0.24.2 with the text already written.
- Task 6 is proven on a throwaway repository before it replaces the tag trigger here, because a mistake publishes a release and rewrites the tap formula.
- The first automated release is a real one: a fragment for a small fix, merged, watched to the tap formula.

## Verification
`scripts/gates.sh`. A pull request that edits `version` in `Cargo.toml` shows a red `version` check and cannot merge. Two fragment pull requests merged within a minute produce two releases or one combined release, never a lost fragment, and the tap formula names the newest version.

## Decided by the user (2026-11-11)
- Dev builds read `v0.0.0`, and the guidelines say so.
- A pull request that touches the binary always carries a fragment and is always released; a refactor goes out as a patch. A pull request that touches nothing in the binary releases nothing.
- Majors are left out for now.

## Outcome (2026-11-12)
- **#63, groundwork** (tasks 1, 2, 4): `scripts/feeds-binary.sh`, `fragments.sh`, `next-version.sh` and `release-commit.sh` with their tests; `dev-install-skippable.sh` called `feeds-binary.sh` from then on, nothing called the release scripts yet. Four review rounds.
- **#64, cutover** (tasks 3, 5, 6, and the removal of `scripts/release.sh` from task 9): `main` at `0.0.0`, the v0.24.2 entry as a `fix` fragment, `release.yml` as the release queue on every push to `main` (`scripts/release-tag.sh`), drafts resumed, `build` and `publish` skipping a release that is already complete, a `tap` job that only moves the formula forward. Its merge made v0.24.2 the first automated release (run 43135383854: tag on a bot commit off `main`, six assets, attestation, tap formula), checked by downloading a tarball, its checksum and `gh attestation verify`.
- **#65, enforcement** (task 7, squash-merged): the `version` check on `pull_request_target` (`scripts/version-check.sh`), and `check` running clippy and the tests with `--locked`. Five review rounds. Then, by the owner on 2026-11-12, the `main` ruleset gained `version` as a required check, the up-to-date rule (`strict_required_status_checks_policy: true`) and squash as its only merge method; the repository settings allow squash merges only.
- **#66, cleanup** (tasks 8 and 9): this outcome, the `PLAN.md` entry, and the guidelines that still called the `version` requirement a step to come.

What changed against the plan:
- **The check asks cargo and rehearses the release** instead of reading `Cargo.toml` and `Cargo.lock` line by line. Every review round found another way to hide a version from a line reader (a decoy in a multi-line string or inline table, an escaped key, a `version = ` line above `[package]`), so cargo reads the head in an isolated copy, and the release rewrite is rehearsed there with a random probe version that all three readers must see. `Cargo.lock` must be in the shape cargo writes. Far larger than the plan's "fails when the pull request changes the version".
- **Squash only.** A review showed a rebase-merge replays a pull request's own commits and drops its merge commits, so it can land a tree the check never judged. The owner chose squash-only over teaching the check to replay a rebase-merge.
- **Branches must be up to date before merging**, so a pull request's green CI tests exactly what `main` becomes; every merge is released unattended.
- **`scripts/release.sh` went in #64**, not here, so pull request 4 shrank to documentation.
- **Task 8, the tag ruleset, stayed unproven.** On the private throwaway repository every `POST .../rulesets` answered 403 "Upgrade to GitHub Pro or make this repository public to enable this feature", with GitHub Actions (`Integration` 15368), the admin role, a deploy key or no bypass actor alike. Whether the job's `GITHUB_TOKEN` counts as a bypass actor needs a public repository or a paid plan to prove; nothing was applied to this repository. The owner can still push or delete a `v*` tag by hand.
- A merge queue was considered and left out: the repository is user-owned.
