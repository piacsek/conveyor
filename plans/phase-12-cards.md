# Phase 12 — two things the cards were getting wrong

## 1. A cancelled run blamed the jobs its cancellation killed

GitHub marks the jobs a cancellation stops as **failed**, so a cancelled build card carried a
red `✗ Nx / Status · Check Nx workflow status` line for work that never failed on its own — the
loudest thing on the card, and wrong. `build_jobs` and the queue's `failed_job` now drop the
job line when the run is `Cancelled` and fall back to the sha. The details pane still lists
every job: there the conclusions are stated as facts, not as a headline.

## 2. The title shared the first line with the number and the age

`✓ #7108 [AIE-65] Remove commit and push… 3h` — the title was the part that got clipped, and it
is the one thing on the card you cannot reconstruct from the others. A build card is four lines
now: the number with the age, then the title on a line of its own at full width, then the meta
line, then the jobs line.

Row `n` starts at screen line `1 + 4n`; card tests index accordingly, and a test that wants a
third card needs a terminal taller than the 12-row default.

## Found in review

Three mutations survived the first pass and are now killed: the title line's `truncate` could
be deleted entirely (nothing asserted the `…`), the `INDENT` could be dropped from its width,
and the details pane could stop listing the jobs the card now hides — the one invariant the
whole cancellation change leans on. The Builds column also had **no snapshot at all**; it has
one now, carrying a running, a failed and a cancelled card, which pins the line order too.

The e2e assertion had been weakened rather than moved: two whole-screen `contains` that were
both true before the change. It reads lines now.

The bold that marks the selected card lives on `card()`'s first line, which is now just the
number — so the emphasis had quietly left the title. `build_row` styles the title line itself.
