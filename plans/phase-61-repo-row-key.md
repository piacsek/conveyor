# Phase 61: two pull requests with the same number in different repositories stay apart

## Context
Issue #121. My PRs keys a row by its pull request number alone (`impl Row for PullRequest`,
`key() = self.number`, `src/app/column.rs`). The default query spans every repository, so
`acme/webapp#12` and `acme/infra#12` can sit in the same column. Then a refresh moves the
selection to the first `#12`, a write in flight on one refuses `S` on the other, a failure mark
shows on both cards, a merge of one makes the other wait for My PRs to catch up, and the popup
and the confirm dialog read the same for both. Phase 60 is the sweep, so this is 61.

## Verified facts (2026-11-17, at `6512665`)
- `Column::receive` and `clear_filter` keep the selection through `reselect(key)`, the first
  visible row with that key; `merge_keeping_order` matches old and new rows by key too.
- `Write.subject` and `Failure.subject` are the row key (`write_on`, `failure_on`,
  `Listing::holds`), but `start_write` and `finish_write` are called with `target.number`, so
  today the number and the key are the same value. `Write::progress` prints `#{subject}`.
- `Input::Merged`, `BranchUpdated` and `StatusChanged` carry the number only;
  `CheckedOut` already carries its `CheckoutTarget`.
- `App::merge_landing` is `Vec<(u64, usize, bool)>` keyed by number; `Context::landing(number)`
  refuses `t` (convert to draft) and `u`.
- `impl Row for Deployment` already hashes its `env` into the `u64` key with
  `std::hash::DefaultHasher::new()`, whose keys are fixed, so the same input gives the same key
  for the whole session.
- The popup title is `Manage #N` (`PrsStage::menu`) and `reconfirm` compares the fresh title
  with `Manage #{number}`; the confirm dialog's `question` (`src/ui/dialog.rs`) prints
  `#N <title>`. The cards already print the short repo (`text::short_repo`).
- The demo fixture already holds two repositories (`acme/webapp`, `acme/auth`).

## Design
- **The key hashes repo and number together**, the smaller of the two designs the issue offers:
  `Row::key` stays `u64`, so `Listing`, `holds`, `selected_key`, `write_on`, `failure_on`, the
  ui lookups and the other three stages do not change, and `Deployment` already set the
  precedent. A typed key (`(String, u64)`) would make `Row` generic over its key and touch
  every stage. A collision of two 64-bit SipHash values among a few dozen rows is not a case.
  One function, `column::pull_key(repo, number)`, gives the key to the row and to the targets.
- **The display number leaves the key.** The pull request kinds of `WriteKind` carry their
  number (`Merge(n)`, `Update(n)`, `Status(n, change)`, `Checkout(n)`), so `progress` still
  prints `#N`; `subject` is the row key.
- **Results carry their target**, as `CheckedOut` already does: `Merged(MergeTarget, …)`,
  `BranchUpdated(UpdateTarget, …)`, `StatusChanged(StatusTarget, …)`, so a result finds its own
  write by key and kind without relying on the order a worker answers in.
- **`merge_landing` holds the row key.**
- **The popup and the dialog name the repository when the active My PRs tab holds rows from
  more than one repository** (all its rows, not only the ones the filter shows, so typing a
  filter never renames the popup): `Manage infra#12`, `merge infra#12 Drop the users table?`.
  One repository: unchanged, `Manage #12`. `PrsStage::label(repo, number)` gives `infra#12` or
  `#12` to both, and `reconfirm` compares the fresh title with the one that label makes.

## States and what the screen shows
`webapp#12` and `infra#12` are both in the active My PRs tab unless the row says otherwise.

| Situation | Before | After |
| --- | --- | --- |
| Cursor on `infra#12`, refresh delivers the same rows | cursor jumps to `webapp#12` | stays on `infra#12` |
| Same, filter cleared with `Esc` | jumps to `webapp#12` | stays on `infra#12` |
| A write in flight on `webapp#12`, `S` on `infra#12` | refused with `webapp#12`'s progress | popup opens |
| A write on `webapp#12` failed | mark on both cards | mark on `webapp#12` only |
| Merge of `webapp#12` sent, `u` or `t` on `infra#12` | `waiting for My PRs to catch up with #12` | offered |
| `S` on `infra#12` | `Manage #12` | `Manage infra#12` |
| `m` then the confirm dialog on `infra#12` | `merge #12 Drop the users table?` | `merge infra#12 Drop the users table?` |
| Only `acme/webapp` rows in the tab | `Manage #12` | `Manage #12` (unchanged) |
| Footer progress and result lines | `merging #12…` | `merging #12…` (unchanged) |
| `y` on `webapp#12` | `copied #12` | `copied #12` (unchanged) |
| `acme/webapp#12` and the fork `me/webapp#12` in one tab, `S` on the fork | `Manage #12` | `Manage me/webapp#12` |
| Popup open on `webapp#12`, a refresh adds `acme/infra#7`, then a letter | acts | acts (the fresh title reads `Manage webapp#12`; the row key is compared, not the title) |

## Tasks
1. Selection: `receive` of `acme/webapp#12` and `acme/infra#12`, `j`, the same rows again; the
   selection stays on `acme/infra#12`. Green keys the row by `pull_key`, and with it the writes
   and failure marks (results carry their target). Pinned beside it: a write in flight on
   `webapp#12` does not refuse `S` on `infra#12`; a failed write marks only its own card.
2. `merge_landing`: a merge sent for `webapp#12` does not refuse `u` on `infra#12`.
3. The popup title and the confirm dialog name the short repository when the tab holds more
   than one; one repository keeps `#N`.
4. Docs and screenshot: drop the Known limits entry in root `AGENTS.md`, correct
   `src/app/AGENTS.md` and `src/ui/AGENTS.md`, the demo fixture gains a pull request with the
   same number as another in a second repository, a `shoot` of the confirm dialog, the
   fragment `changes/phase-61-repo-row-key.md`.

## Outcome

Shipped in #133 over two review rounds. A My PRs row is keyed by its repository and number
together (`column::pull_key`), so selection, the busy spinner, failure marks and the merge wait
belong to one pull request; write results carry their whole target. While the active tab lists
more than one repository, the `S` popup and the confirm dialog name the repository
(`Manage infra#12`), with the owner when two share a short name (`Manage me/webapp#12`). A
letter in the popup now checks that the same row is selected (`Menu::subject`) instead of
comparing titles, since the title can gain a repository name on a refresh. The "A pull request
number is the row key" known limit is gone whole: Merge queue rows are still keyed by number,
but nothing writes to them (`S` refuses there) and each tab is one repository. The demo gains
`acme/infra#4821` and `docs/confirm.png`. Not covered: a long repository label clips `#N` off
the popup title at narrow widths. Backlog: a merge result landing after a My PRs tab switch is
recorded against the tab now shown.

## Hurdles

- `test-seam` The `y` notice printed the row hash (`copied #13499…`) after the key change; the test's `contains("copied #1")` matched a hash starting with 1, so review round 1 found it, and the test now asserts the whole notice → code
- `test-seam` The e2e demo test and the `confirm_layout` and `manage_layout` snapshots pinned the old popup wording (`Manage #4821`), so the first ship of the label failed twice before they were updated → code
- `tooling` The first `ship.sh` of the phase was piped through `tail`, which the rule forbids; later ships wrote to a log file → none
- `review-loop` Round 1 asked for a Merge queue known limit (number-only keys sharing the spinner and mark) that cannot happen, since `S` refuses in Merge queue; round 2 removed it again → none
