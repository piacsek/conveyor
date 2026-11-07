# Phase 29: `m` adds to the merge queue where auto-merge is off

## Context
On a repository with a merge queue and "Allow auto-merge" off, `S` then `m` (or `a`) failed with
`GraphQL: Auto merge is not allowed for this repository (enablePullRequestAutoMerge)`, while
GitHub's own "Merge when ready" button worked.

## Verified facts (2026-11-06)
- The repository: `autoMergeAllowed: false`; the pull request: `isMergeQueueEnabled: true`,
  `viewerCanEnableAutoMerge: false`, `mergeStateStatus: CLEAN`.
- gh v2.103.3 `pkg/cmd/pr/merge/merge.go`: when the base needs a merge queue, `shouldAddToMergeQueue`
  sets `AutoMergeEnable`, and `http.go` sends `enablePullRequestAutoMerge`. gh never sends
  `enqueuePullRequest`, so `gh pr merge` fails on such a repository like `a` does.
- GitHub's "Merge when ready" sends `enqueuePullRequest(input: {pullRequestId, expectedHeadOid})`,
  which needs no auto-merge setting. A pull request whose checks still run joins as `AWAITING_CHECKS`.
- `PullRequest.isMergeQueueEnabled`: whether the base ref has a merge queue.

## Decision
On a pull request whose base has a merge queue, `m` reads `add to merge queue` and sends
`enqueuePullRequest` with the head as `expectedHeadOid`, after `y`. Elsewhere `m` keeps
`gh pr merge`. `a` keeps following GitHub's `viewerCan*AutoMerge` flags.

## Tasks (each starts red)
1. `isMergeQueueEnabled` in `prs.graphql`, parsed to `PullRequest::merge_queue_enabled`. `tests/parse_prs.rs`.
2. `StatusChange::Enqueue { node_id, head_oid }` and its `CliGh::status_args` (the
   `enqueuePullRequest` mutation). `tests/github.rs`.
3. The `m` offer on a merge-queue pull request: label, confirm question
   `add #N <title> to the merge queue?`, `y` re-checks and sends `Enqueue`, success sets
   `merge_landing` and refreshes My PRs and the queue. Drafts and queued pull requests stay refused.
   `tests/manage.rs`.
4. AGENTS.md (`S`/`m` bullets), `PLAN.md` outcome, v0.21.2 with its `CHANGELOG.md` entry.

## Verification
`scripts/gates.sh`; `conveyor-dev`, `S` on a merge-queue pull request reads `m  add to merge queue`.
