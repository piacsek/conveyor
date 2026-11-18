# Phase 72: approve a pull request without leaving conveyor

## Context

Issue #113. Reviewing someone else's pull request ends in the browser today only to press
"Approve". The `S` popup on a My PRs row gains approve under the letter `v` (the owner's choice;
`v` is no Normal-mode key). It asks to confirm with `y`, like merge, and runs
`gh pr review <N> -R <owner>/<repo> --approve`. The card shows the review state after the next
refresh. On the viewer's own pull request it is offered dim, with the reason. A failed approve
leaves the usual failure mark.

## Verified facts (2026-11-18, `main` at 5cae050, gh 2.104.0)

- `v` is no Normal-mode key (`src/app/keymap.rs`), and no popup letter: the popup uses
  `m u f t a e c x`.
- The popup is `Mode::Manage(Menu)` built from `PrsStage::offers` (`src/app/stages/prs.rs`), one
  `Offer { key, label, choice: Result<Choice, String> }` per action; an `Err` offer draws dim with
  `· <reason>` and its letter does nothing. A letter re-derives the menu and refuses with "the
  pull request changed; press S again" when the subject or the offer moved.
- Status changes run as `Choice::Status(StatusTarget)` → `Action::ChangeStatus` →
  `Request::ChangeStatus` → the status worker → `CliGh::change_status` (argv from `status_args`,
  `WRITE_DEADLINE`) → `Input::StatusChanged` → `App::receive_status`. Close and the changes that
  land a merge go through `Mode::Confirm(Pending::Status(target))`, whose `y` calls `reconfirm`
  with the letter `confirm_status` gives and compares the whole `StatusTarget`, `head_oid`
  included.
- On a successful status change `run()` refreshes My PRs, the Merge queue unless the change is
  `Ready`, and calls `follow_up_prs()`.
- `Demo` fails every write with `demo: nothing was changed`.
- The search query (`src/sources/queries/prs.graphql`) can read `viewerDidAuthor` and
  `viewerLatestReview { state commit { oid } }` on a `PullRequest`. Read on this repository's own
  pull requests with `gh api graphql`: every one opened by the owner answers
  `viewerDidAuthor: true`, `viewerLatestReview: null`, `reviewDecision: null` (no required
  reviews); #69, opened by someone else and approved by the owner, answers
  `viewerDidAuthor: false`, `viewerLatestReview.state: "APPROVED"`.
- `gh pr review --approve` (gh 2.104.0 source, `pkg/cmd/pr/review/review.go` and
  `api/queries_pr_review.go`) finds the pull request, then sends
  `addPullRequestReview(input: {pullRequestId, event: APPROVE, body: ""})` with no commit pin; an
  error comes back as `failed to create review: <GitHub's message>`. `--approve` needs no body;
  the success line is printed only on a terminal.
- Not verified, since it takes a write: GitHub's answer for the viewer's own pull request (known
  wording "Can not approve your own pull request"), a draft, a closed or merged one, and one the
  viewer already approved (accepted as another review). Each lands as the usual failure mark.
- The card already shows `reviewDecision` (`approved`, `changes requested`, `review required`,
  nothing when GitHub reports none), so an approval that satisfies the required reviews shows on
  the card after the next refresh with no card change.

## States, and what the popup and the card show

| State of the row | `v` in the popup | After `v` | Card |
| --- | --- | --- | --- |
| Someone else's open pull request | `v approve` | confirm `approve #N <title>?`, `y` sends | review state as today |
| The viewer's own (`viewerDidAuthor`) | dim, `· you opened #N` | nothing | unchanged |
| Approved by the viewer at the current head | dim, `· you already approved #N` | nothing | `approved` when GitHub's decision is |
| Approved by the viewer at an older head | `v approve` | confirm, `y` sends | review state as today |
| Changes requested by the viewer | `v approve` | confirm, `y` sends (an approval replaces it) | `changes requested` until the refresh |
| Draft | `v approve` | confirm, `y` sends; GitHub decides | failure mark if GitHub refuses |
| Closed or merged (only with a custom query) | `v approve` | confirm, `y` sends; GitHub decides | failure mark if GitHub refuses |
| In the merge queue | `v approve` | confirm, `y` sends | review state as today |
| A write in flight on the row | `S` refused with the progress line | n/a | `approving…` |
| The approve failed | `v approve` again | confirm | `! approve failed: <reason>` until the next write or the row changes |
| The approve timed out | `v approve` | confirm | `! may be approved` |
| Approved | n/a | footer `#N approved`, My PRs refreshed | the new review state after the refresh |
| Head moved, or approved elsewhere, between `v` and `y` | n/a | `y` refuses: the pull request changed; press S again | unchanged |

Draft, closed and merged are offered rather than dimmed: no other action checks them up front,
and GitHub's own answer is the one the user needs to read. The owner may overturn this.

## Design

- `StatusChange::Approve`, with no fields: approve is a status change, so it reuses the
  request, the status worker, the write in flight, the failure mark and `reconfirm`. No new
  worker kind.
- `PullRequest` gains `viewer_did_author: bool` and `viewer_approved_oid: Option<String>` (the
  commit of the viewer's latest review when its state is `APPROVED`). A missing field reads as
  `false` / `None`, so the offer shows.
- The offer is `v approve`, after `x` in the popup's order of status changes. Own pull request:
  `Err("you opened #N")`. Approved at the current head: `Err("you already approved #N")`.
- `v` asks to confirm: `confirm_status` maps `Approve` to `v`, the dialog asks
  `approve #N <title>?`.
- Wording: progress `approving #N…`, card `approving…`, failure `approve failed`, done
  `#N approved`, timeout `may be approved`.
- A successful approve refreshes My PRs and follows up, like `Ready`; the queue is not refreshed.
- `status_args` for `Approve`: `pr review <N> -R <repo> --approve`.
- The help line and the README row for `S` add `approve`.

## Tasks

Each is one red-green-refactor cycle, outside-in from `run()` where the key drives it, shipped
with `scripts/ship.sh`.

1. **Parse the viewer's part.** The query reads `viewerDidAuthor` and
   `viewerLatestReview { state commit { oid } }`; `parse_pull_request` fills the two fields
   (`tests/parse_prs.rs`). Ships with this plan, its `PLAN.md` and `plans/metrics.md` rows and
   the fragment.
2. **The popup offers approve.** Someone else's row shows `v approve`; the viewer's own row
   shows it dim with `you opened #N`; one approved at the current head shows it dim with
   `you already approved #N`; one approved at an older head offers it.
3. **`v` confirms and `y` sends.** The dialog question, the `Request::ChangeStatus` with
   `Approve`, the progress line and card word, the done line, the refresh of My PRs only, the
   failure mark, and `y` refused after the head moved.
4. **`gh pr review --approve`.** `status_args` and the timeout wording
   (`tests/github/status.rs`); the demo refuses it.
5. **The demo, the help, the screenshots.** `viewerDidAuthor` in the demo fixtures, the help
   line and README row, a `shoot approve` line in `scripts/screenshots.sh`, the images.
6. **Docs.** `src/app/AGENTS.md` (the popup letters, the approve bullet), `src/sources/AGENTS.md`
   where it lists the status argv.

## Outcome

- Shipped in #144 as `StatusChange::Approve`: the `S` popup on a My PRs row offers `v approve`,
  asks `approve #N <title>?`, and `y` runs `gh pr review <N> -R <repo> --approve` through the
  status worker. A success refreshes My PRs only and says `#N approved`; a failure marks the
  card `! approve failed: <reason>`, a timeout `may be approved`.
- The offer is dim on the viewer's own pull request (`viewerDidAuthor`) and on one the viewer
  approved at the current head (`viewerLatestReview`); an approval of an older head offers it
  again.
- Decisions taken without the owner, each open to overturn: drafts, closed and queued pull
  requests are offered (GitHub's refusal becomes the failure mark); `v` sits last in the popup,
  after `x`; the card is unchanged and relies on `reviewDecision`, which stays empty on a
  repository without required reviews; an approve does not refresh the queue; the help line and
  the README row list `approve`.
- The demo's mine tab is the viewer's own and its review tab is someone else's, so
  `docs/approve.png` shows the popup with `v approve` on a review-tab row.
- Not verified, since each needs a write: GitHub's wording for approving one's own pull
  request, a draft, a closed one and one already approved.

## Hurdles

- `test-seam` The `manage_layout` snapshot failed the first ship of task 2: the new popup line was expected, and the snapshot was not run before shipping → none
- `ci` `calls::a_call_made_while_another_hangs_is_not_held_up_by_it` failed once at load 25 to 41 while sibling phases built in parallel; untouched by this change, it passed on the retry → none
- `tooling` `scripts/screenshots.sh` rewrote `docs/confirm.png` and `docs/no-queue.png` for an age that read `2s ago` instead of `1s ago`; both were reverted by hand → none
