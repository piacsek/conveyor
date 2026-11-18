# Phase 76: take a pull request out of the merge queue, or move it to the front, from the Merge queue column

## Context

The Merge queue column shows the entries of a repository's queue but offers no action on them:
`S` there has no menu. Leaving the queue exists only on a My PRs row (`e`), so a pull request
that is not the viewer's, or not in a My PRs tab, can only be removed on the web. Moving one to
the front is not offered anywhere. The owner asked for both on Merge queue rows.

Phase 75 is left for the sweep of phases 70 to 74, which needs phase 74 on `main` and the
owner's decisions; this phase takes 76.

## Verified facts (2026-11-18, at `f1440c8`)

- GitHub's GraphQL schema, read by introspection: `DequeuePullRequestInput` takes `id` ("The ID
  of the pull request to be dequeued"); `EnqueuePullRequestInput` takes `pullRequestId`, `jump`
  ("Add the pull request to the front of the queue") and `expectedHeadOid`. `MergeQueueEntry`
  has `position`, `state`, `jump`, `solo`, `enqueuer` and `pullRequest`. There is no mutation
  that takes a position: an entry can go to the end or to the front, nothing between.
- `gh` has no command for either; conveyor already sends `dequeuePullRequest` and
  `enqueuePullRequest` (with the head as `expectedHeadOid`) from My PRs as
  `StatusChange::Dequeue { node_id }` and `StatusChange::Enqueue { node_id, head_oid }`
  (`src/model/prs.rs`, `src/sources/github.rs`, `src/app/stages/prs.rs`).
- The queue query (`src/sources/queries/queue.graphql`) reads `pullRequest { number title url
  author { login } }`: no node `id` and no head oid, so a queue row cannot name its pull
  request to a mutation yet. `QueueEntry` (`src/model/queue.rs`) has `position`, `state`
  (`AwaitingChecks`, `Locked`, `Mergeable`, `Queued`, `Unmergeable`, `Unknown`) and `jump`. Its
  `head_sha` is the entry's `headCommit`, the commit the merge group tests, not the pull
  request's head, so the head a mutation pins is a new field read from `headRefOid`.
- `j` is a Normal-mode key (move down, `src/app/keymap.rs`), and `src/app/AGENTS.md` keeps every
  popup letter that writes to GitHub off the Normal-mode keys, so `j` is not free. The nearest
  free letter is `i`: `k` is a Normal-mode key too, `i` is neither a Normal-mode key nor an
  offer of any popup.
- `QueueStage` (`src/app/stages/queue.rs`) has tabs, one per `[[repo]]`, and inherits the
  default `StageView::menu`, which offers nothing. Main builds overrides it for `f`.
- Sensitive actions ask `y` in the confirm dialog and are re-checked by `reconfirm` when the
  row changed meanwhile; the rest act on their letter (`src/app/AGENTS.md`).
- **Jumping an entry already queued (task 4, read on 2026-11-18, no write sent):**
  - [Managing a merge queue](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue)
    offers the jump only "when adding a pull request to a merge queue", and says a jump "will
    cause a full rebuild of all in-progress pull requests". It says nothing of an entry already
    in the queue.
  - The [GraphQL mutations reference](https://docs.github.com/en/graphql/reference/pulls)
    describes `enqueuePullRequest` as "Add a pull request to the merge queue" and `jump` as "Add
    the pull request to the front of the queue"; it is silent on a pull request already queued.
  - Who may jump: the "Jump the merge queue" permission, held by admins or a custom repository
    role that grants it ([custom repository roles](https://docs.github.com/en/organizations/managing-user-access-to-your-organizations-repositories/managing-repository-roles/about-custom-repository-roles)).
  - `gh` has no enqueue or jump code (`pkg/cmd/pr/merge/http.go` enqueues through auto-merge only).
  - **Still unverified:** whether GitHub moves an entry already queued to the front or refuses
    it. Nothing read says it can never work, so the phase goes on under decision 3: a refusal
    becomes the card's failure mark with GitHub's reason and the queue is unchanged.

## Decided before code

The owner asked for this phase to run without waiting, so these follow the recommendation in
each case; the report of the phase lists them for the owner.

1. **Remove** is `e`, the letter My PRs uses for leaving the queue. On a Merge queue row it asks
   `y`: the pull request may be someone else's, and they lose their place.
2. **Move to the front** is `j` when the popup does not use `j` for anything; else the builder
   picks the nearest free letter and reports it. `j` is taken (see "Verified facts"): it is `i`.
   It asks `y`, and the dialog says that the merge groups behind it are rebuilt, so the checks
   of every entry behind restart.
3. **Jump is one mutation**, `enqueuePullRequest` with `jump: true` and the pull request's head
   as `expectedHeadOid`. If GitHub refuses it for an entry already queued, the card carries the
   usual failure mark with GitHub's reason and the queue is unchanged. A dequeue followed by an
   enqueue is not sent: a failure between the two would leave the pull request out of the queue.
   If the reading of task 4 shows GitHub refuses it for every queued entry, the builder stops
   and hands back instead of shipping an offer that can never work.
4. **No move up by one place and no reorder**: GitHub has no such operation.
5. **Permission follows GitHub**: the offer is not hidden by a guess about the viewer's role; a
   refusal becomes the failure mark, as approve does.

## Design

- **Query and model.** `queue.graphql` adds `pullRequest { id headRefOid }`; `QueueEntry` gains
  the node id and the pull request's head oid, parsed leniently (a missing one dims both offers
  with the reason, it never fails the column). No new call per refresh: two more fields.
- **Menu.** `QueueStage::menu` offers `e  remove from the merge queue` and `i  move to the
  front`. Both reuse `Request::ChangeStatus`: remove sends the existing `StatusChange::Dequeue`,
  jump sends `StatusChange::Enqueue` with a `jump` flag (one variant, not a second path), so
  the worker, the deadline, the spawn lock, `--demo` (every write fails) and the failure marks
  are the existing ones. The wording of `status.rs` and `writes.rs` covers the jump.
- **Tabs.** A write names the tab it was made in: the row's mark, the footer notice, the
  refresh after it and the follow-up land in that tab even after a switch (the rule the sweep
  of phase 70 added), with a test that switches tabs between the request and its landing.
- **Confirm.** Both ask `y`; `reconfirm` refuses when the entry left the queue, or the pull
  request's head changed, between the popup and the `y`.
- **After a success.** The queue of that tab and My PRs refresh, with the usual follow-up. A
  removed entry stays on screen until the refresh; the write in flight marks its card.
- **Docs.** `ui::help::KEYS` and the README key table if the popup's line changes; a `shoot` of
  the Merge queue popup in `scripts/screenshots.sh` committed under `docs/`; `src/app/AGENTS.md`
  (offers per stage, the sensitive actions), `src/sources/AGENTS.md` (the query) and
  `src/model/AGENTS.md` if parsing rules change. Fragment `changes/phase-76-queue-actions.md`,
  `kind: feat`.

### States

| State of the selected queue row | `e` remove | `i` move to the front |
| --- | --- | --- |
| queued, not first | offered, asks `y` | offered, asks `y` |
| first in the queue (position 1) | offered | dim: `already at the front` |
| `jump` already set on the entry | offered | dim: `already jumping the queue` |
| `Locked` (being merged) | dim: `being merged` | dim: `being merged` |
| `Unmergeable` | offered | offered; GitHub's answer decides |
| `Unknown` state | offered | offered |
| node id or head oid missing in the answer | dim: `GitHub sent no id for #N` | the same |
| a write already in flight on the row | dim, the wording of writes in flight | the same |
| the repository has no merge queue (the tab is struck) | no rows, so no popup | the same |
| nothing selected, or an empty queue | `S` says there is nothing to act on | the same |
| the entry left the queue between `S` and `y` | `reconfirm` refuses, nothing sent | the same |
| the pull request's head changed between `S` and `y` | `reconfirm` refuses | `reconfirm` refuses |
| the user switched tabs before the result | the result lands in the tab it was sent from | the same |
| GitHub refuses (no permission, already queued, gone) | failure mark with GitHub's reason | the same |
| the call times out | the mark says it may have happened | the same |
| `--demo` | the write fails with the demo's refusal, nothing spawned | the same |
| the same pull request on a My PRs row | `e` there is unchanged and still acts on the letter | no `i` there |

## Tasks

1. Query and model: the node id and head oid on a queue entry, lenient.
2. `e` on a Merge queue row: the offer, its refusals, the confirm, `reconfirm`, the write, the
   refresh and the tab it lands in.
3. The failure mark and the footer wording for a removal sent from the queue.
4. Read what GitHub does for a jump of a queued entry (documentation, the web client); record
   it under "Verified facts"; stop and hand back if it can never work.
5. `i` on a Merge queue row: the `jump` flag, the offer and its refusals, the confirm wording,
   the write.
6. Help, README, screenshot of the popup, `AGENTS.md` files, fragment.

## Outcome

## Hurdles
