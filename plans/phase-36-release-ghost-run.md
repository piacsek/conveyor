# Phase 36: a release run GitHub never started does not block the next release

## Context
`scripts/release.sh 0.24.2` exits 1 with `a release run is queued or in progress; wait for it`. The run it sees is a second `release` run for the already released tag `v0.24.1`: GitHub lists it as `queued`, never gave it a job, and refuses every way of removing it. It will stay in the list, so the guard would refuse every future release. The guard has to skip such a run and keep refusing while a real one is in flight.

## Verified facts (checked 2026-11-11, gh v2.103.3, read-only calls against this repository)
- `gh run list --workflow release.yml --json status,databaseId,createdAt` answers the stuck run as `"status":"queued"`, `"createdAt":"2026-11-11T20:46:25Z"`, with `updatedAt` equal to `createdAt` and an empty `conclusion`. `createdAt` is always UTC in the form `YYYY-MM-DDTHH:MM:SSZ`, so two of them compare as plain strings.
- `jobs` is a field of `gh run view --json` only, not of `gh run list --json`. `gh run view <id> --json jobs --jq '.jobs | length'` prints `0` for the stuck run.
- The run cannot be removed: `gh run cancel` answers `Cannot cancel a workflow run that is completed`, the force-cancel API answers 409 `has not been queued yet`, `gh run delete` answers HTTP 403 and the web UI says `Failed to cancel workflow`.
- `date -u -v-1H +%Y-%m-%dT%H:%M:%SZ` prints the time one hour ago on macOS (BSD); GNU `date` rejects `-v` and wants `-d '1 hour ago'`, which BSD rejects as an illegal option. The script tries the first and falls back to the second.
- `tests/scripts/release.rs` replaces `gh` with `tests/fixtures/reply-call.sh`, which prints every file in `$REPLIES` whose name is part of its argv and does not run `--jq`. A reply therefore holds what `gh` prints after its `--jq`, and the age has to be judged in the script, not inside `--jq`, to be under test.

## The rule
The guard lists every run that is not `completed` as `<id> <status> <createdAt>`. A run is ignored when all three hold: its status is `queued`, its `createdAt` is more than one hour ago, and `gh run view <id> --json jobs` counts no job. The script prints one line naming the run and why. Any other run that is not completed refuses with the message the script already has. A job count that cannot be read counts as a real run.

## Tasks (each starts red)
1. A queued run is skipped with a line that names it, and the release goes on.
2. A queued run that has a job still refuses.
3. A queued run created less than an hour ago still refuses, jobs or not.
4. Root `AGENTS.md` (Releasing, step 3) and the outcome in `PLAN.md`. No version bump and no `CHANGELOG.md` entry: scripts only.

## Verification
`scripts/gates.sh`. Not run for real here: the next `scripts/release.sh X.Y.Z` on `main` should print the ignored run and go on to tag.
