# Phase 11 — the run speaks for the queue entry, and `⊘` means cancelled

## Why

A merge queue cancels runs routinely when it re-batches. The queue card read its glyph and
word from `CheckState`, which has no word for that: v0.10.0 mapped a cancelled run to `?`, and
the version before it to `✗ failure`. Meanwhile the details pane, one line below, said
`cancelled` — the card and the pane contradicting each other on the same screen.

## How

Once an entry has a run, the run speaks for the entry: the card's glyph and status word come
from `Build::status`, exactly as in Main builds, and the pane drops its `checks:` line because
the run line already carries that fact. The GraphQL rollup is left untouched on the entry and
is still what a run-less entry reads from, so `checks_of` (the lossy `BuildStatus → CheckState`
map) is gone rather than patched.

`BuildStatus::Cancelled` also gets its own glyph, `⊘`, in every column. It used to share the
grey `-` with a *skipped* check, which is a different fact: a cancelled run is work that was
stopped, a skipped check is work that was never required.

## Tasks

1. The queue card's glyph and word come from the run when there is one.
2. The pane's `checks:` line is for run-less entries only; `checks_of` deleted.
3. `⊘` for cancelled, everywhere; `-` stays for skipped.
